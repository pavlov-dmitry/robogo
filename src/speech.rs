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

type OnError = Box<dyn FnMut(io::Error)>;

pub struct Speech {
    voice: String,
    speech_queue: LinkedList<String>,
    on_error: OnError,
    current: Option<process::Child>,
}

impl Speech {
    pub fn new(settings: Settings) -> Speech {
        Speech {
            voice: settings.voice,
            speech_queue: LinkedList::new(),
            on_error: Box::new(|_| {}),
            current: None,
        }
    }

    pub fn on_error<F>(&mut self, f: F) -> &Self
    where
        F: FnMut(io::Error) + 'static,
    {
        self.on_error = Box::new(f);
        self
    }

    pub fn say(&mut self, text: &str) {
        self.speech_queue.push_back(String::from(text));
    }

    pub fn step(&mut self) {
        if let Some(current) = &mut self.current {
            match current.try_wait() {
                Ok(maybe_status) => {
                    if let Some(_) = maybe_status {
                        self.current = None;
                    }
                }
                Err(e) => (self.on_error)(e),
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
                    Err(e) => (self.on_error)(e),
                }
            }
        }
    }
}
