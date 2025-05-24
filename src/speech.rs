use std::collections::LinkedList;
use std::io::{self, Result, Write};
use std::process::{self, Command};

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

pub type Error = io::Error;
pub enum Msg {
    Error(Error),
}

pub struct Speech {
    voice: String,
    speech_queue: LinkedList<String>,
    current: Option<process::Child>,
}

impl Speech {
    pub fn new(settings: Settings) -> Speech {
        Speech {
            voice: settings.voice,
            speech_queue: LinkedList::new(),
            current: None,
        }
    }

    pub fn say(&mut self, text: &str) {
        self.speech_queue.push_back(String::from(text));
    }

    pub fn step(&mut self) -> Option<Msg> {
        if let Some(current) = &mut self.current {
            match current.try_wait() {
                Ok(maybe_status) => {
                    if let Some(_) = maybe_status {
                        self.current = None;
                    }
                }
                Err(e) => return Some(Msg::Error(e)),
            }
        }
        if self.current.is_none() {
            if let Some(text) = self.speech_queue.pop_front() {
                // реализация говно, но работает. Полностью из нутри раста с наскоку не вышло. Пусть пока так.
                let voice = self.voice.as_str();
                let cmd = format!("echo \"{text}\" | RHVoice-test -p \"{voice}\"");
                match Command::new("sh").arg("-c").arg(cmd).spawn() {
                    Ok(child) => {
                        self.current = Some(child);
                    }
                    Err(e) => return Some(Msg::Error(e)),
                }
            }
        }
        None
    }
}
