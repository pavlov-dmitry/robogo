mod board;
mod game;
mod katago;
mod listen;
mod regime;
mod speech;
mod vision;

use clap::{Parser, Subcommand};

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

    /// Режим распознования доски по фотографии (по умолчанию сохраняет файлы промежуточных шагов)
    ParseBoard {
        photo_filename: String,
        dump_files: Option<bool>,
    },

    /// Режим калибровки камеры по фото (нужен путь на папку с файлми 1.jpg, 2.jpg и т.д. и их количество)
    Calibrate { photo_dir: String, count: u32 },

    ///Тест калибровки камеры. Отображает фото с применённой калибровкой
    TestCalibration { photo_filename: String },

    ///Тесты зрения. на вход папка с подпаками тестами где есть photo.jpg и board.txt. Если board.txt нет значит ничего не должно распозноваться
    VisionTests { tests_dir: String },

    /// Режим тестирования распознования голоса
    Listen,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Some(cmd) = cli.command {
        match cmd {
            Commands::Camera => regime::camera::exec(),
            Commands::Vision => regime::check_vision::exec(),

            Commands::ParseBoard {
                photo_filename,
                dump_files,
            } => regime::parse_board::exec(&photo_filename, dump_files),

            Commands::Calibrate { photo_dir, count } => {
                vision::calibrate_by(&photo_dir, count)?;
                Ok(())
            }

            Commands::TestCalibration { photo_filename } => {
                vision::test_calibration(&photo_filename)?;
                Ok(())
            }

            Commands::VisionTests { tests_dir } => regime::vision_tests::exec(&tests_dir),

            Commands::Listen => regime::check_listen::exec(),
        }
    } else {
        // по дефолту сразу переходиим к игре
        let result = regime::human_vs_ai::exec();
        if let Err(e) = result {
            println!("Error: {:?}", e);
        }
        Ok(())
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum Error {
    Katago(katago::Error),
    Speech(speech::Error),
    Vision(vision::Error),
    Listen(listen::Error),
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
impl From<listen::Error> for Error {
    fn from(value: listen::Error) -> Self {
        Error::Listen(value)
    }
}
fn game() -> Result<()> {
    let mut game = game::Game::new();
    let mut katago = katago::Katago::new(katago::Settings::default())?;
    let mut vision = vision::Vision::new(vision::Settings::default())?;
    let mut speech = speech::Speech::new(speech::Settings::default());
    let mut listen = listen::Listen::new(VoiceCommandsSettings::default());

    vision.spawn();
    katago.spawn();
    listen.spawn();

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
        if let Some(msg) = listen.step() {
            nothing_todo = false;
            match msg {
                listen::Msg::Text(txt) => println!("Human say: {txt}"),
                listen::Msg::Cmd(cmd) => game.on_voice_cmd(cmd),
                listen::Msg::Err(e) => return Err(Error::from(e)),
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

    loop {
        println!("Game finished! Press Ctrl+C");
        speech.step();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    //Ok(())
}
