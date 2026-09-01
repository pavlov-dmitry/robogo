mod arm;
mod board;
mod config;
mod human_vs_ai_game_control;
mod katago;
mod listen;
mod regime;
mod speech;
mod vision;

use clap::{Parser, Subcommand};
use thiserror::Error;

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

    /// Режим тестирования руки по сериал-порту, если запустить без имени порта, то выдаст список досутпных с их именами
    Arm { portname: Option<String> },
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

            Commands::Calibrate { photo_dir, count } => regime::calibrate::exec(photo_dir, count),

            Commands::TestCalibration { photo_filename } => {
                regime::test_calibration::exec(photo_filename)
            }

            Commands::VisionTests { tests_dir } => regime::vision_tests::exec(&tests_dir),

            Commands::Listen => regime::check_listen::exec(),

            Commands::Arm { portname } => {
                regime::check_arm::exec(portname)?;
                Ok(())
            }
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
#[derive(Error, Debug)]
pub enum Error {
    #[error("Ошибка работы с Katago.")]
    Katago(#[from] katago::Error),
    #[error("Ошибка работы с речью")]
    Speech(#[from] speech::Error),
    #[error("Ошибка работы со зрением")]
    Vision(#[from] vision::Error),
    #[error("Ошибка работы со слухоч")]
    Listen(#[from] listen::Error),
    #[error("Ошибка в логике Человек против ИИ")]
    HumanVsAi(#[from] human_vs_ai_game_control::Error),
    #[error("Ошибка парсинга доски.")]
    BoardParseError(#[from] board::BoardParseError),
    #[error("Ошибка ввода-вывода")]
    Io(#[from] std::io::Error),
    #[error("Ошибка обмена с Arduino")]
    ArduinoPort(#[from] arm::arduino_port::Error),
    #[error("Ошибка обмена с рукой")]
    Arm(#[from] arm::Error),
}

type Result<T> = std::result::Result<T, Error>;
