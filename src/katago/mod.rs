mod gtp;
mod parse;

use std::collections::LinkedList;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;

use super::board::{self, Color};
use gtp::{Gtp, Settings, State};

type OnStateUpdate = Box<dyn FnMut(State)>;
type OnError = Box<dyn FnMut(Error)>;

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
    Play(Color, board::Position),
    GenMove(Color),
}
enum Msg {
    Error(Error),
    State(State),
}

struct Katago {
    gtp: Option<Gtp>,
    thread_handler: Option<JoinHandle<()>>,
    on_error: OnError,
    msg_rx: Option<Receiver<Msg>>,
    cmd_tx: Option<Sender<Cmd>>,
    state_handlers: LinkedList<OnStateUpdate>,
}

impl Katago {
    pub fn new(settings: Settings) -> Result<Self> {
        Ok(Katago {
            gtp: Some(Gtp::new(settings)?),
            thread_handler: None,
            on_error: Box::new(|_| {}),
            msg_rx: None,
            cmd_tx: None,
            state_handlers: LinkedList::new(),
        })
    }

    pub fn on_error<F>(&mut self, f: F) -> &Self
    where
        F: FnMut(Error) + 'static,
    {
        self.on_error = Box::new(f);
        self
    }

    pub fn spawn(&mut self) -> &Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
        let (msg_tx, msg_rx) = mpsc::channel::<Msg>();
        let gtp = self.gtp.take().expect("GTP must be created.");
        let handler = std::thread::spawn(move || {
            Katago::main_loop(gtp, cmd_rx, msg_tx);
        });
        self.thread_handler = Some(handler);
        self.msg_rx = Some(msg_rx);
        self.cmd_tx = Some(cmd_tx);
        self
    }

    pub fn play<F>(&mut self, color: Color, pos: board::Position, f: F)
    where
        F: FnMut(State) + 'static,
    {
        if let Some(tx) = &self.cmd_tx {
            tx.send(Cmd::Play(color, pos));
            self.state_handlers.push_back(Box::new(f));
        }
    }

    pub fn genmove_for<F>(&mut self, color: Color, f: F)
    where
        F: FnMut(State) + 'static,
    {
        if let Some(tx) = &self.cmd_tx {
            tx.send(Cmd::GenMove(color));
            self.state_handlers.push_back(Box::new(f));
        }
    }

    pub fn step(&mut self) {
        if let Some(rx) = &self.msg_rx {
            match rx.try_recv() {
                Ok(msg) => match msg {
                    Msg::Error(e) => (self.on_error)(e),
                    Msg::State(s) => {
                        let mut handler = self
                            .state_handlers
                            .pop_front()
                            .take()
                            .expect("Handlers linked list error");
                        (handler)(s);
                    }
                },
                Err(e) => match e {
                    TryRecvError::Disconnected => (self.on_error)(Error::Disconnected),
                    TryRecvError::Empty => {}
                },
            }
        }
    }

    fn gtp_play(gtp: &mut Gtp, color: Color, pos: board::Position) -> Result<State> {
        gtp.play(color, pos)?;
        let state = gtp.get_current_state()?;
        Ok(state)
    }

    fn gtp_genmove_for(gtp: &mut Gtp, color: Color) -> Result<State> {
        let _ = gtp.genmove_for(color)?;
        let state = gtp.get_current_state()?;
        Ok(state)
    }

    fn send(msg_tx: &mut Sender<Msg>, maybe_state: Result<State>) {
        match maybe_state {
            Ok(state) => {
                msg_tx.send(Msg::State(state));
            }
            Err(e) => {
                msg_tx.send(Msg::Error(e));
            }
        }
    }

    fn main_loop(mut gtp: Gtp, cmd_rx: Receiver<Cmd>, mut msg_tx: Sender<Msg>) {
        if let Err(e) = gtp.wait_gtp_ready() {
            let _ = msg_tx.send(Msg::Error(Error::from(e)));
            return;
        }
        for cmd in cmd_rx.iter() {
            match cmd {
                Cmd::Play(color, pos) => {
                    Katago::send(&mut msg_tx, Katago::gtp_play(&mut gtp, color, pos));
                }
                Cmd::GenMove(color) => {
                    Katago::send(&mut msg_tx, Katago::gtp_genmove_for(&mut gtp, color));
                }
            }
        }
    }
}
