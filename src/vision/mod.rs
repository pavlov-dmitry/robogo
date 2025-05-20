mod proc;

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;
use std::time::SystemTime;

use crate::board::has_diff;

use super::board::{self, Board, Cell, Position};
use proc::Settings;

use opencv::core::Mat;
use opencv::prelude::*;
use opencv::videoio::{self, VideoCapture};

type OnBoardUpdated = Box<dyn FnMut(Board)>;
type OnError = Box<dyn FnMut(Error)>;

enum Message {
    Board(Box<Board>),
    Error(Box<proc::Error>),
}

pub enum Error {
    ProcError(proc::Error),
    CameraNotOpened,
    CameraSetParamsError,
    Disconnected,
}

impl From<proc::Error> for Error {
    fn from(value: proc::Error) -> Self {
        Error::ProcError(value)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Vision {
    settings: Settings,
    on_board_updated: OnBoardUpdated,
    on_error: OnError,
    thread_handler: Option<JoinHandle<()>>,
    rx: Option<Receiver<Message>>,
    quit_tx: Option<Sender<()>>,
    camera: Option<VideoCapture>,
}

impl Vision {
    pub fn new(settings: Settings) -> Result<Vision> {
        let mut cam = VideoCapture::new(0, videoio::CAP_ANY)?;
        if !cam.is_opened()? {
            return Err(Error::CameraNotOpened);
        }
        let width_success = cam.set(videoio::CAP_PROP_FRAME_WIDTH, 1920.0)?;
        let height_success = cam.set(videoio::CAP_PROP_FRAME_HEIGHT, 1080.0)?;
        if !width_success || !height_success {
            return Err(Error::CameraSetParamsError);
        }

        Ok(Vision {
            settings: settings,
            on_board_updated: Box::new(|_| {}),
            on_error: Box::new(|_| {}),
            thread_handler: None,
            rx: None,
            quit_tx: None,
            camera: Some(cam),
        })
    }

    pub fn on_board_updated<F>(&mut self, cb: F) -> &Self
    where
        F: FnMut(Board) + 'static,
    {
        self.on_board_updated = Box::new(cb);
        self
    }

    pub fn on_error<F>(&mut self, cb: F) -> &Self
    where
        F: FnMut(Error) + 'static,
    {
        self.on_error = Box::new(cb);
        self
    }

    fn read_board(camera: &mut VideoCapture, settings: &Settings) -> opencv::Result<Option<Board>> {
        let mut frame = Mat::default();
        camera.read(&mut frame)?;
        if frame.empty() {
            return Ok(None);
        }

        if let Some(border) = proc::find_board_border(settings, &frame)? {
            let warped_img = proc::warp_board_by_border(settings, &border, &frame)?;
            let board = proc::find_stones(settings, &warped_img, 19)?;
            return Ok(Some(board));
        }

        Ok(None)
    }

    fn main_loop(
        mut camera: VideoCapture,
        settings: Settings,
        tx: Sender<Message>,
        quit_rx: Receiver<()>,
    ) {
        let mut sended_board = Board::default();
        let mut last_board = Board::default();
        let mut last_board_time = std::time::SystemTime::now();

        loop {
            if quit_rx.try_recv().is_ok() {
                break;
            }
            match Vision::read_board(&mut camera, &settings) {
                Ok(maybe_board) => {
                    if let Some(brd) = maybe_board {
                        if board::has_diff(&last_board, &brd) {
                            last_board = brd;
                            last_board_time = std::time::SystemTime::now();
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Message::Error(Box::new(e)));
                    break;
                }
            }
            if let Ok(dur) = last_board_time.elapsed() {
                if dur.as_millis() > 1500 && has_diff(&sended_board, &last_board) {
                    sended_board = last_board.clone();
                    tx.send(Message::Board(Box::new(sended_board.clone())));
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    pub fn spawn(&mut self) {
        let (tx, rx) = mpsc::channel::<Message>();
        let (quit_tx, quit_rx) = mpsc::channel::<()>();
        let camera = self.camera.take().unwrap();
        let settings = self.settings.clone();
        let handler = std::thread::spawn(move || {
            Vision::main_loop(camera, settings, tx, quit_rx);
        });
        self.thread_handler = Some(handler);
        self.rx = Some(rx);
        self.quit_tx = Some(quit_tx);
    }

    pub fn step(&mut self) {
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(msg) => match msg {
                    Message::Board(brd) => (self.on_board_updated)(*brd),
                    Message::Error(e) => (self.on_error)(Error::ProcError(*e)),
                },
                Err(e) => match e {
                    TryRecvError::Disconnected => (self.on_error)(Error::Disconnected),
                    TryRecvError::Empty => {}
                },
            }
        }
    }
}

impl Drop for Vision {
    fn drop(&mut self) {
        if let Some(rx) = self.quit_tx.take() {
            let _ = rx.send(());
        }
        if let Some(handler) = self.thread_handler.take() {
            let _ = handler.join();
        }
    }
}
