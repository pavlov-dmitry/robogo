use std::{
    io::{self, BufRead, BufReader, Write},
    process::{Command, Stdio},
};

pub struct Settings {
    dir: String,
    config: String,
    model: String,
    human_model: String,
}

impl Settings {
    pub fn default() -> Settings {
        Settings {
            dir: String::from("./katago"),
            config: String::from("gtp_human5k_example.cfg"),
            model: String::from("kata1-b28c512nbt-s8536703232-d4684449769.bin.gz"),
            human_model: String::from("b18c384nbt-humanv0.bin"),
        }
    }
}

pub struct Katago {
    process: std::process::Child,
}

impl Katago {
    pub fn new(settings: Settings) -> io::Result<Katago> {
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
        Ok(Katago { process })
    }

    pub fn wait_gtp_ready(&mut self) -> io::Result<()> {
        let stdout = self.process.stderr.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::BrokenPipe, "Katago stdout not aviable")
        })?;
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let line = line?;
            println!("line readed: ({}) {}", line.len(), line);
            //ждём строку с GTP ready
            if line.starts_with("GTP ready") {
                break;
            }
        }
        Ok(())
    }

    pub fn send(&mut self, cmd: &str) -> io::Result<String> {
        let stdin =
            self.process.stdin.as_mut().ok_or_else(|| {
                io::Error::new(io::ErrorKind::BrokenPipe, "Katago stdin not aviable")
            })?;

        writeln!(stdin, "{}", cmd)?;
        stdin.flush()?;

        println!("cmd: {}", cmd);

        let stdout = self.process.stdout.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::BrokenPipe, "Katago stdout not aviable")
        })?;
        let reader = BufReader::new(stdout);
        let mut response = String::new();
        for line in reader.lines() {
            let line = line?;
            println!("line readed: ({}) {}", line.len(), line);
            //команды кончаются пустой строкой
            if line.is_empty() {
                break;
            }
            response.push_str(&line);
            response.push('\n');
        }
        Ok(response)
    }
}
