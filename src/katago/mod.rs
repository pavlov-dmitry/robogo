use super::board::{self, Board, Color, ParsePositionError};
use chrono::Local;
use std::str::FromStr;
use std::{
    fs::File,
    io::{self, BufRead, BufReader, Write},
    num::ParseIntError,
    process::{Command, Stdio},
};

mod parse;

pub struct Settings {
    dir: String,
    config: String,
    model: String,
    human_model: String,
    log_filename: String,
    dump_to_filename: bool,
}

impl Settings {
    pub fn default() -> Settings {
        Settings {
            dir: String::from("./katago"),
            config: String::from("gtp_human5k_example.cfg"),
            model: String::from("kata1-b28c512nbt-s8536703232-d4684449769.bin.gz"),
            human_model: String::from("b18c384nbt-humanv0.bin"),
            log_filename: String::from("./vision_dump/katago.log"),
            dump_to_filename: true,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    InvalidTextProtocol,
    ParseIntError,
    ParsePositionError,
    UnknownError(String),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<ParseIntError> for Error {
    fn from(_: ParseIntError) -> Self {
        Error::ParseIntError
    }
}

impl From<ParsePositionError> for Error {
    fn from(_: ParsePositionError) -> Self {
        Error::ParsePositionError
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct State {
    pub board: Board,
    pub move_num: u32,
    pub next_move: Color,
    pub black_captured: u32,
    pub white_captured: u32,
}

pub struct Katago {
    process: std::process::Child,
    log: Option<File>,
    board_size: usize,
}

fn timestamp() -> String {
    let now = Local::now();
    format!("{}", now.format("%F %T"))
}

impl Katago {
    pub fn new(settings: Settings) -> Result<Katago> {
        let process = Command::new("./katago")
            .current_dir(settings.dir)
            .arg("gtp")
            .arg("-config")
            .arg(settings.config)
            .arg("-model")
            .arg(settings.model)
            .arg("-human-model")
            .arg(settings.human_model)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let log_file = if settings.dump_to_filename && !settings.log_filename.is_empty() {
            Some(File::create(settings.log_filename)?)
        } else {
            None
        };
        Ok(Katago {
            process: process,
            log: log_file,
            board_size: 19,
        })
    }

    pub fn wait_gtp_ready(&mut self) -> Result<()> {
        let stdout = self.process.stderr.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::BrokenPipe, "Katago stdout not aviable")
        })?;
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let line = line?;
            if let Some(log) = &mut self.log {
                writeln!(log, "[{}] READ: {}", timestamp(), line)?;
            }
            //ждём строку с GTP ready
            if line.starts_with("GTP ready") {
                break;
            }
        }
        Ok(())
    }

    fn send(&mut self, cmd: &str) -> Result<String> {
        let stdin =
            self.process.stdin.as_mut().ok_or_else(|| {
                io::Error::new(io::ErrorKind::BrokenPipe, "Katago stdin not aviable")
            })?;

        writeln!(stdin, "{}", cmd)?;
        stdin.flush()?;

        if let Some(log) = &mut self.log {
            writeln!(log, "[{}] CMD: {}", timestamp(), cmd)?;
        }

        let stdout = self.process.stdout.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::BrokenPipe, "Katago stdout not aviable")
        })?;
        let reader = BufReader::new(stdout);
        let mut response = String::new();
        for line in reader.lines() {
            let line = line?;
            if let Some(log) = &mut self.log {
                writeln!(log, "[{}] READ: {}", timestamp(), line)?;
            }
            //команды кончаются пустой строкой
            if line.is_empty() {
                break;
            }
            response.push_str(&line);
            response.push('\n');
        }
        Ok(response)
    }

    pub fn get_current_state(&mut self) -> Result<State> {
        let answer = self.send("showboard")?;
        if answer.starts_with("?") {
            return Err(Error::UnknownError(answer));
        }

        let lines: Vec<&str> = answer.as_str().lines().collect();
        if lines.len() < self.board_size + 6 {
            return Err(Error::InvalidTextProtocol);
        }

        let mut board = Board::new_with_size(self.board_size);
        for line_number in 0..self.board_size {
            let y = self.board_size - line_number - 1;
            parse::board_line(lines[2 + line_number], self.board_size, |x, color| {
                board.set(board::Position::new(x, y), board::Cell::from(color));
            })?
        }

        Ok(State {
            move_num: parse::move_num(lines[0])?,
            board: board,
            next_move: parse::next_move(lines[self.board_size + 2])?,
            black_captured: parse::black_captured(lines[self.board_size + 4])?,
            white_captured: parse::white_captured(lines[self.board_size + 5])?,
        })
    }

    pub fn play(&mut self, color: Color, pos: board::Position) -> Result<()> {
        let cmd = format!("play {color} {pos}");
        let answer = self.send(&cmd)?;
        if answer.starts_with("?") {
            return Err(Error::UnknownError(answer));
        }
        Ok(())
    }

    pub fn genmove_for(&mut self, color: Color) -> Result<board::Position> {
        let cmd = format!("genmove {color}");
        let answer = self.send(&cmd)?;
        if answer.starts_with("?") {
            return Err(Error::UnknownError(answer));
        }
        let pos_str = answer.get(2..).ok_or_else(|| Error::InvalidTextProtocol)?;
        let position = board::Position::from_str(pos_str)?;
        Ok(position)
    }
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.board)?;
        writeln!(f, "move number: {}", self.move_num)?;
        writeln!(f, "next move: {}", self.next_move)?;
        writeln!(f, "black stones captured: {}", self.black_captured)?;
        writeln!(f, "white stones captured: {}", self.white_captured)?;
        Ok(())
    }
}
