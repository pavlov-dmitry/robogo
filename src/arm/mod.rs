pub mod arduino_port;
pub mod arm_positioner;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Ошибка обмена с Arduino")]
    ArduinoPort(#[from] arduino_port::Error),
}
