mod gtp;
mod parse;

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;

use super::board::{self, Color, Move};
use gtp::Gtp;
pub use gtp::{Settings, State};

#[derive(Debug)]
pub enum Error {
    Gtp(gtp::Error),
    Disconnected,
}
pub type Result<T> = std::result::Result<T, Error>;

impl From<gtp::Error> for Error {
    fn from(value: gtp::Error) -> Self {
        Error::Gtp(value)
    }
}

enum Cmd {
    Play(Color, board::Move),
    GenMove(Color),
}
pub enum Msg {
    Error(Error),
    State(State),
    Move(Move),
}

pub struct Katago {
    gtp: Option<Gtp>,
    thread_handler: Option<JoinHandle<()>>,
    msg_rx: Option<Receiver<Msg>>,
    cmd_tx: Option<Sender<Cmd>>,
}

impl Katago {
    pub fn new(settings: Settings) -> Result<Self> {
        Ok(Katago {
            gtp: Some(Gtp::new(settings)?),
            thread_handler: None,
            msg_rx: None,
            cmd_tx: None,
        })
    }

    pub fn spawn(&mut self) {
        let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
        let (msg_tx, msg_rx) = mpsc::channel::<Msg>();
        let gtp = self.gtp.take().expect("GTP must be created.");
        let handler = std::thread::spawn(move || {
            Katago::main_loop(gtp, cmd_rx, msg_tx);
        });
        self.thread_handler = Some(handler);
        self.msg_rx = Some(msg_rx);
        self.cmd_tx = Some(cmd_tx);
    }

    pub fn play(&mut self, color: board::Color, mv: board::Move) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(Cmd::Play(color, mv));
        }
    }

    pub fn genmove_for(&mut self, color: Color) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(Cmd::GenMove(color));
        }
    }

    pub fn step(&mut self) -> Option<Msg> {
        if let Some(rx) = &self.msg_rx {
            match rx.try_recv() {
                Ok(msg) => Some(msg),
                Err(e) => match e {
                    TryRecvError::Disconnected => Some(Msg::Error(Error::Disconnected)),
                    TryRecvError::Empty => None,
                },
            }
        } else {
            None
        }
    }

    fn send_state(msg_tx: &mut Sender<Msg>, gtp: &mut Gtp) {
        let _ = match gtp.get_current_state() {
            Ok(state) => msg_tx.send(Msg::State(state)),
            Err(e) => msg_tx.send(Msg::Error(Error::from(e))),
        };
    }

    fn main_loop(mut gtp: Gtp, cmd_rx: Receiver<Cmd>, mut msg_tx: Sender<Msg>) {
        // ждем загрузки katago
        if let Err(e) = gtp.wait_gtp_ready() {
            let _ = msg_tx.send(Msg::Error(Error::from(e)));
            return;
        }
        //обрабатываем очередь команд (тут работает синхронно, дожидаясь ответа на каждую команду)
        for cmd in cmd_rx.iter() {
            match cmd {
                Cmd::Play(color, mv) => {
                    if let Err(e) = gtp.play(color, mv) {
                        let _ = msg_tx.send(Msg::Error(Error::from(e)));
                    }
                    Katago::send_state(&mut msg_tx, &mut gtp);
                }
                Cmd::GenMove(color) => {
                    let _ = match gtp.genmove_for(color) {
                        Ok(mv) => msg_tx.send(Msg::Move(mv)),
                        Err(e) => msg_tx.send(Msg::Error(Error::from(e))),
                    };
                    Katago::send_state(&mut msg_tx, &mut gtp);
                }
            }
        }
    }
}
