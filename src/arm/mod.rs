pub mod arduino_port;

#[derive(Debug)]
pub enum Error {
    ArduinoPort(arduino_port::Error),
}

impl From<arduino_port::Error> for Error {
    fn from(value: arduino_port::Error) -> Self {
        Error::ArduinoPort(value)
    }
}
