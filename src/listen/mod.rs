use cpal::traits::*;
use std::{
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread::JoinHandle,
};
use vosk;

pub struct Listen {
    vosk_model: Option<vosk::Model>,
    thread_handler: Option<JoinHandle<()>>,
    text_rx: Option<Receiver<String>>,
}

#[derive(Debug)]
pub enum Error {
    ThreadDisconnected,
}

pub enum Msg {
    Text(String),
    Err(Error),
}

impl Listen {
    pub fn new() -> Listen {
        Listen {
            vosk_model: None,
            thread_handler: None,
            text_rx: None,
        }
    }

    pub fn step(&self) -> Option<Msg> {
        if let Some(rx) = &self.text_rx {
            match rx.try_recv() {
                Ok(txt) => Some(Msg::Text(txt)),
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

        let (text_tx, text_rx) = mpsc::channel::<String>();
        let handler = std::thread::spawn(move || {
            Listen::main_loop(recognizer, text_tx);
        });
        self.thread_handler = Some(handler);
        self.text_rx = Some(text_rx);
    }

    fn main_loop(mut recognizer: vosk::Recognizer, text_tx: Sender<String>) {
        let host = cpal::default_host();
        let device = host.default_input_device().expect("No input audio device");
        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(16000),
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
                            let _ = text_tx.send(String::from(result.text));
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
}
