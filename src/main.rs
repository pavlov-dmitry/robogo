mod board;
mod game;
mod katago;
mod speech;
mod vision;

use clap::{Parser, Subcommand};
use std::str::FromStr;

#[derive(Parser)]
#[command(version, about, long_about=None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Режим фотоаппарата, для настройки камеры.
    Camera,

    /// Режим тестирования компьютерного зрения. Распознаёт позицию на доске.
    Vision,

    /// Режим распознования доски по фотографии
    ParseBoard { photo_filename: String },

    /// Режим калибровки камеры по фото (нужен путь на папку с файлми 1.jpg, 2.jpg и т.д. и их количество)
    Calibrate { photo_dir: String, count: u32 },

    ///Тест калибровки камеры. Отображает фото с применённой калибровкой
    TestCalibration { photo_filename: String },

    ///Тесты зрения. на вход папка с подпаками тестами где есть photo.jpg и board.txt. Если board.txt нет значит ничего не должно распозноваться
    VisionTests { tests_dir: String },
}

fn game() -> Result<()> {
    let mut game = game::Game::new();
    let mut katago = katago::Katago::new(katago::Settings::default())?;
    let mut vision = vision::Vision::new(vision::Settings::default())?;
    let mut speech = speech::Speech::new(speech::Settings::default());

    vision.spawn();
    katago.spawn();

    loop {
        let mut nothing_todo = true;
        //обработка подсисетмы зрения
        if let Some(msg) = vision.step() {
            nothing_todo = false;
            match msg {
                vision::Msg::Board(brd) => game.on_human_update_board(brd),
                vision::Msg::Error(e) => return Err(Error::from(e)),
            }
        }
        // обработка подсистемы AI
        if let Some(msg) = katago.step() {
            nothing_todo = false;
            match msg {
                katago::Msg::State(state) => game.on_ai_state_update(state),
                katago::Msg::Move(mv) => game.on_ai_move(mv),
                katago::Msg::Error(e) => return Err(Error::from(e)),
            }
        }
        // обработка подсистемы ведения игры
        if let Some(msg) = game.step() {
            nothing_todo = false;
            match msg {
                game::Msg::WrongStones(ws) => speech.say_for(&ws),
                game::Msg::HumanPlay(color, mv) => {
                    speech.say("Ага.");
                    katago.play(color, mv);
                }
                game::Msg::NeedAiMove(cl) => katago.genmove_for(cl),
                game::Msg::Speech(s) => speech.say(&s),
                game::Msg::Error(e) => return Err(Error::from(e)),
                game::Msg::GameFinished => {
                    speech.say("Спасибо за игру.");
                    break;
                }
            }
        }

        // обработка сообщений от подсистемы синтеза речи
        if let Some(msg) = speech.step() {
            match msg {
                speech::Msg::Error(e) => return Err(Error::from(e)),
            }
        }

        if nothing_todo {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    Ok(())
}

fn vision() -> Result<()> {
    let mut vision = vision::Vision::new(vision::Settings::default())?;
    vision.spawn();

    loop {
        if let Some(msg) = vision.step() {
            match msg {
                vision::Msg::Board(brd) => println!("{brd}"),
                vision::Msg::Error(e) => return Err(Error::from(e)),
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn parse_board(photo: &str) -> Result<()> {
    let settings = vision::Settings::default();
    let board = vision::parse_board_from(photo, &settings)?;
    match board {
        Some(brd) => println!("{brd}"),
        None => println!("Доска не найдена."),
    }
    Ok(())
}

fn vision_tests(tests_dir: &str) -> Result<()> {
    let mut success_count = 0;
    let mut failed_count = 0;

    for entry in std::fs::read_dir(tests_dir)? {
        let entry = entry?;
        let entry_type = entry.file_type()?;

        // проходимся только директориям
        if entry_type.is_dir() {
            // имя теста это имя директории
            let name = entry.file_name();
            println!("Test {}", name.display());

            let settings = vision::Settings::default();
            let photo_filename = format!("{}/photo.jpg", entry.path().as_os_str().display());
            let board_from_vision = vision::parse_board_from(&photo_filename, &settings)?;
            let board_filename = format!("{}/board.txt", entry.path().as_os_str().display());
            let is_board_file_exists = std::fs::exists(&board_filename)?;

            let test_success = match board_from_vision {
                Some(vision_board) => {
                    println!("VISION:\n{}", vision_board);
                    if is_board_file_exists {
                        let board_txt = std::fs::read_to_string(board_filename)?;
                        let board = board::Board::from_str(&board_txt)?;
                        println!("SOURCE BOARD:\n{}", board);
                        vision_board == board
                    } else {
                        println!("SOURCE BOARD DO NOT EXISTS");
                        false
                    }
                }
                None => {
                    println!("VISION: None");
                    println!("Source board exists: {is_board_file_exists}");
                    !is_board_file_exists
                }
            };

            // подсчитывам количество пройденных и не пройденных тестов
            if test_success {
                success_count += 1;
                println!("Test {} success.", name.display());
            } else {
                failed_count += 1;
                println!("Test {} FAILED!", name.display());
            }
            println!("---------------------------------------------\n");
        }
    }
    println!("All tests finished. {success_count} success, {failed_count} failed.");
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Some(cmd) = cli.command {
        match cmd {
            Commands::Camera => {
                vision::camera_mode()?;
                Ok(())
            }

            Commands::Vision => vision(),

            Commands::ParseBoard { photo_filename } => parse_board(&photo_filename),

            Commands::Calibrate { photo_dir, count } => {
                vision::calibrate_by(&photo_dir, count)?;
                Ok(())
            }

            Commands::TestCalibration { photo_filename } => {
                vision::test_calibration(&photo_filename)?;
                Ok(())
            }

            Commands::VisionTests { tests_dir } => vision_tests(&tests_dir),
        }
    } else {
        // по дефолту сразу переходиим к игре
        let result = game();
        if let Err(e) = result {
            println!("Error: {:?}", e);
        }
        Ok(())
    }
}

#[allow(dead_code)]
#[derive(Debug)]
enum Error {
    Katago(katago::Error),
    Speech(speech::Error),
    Vision(vision::Error),
    Game(game::Error),
    BoardParseError,
    Io(std::io::Error),
}

type Result<T> = std::result::Result<T, Error>;

impl From<katago::Error> for Error {
    fn from(value: katago::Error) -> Self {
        Error::Katago(value)
    }
}
impl From<speech::Error> for Error {
    fn from(value: speech::Error) -> Self {
        Error::Speech(value)
    }
}
impl From<vision::Error> for Error {
    fn from(value: vision::Error) -> Self {
        Error::Vision(value)
    }
}
impl From<game::Error> for Error {
    fn from(value: game::Error) -> Self {
        Error::Game(value)
    }
}
impl From<board::BoardParseError> for Error {
    fn from(_: board::BoardParseError) -> Self {
        Error::BoardParseError
    }
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}
