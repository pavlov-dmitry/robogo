use serialport::{self, SerialPort, SerialPortInfo};

use std::io::{BufRead, BufReader, BufWriter, Write};

type SerialPortPtr = Box<dyn SerialPort>;

pub struct ArduinoPort {
    reader: BufReader<SerialPortPtr>,
    writer: BufWriter<SerialPortPtr>,
}

#[derive(Debug)]
pub enum Error {
    Serial(serialport::Error),
    Io(std::io::Error),
}

type Result<T> = std::result::Result<T, Error>;

impl ArduinoPort {
    pub fn new(portname: &str, timeout_secs: u64) -> Result<ArduinoPort> {
        let port_to_read = serialport::new(portname.to_string(), 9600)
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .open()?;
        let port_to_write = port_to_read.try_clone()?;
        let arduino_port = ArduinoPort {
            reader: BufReader::new(port_to_read),
            writer: BufWriter::new(port_to_write),
        };
        Ok(arduino_port)
    }

    pub fn available_ports() -> Result<Vec<SerialPortInfo>> {
        let ports = serialport::available_ports()?;
        Ok(ports)
    }

    pub fn send_cmd(&mut self, cmd: &str) -> Result<String> {
        write!(self.writer, "{cmd}\n")?;
        self.writer.flush()?;

        let mut answer = String::new();
        loop {
            let mut line = String::new();
            self.reader.read_line(&mut line)?;
            if line.trim() == "done" {
                break;
            } else {
                answer.push_str(&line);
            }
        }
        Ok(answer)
    }
}

impl From<serialport::Error> for Error {
    fn from(value: serialport::Error) -> Self {
        Error::Serial(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}
