use std::io::{self, Result, Write};
use std::process::{Command, Stdio};

pub struct Settings {
    voice: String,
}

impl Settings {
    pub fn default() -> Settings {
        Settings {
            voice: String::from("mikhail"),
        }
    }
}

pub struct Speech {
    voice: String,
}

impl Speech {
    pub fn new(settings: Settings) -> Speech {
        Speech {
            voice: settings.voice,
        }
    }

    pub fn say(&self, text: &str) -> Result<bool> {
        // реализация говно, но работает. Полностью из нутри раста с наскоку не вышло. Пусть пока так.
        let voice = self.voice.as_str();
        let cmd = format!("echo \"{text}\" | RHVoice-test -p \"{voice}\"");
        let status = Command::new("sh").arg("-c").arg(cmd).status()?;
        Ok(status.success())
    }
}
