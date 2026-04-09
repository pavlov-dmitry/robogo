use cpal::traits::*;
use std::{
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread::JoinHandle,
};
use thiserror::Error;
use vosk;

type CmdParserPtr = Box<dyn CmdParser>;

pub struct Listen {
    vosk_model: Option<vosk::Model>,
    thread_handler: Option<JoinHandle<()>>,
    msg_rx: Option<Receiver<Msg>>,
    voice_settings: VoiceCommandsSettings,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Потом аварийно завершился")]
    ThreadDisconnected,
}

#[derive(Debug, Clone)]
pub enum VoiceCmd {
    Yes,
    No,
    Pass,
    Resign,
}

pub enum Msg {
    Text(String),
    Cmd(VoiceCmd),
    Err(Error),
}

#[derive(Clone)]
pub struct VoiceCommandsSettings {
    name: String,
    yes: Vec<String>,
    no: Vec<String>,
    ingame_resign: Vec<String>,
    ingame_pass: Vec<String>,
}

impl Default for VoiceCommandsSettings {
    fn default() -> Self {
        VoiceCommandsSettings {
            name: String::from("сай"),
            yes: vec![String::from("да")],
            no: vec![String::from("нет")],
            ingame_resign: vec![String::from("сдаюсь"), String::from("я сдаюсь")],
            ingame_pass: vec![String::from("пас"), String::from("я пасую")],
        }
    }
}

trait CmdParser {
    fn parse(&self, txt: &str) -> Option<VoiceCmd>;
}

struct SimpleCmdParser {
    patterns: Vec<String>,
    val: VoiceCmd,
}

impl SimpleCmdParser {
    fn new(ptrns: &Vec<String>, val: VoiceCmd) -> SimpleCmdParser {
        SimpleCmdParser {
            patterns: ptrns.clone(),
            val: val,
        }
    }
}

impl CmdParser for SimpleCmdParser {
    fn parse(&self, txt: &str) -> Option<VoiceCmd> {
        for pattern in &self.patterns {
            if txt.starts_with(pattern) {
                return Some(self.val.clone());
            }
        }
        None
    }
}

fn create_simple_processor(ptrns: &Vec<String>, val: VoiceCmd) -> CmdParserPtr {
    Box::new(SimpleCmdParser::new(ptrns, val))
}

fn create_voice_cmd_parsers(voice_cmd_settings: &VoiceCommandsSettings) -> Vec<CmdParserPtr> {
    let mut result = Vec::new();
    result.push(create_simple_processor(
        &voice_cmd_settings.yes,
        VoiceCmd::Yes,
    ));
    result.push(create_simple_processor(
        &voice_cmd_settings.no,
        VoiceCmd::No,
    ));
    result.push(create_simple_processor(
        &voice_cmd_settings.ingame_pass,
        VoiceCmd::Pass,
    ));
    result.push(create_simple_processor(
        &voice_cmd_settings.ingame_resign,
        VoiceCmd::Resign,
    ));
    result
}

impl Listen {
    pub fn new(voice_settings: VoiceCommandsSettings) -> Listen {
        Listen {
            vosk_model: None,
            thread_handler: None,
            msg_rx: None,
            voice_settings: voice_settings,
        }
    }

    pub fn step(&self) -> Option<Msg> {
        if let Some(rx) = &self.msg_rx {
            match rx.try_recv() {
                Ok(msg) => Some(msg),
                Err(e) => match e {
                    TryRecvError::Empty => None,
                    TryRecvError::Disconnected => Some(Msg::Err(Error::ThreadDisconnected)),
                },
            }
        } else {
            None
        }
    }

    pub fn spawn(&mut self) {
        let vosk_model =
            vosk::Model::new("./vosk-model-small-ru-0.22").expect("can not create Vosk model");
        self.vosk_model = Some(vosk_model);
        let recognizer = vosk::Recognizer::new(self.vosk_model.as_ref().unwrap(), 16000.0)
            .expect("can not create Vosk Recognizer");

        let voice_settings = self.voice_settings.clone();
        let (msg_tx, msg_rx) = mpsc::channel::<Msg>();
        let handler = std::thread::spawn(move || {
            Listen::main_loop(recognizer, voice_settings, msg_tx);
        });
        self.thread_handler = Some(handler);
        self.msg_rx = Some(msg_rx);
    }

    fn main_loop(
        mut recognizer: vosk::Recognizer,
        voice_settings: VoiceCommandsSettings,
        msg_tx: Sender<Msg>,
    ) {
        let parsers = create_voice_cmd_parsers(&voice_settings);
        let host = cpal::default_host();
        let device = host.default_input_device().expect("No input audio device");
        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: 16000,
            buffer_size: cpal::BufferSize::Default,
        };
        let (tx, rx) = mpsc::channel::<Vec<i16>>();
        let stream = device
            .build_input_stream(
                &config,
                move |data: &[i16], _| {
                    let d = Vec::from(data);
                    let _ = tx.send(d);
                },
                move |err| {
                    //TODO: сделать нормальную обработу ошибок
                    println!("MICROPHONE STREAM ERROR: {}", err);
                },
                None,
            )
            .unwrap();
        stream.play().expect("cant start microphone stream.");

        loop {
            let data = rx.recv();
            match data {
                Ok(d) => match recognizer.accept_waveform(&d) {
                    Ok(state) => match state {
                        vosk::DecodingState::Finalized => {
                            let result = recognizer.result().single().expect("single result");
                            if !result.text.is_empty() {
                                let _ = msg_tx.send(Msg::Text(String::from(result.text)));
                                match Listen::process_cmds(
                                    result.text,
                                    &voice_settings.name,
                                    &parsers,
                                ) {
                                    Some(cmd) => {
                                        let _ = msg_tx.send(Msg::Cmd(cmd));
                                    }
                                    None => {}
                                }
                            }
                        }
                        _ => {}
                    },
                    Err(e) => println!("Error: {}", e),
                },
                Err(e) => {
                    println!("Recieve Error: {}", e);
                }
            }
        }
    }

    fn process_cmds(txt: &str, name: &str, parsers: &Vec<CmdParserPtr>) -> Option<VoiceCmd> {
        let name_chars_count = name.chars().count();
        //сначало в строке ищем наше имя
        match txt.find(name) {
            Some(index) => {
                println!("name found: {index}");
                let start_with_name = &txt[index..];
                let all_chars = start_with_name.chars();
                if name_chars_count + 1 >= all_chars.clone().count() {
                    return None;
                }
                let after_name: String = all_chars.skip(name_chars_count + 1).collect();

                //после имени должны быть наши команды. пока рассчитываем что комнада сразу была слитно произенесена без пауз
                for cmd_parser in parsers {
                    if let Some(cmd) = cmd_parser.parse(&after_name) {
                        return Some(cmd);
                    }
                }
                None
            }
            None => None,
        }
    }
}
