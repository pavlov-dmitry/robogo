use chrono::Local;
use serialport::{self, SerialPort, SerialPortInfo};
use std::io::{BufRead, BufReader, BufWriter, Write};
use thiserror::Error;

type SerialPortPtr = Box<dyn SerialPort>;

pub struct ArduinoPort {
    reader: BufReader<SerialPortPtr>,
    writer: BufWriter<SerialPortPtr>,
    log: std::fs::File,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Ошибка обмена по сериал порту")]
    Serial(#[from] serialport::Error),
    #[error("Ошибка обмена ввода вывода")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl ArduinoPort {
    pub fn new(portname: &str, timeout_secs: u64) -> Result<ArduinoPort> {
        let port_to_read = serialport::new(portname.to_string(), 115200)
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .open()?;
        let port_to_write = port_to_read.try_clone()?;
        let log = std::fs::File::create("./log/arduino_post.log")?;
        let arduino_port = ArduinoPort {
            reader: BufReader::new(port_to_read),
            writer: BufWriter::new(port_to_write),
            log: log,
        };
        Ok(arduino_port)
    }

    pub fn available_ports() -> Result<Vec<SerialPortInfo>> {
        let ports = serialport::available_ports()?;
        Ok(ports)
    }

    pub fn send_cmd(&mut self, cmd: &str) -> Result<String> {
        writeln!(self.log, "[{}] write: {cmd}", timestamp())?;
        writeln!(self.writer, "{cmd}")?;
        self.writer.flush()?;

        let mut answer = String::new();
        loop {
            let mut line = String::new();
            self.reader.read_line(&mut line)?;
            writeln!(self.log, "[{}] read: {line}", timestamp())?;
            if line.trim() == "done" {
                break;
            } else {
                answer.push_str(&line);
            }
        }
        Ok(answer)
    }
}

fn timestamp() -> String {
    let now = Local::now();
    format!("{}", now.format("%F_%T%.3f"))
}
