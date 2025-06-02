mod proc;
use chrono::Local;

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;

use super::board::{self, Board, Cell, Pos, has_diff};

use opencv::core::Mat;
use opencv::highgui;
use opencv::imgcodecs;
use opencv::prelude::*;
use opencv::videoio::{self, VideoCapture};

static DEFAULT_CAMERA_CALIBRATION_PATH: &str = "./etc/camera_calibration.json";

#[derive(Clone)]
pub struct Settings {
    proc: proc::Settings,
    stable_board_time_ms: u32,
}

impl Settings {
    pub fn default() -> Self {
        Settings {
            proc: proc::Settings::default(),
            stable_board_time_ms: 750,
        }
    }
}

enum VisionMsg {
    Board(Box<Board>),
    Error(proc::Error),
}

pub enum Msg {
    Board(Box<Board>),
    Error(Error),
}

#[allow(dead_code)]
#[derive(Debug)]
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
    thread_handler: Option<JoinHandle<()>>,
    rx: Option<Receiver<VisionMsg>>,
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
            thread_handler: None,
            rx: None,
            quit_tx: None,
            camera: Some(cam),
        })
    }

    fn read_board(camera: &mut VideoCapture, settings: &Settings) -> opencv::Result<Option<Board>> {
        let mut frame = Mat::default();
        camera.read(&mut frame)?;
        if frame.empty() {
            return Ok(None);
        }

        if let Some(border) = proc::find_board_border(&settings.proc, &frame)? {
            let warped_img = proc::warp_board_by_border(&settings.proc, &border, &frame)?;
            let board = proc::find_stones(&settings.proc, &warped_img, 19)?;
            return Ok(Some(board));
        }

        Ok(None)
    }

    fn main_loop(
        mut camera: VideoCapture,
        settings: Settings,
        tx: Sender<VisionMsg>,
        quit_rx: Receiver<()>,
    ) {
        let mut sended_board = Board::default();
        let mut last_board = Board::default();
        let mut last_board_time = std::time::SystemTime::now();
        let mut last_board_updated = false;

        loop {
            if quit_rx.try_recv().is_ok() {
                break;
            }
            match Vision::read_board(&mut camera, &settings) {
                Ok(maybe_board) => {
                    if let Some(brd) = maybe_board {
                        let diff = board::diff(&last_board, &brd);

                        // специальный режим если изменений больше одного камня отбросить фотографии в отдеьлную папку
                        if settings.proc.is_dump_steps && diff.len() > 1 {
                            let dst = format!("./vision_errors/{}/", timestamp());
                            let _ = copy_dir_all(&settings.proc.dump_dir, dst);
                        }

                        if !diff.is_empty() {
                            last_board = brd;
                            last_board_time = std::time::SystemTime::now();
                            last_board_updated = true;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(VisionMsg::Error(e));
                    break;
                }
            }
            if last_board_updated {
                if let Ok(dur) = last_board_time.elapsed() {
                    if dur.as_millis() > settings.stable_board_time_ms as u128
                        && has_diff(&sended_board, &last_board)
                    {
                        sended_board = last_board.clone();
                        if let Err(_) = tx.send(VisionMsg::Board(Box::new(sended_board.clone()))) {
                            break;
                        }
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    pub fn spawn(&mut self) {
        //создаём каналы управления и данными
        let (tx, rx) = mpsc::channel::<VisionMsg>();
        let (quit_tx, quit_rx) = mpsc::channel::<()>();

        //данные для потока
        let camera = self.camera.take().unwrap();
        let settings = self.settings.clone();

        //создаём поток
        let handler = std::thread::spawn(move || {
            Vision::main_loop(camera, settings, tx, quit_rx);
        });

        //сохраняем себе вторые части каналов и хэндлер потока
        self.thread_handler = Some(handler);
        self.rx = Some(rx);
        self.quit_tx = Some(quit_tx);
    }

    pub fn step(&mut self) -> Option<Msg> {
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(msg) => match msg {
                    VisionMsg::Board(brd) => Some(Msg::Board(brd)),
                    VisionMsg::Error(e) => Some(Msg::Error(Error::from(e))),
                },
                Err(e) => match e {
                    TryRecvError::Disconnected => Some(Msg::Error(Error::Disconnected)),
                    TryRecvError::Empty => None,
                },
            }
        } else {
            None
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

pub fn camera_mode() -> Result<()> {
    let mut cam = VideoCapture::new(0, videoio::CAP_ANY)?;
    if !cam.is_opened()? {
        return Err(Error::CameraNotOpened);
    }
    let width_success = cam.set(videoio::CAP_PROP_FRAME_WIDTH, 1920.0)?;
    let height_success = cam.set(videoio::CAP_PROP_FRAME_HEIGHT, 1080.0)?;
    if !width_success || !height_success {
        return Err(Error::CameraSetParamsError);
    }

    highgui::named_window("Camera", highgui::WINDOW_NORMAL)?;
    let mut frame = Mat::default();

    loop {
        cam.read(&mut frame)?;
        if frame.empty() {
            continue;
        }

        highgui::imshow("Camera", &frame)?;

        let key = highgui::wait_key(10)?;
        match key {
            32 => {
                // Пробел
                let filename = format!("photo_{}.jpg", timestamp());
                imgcodecs::imwrite(&filename, &frame, &opencv::core::Vector::new())?;
            }
            27 => break, // Esc
            _ => {}
        }
    }

    Ok(())
}

pub fn parse_board_from(photo_filename: &str, settings: &Settings) -> Result<Option<Board>> {
    let img = imgcodecs::imread(photo_filename, imgcodecs::IMREAD_COLOR)?;
    let border = proc::find_board_border(&settings.proc, &img)?;
    let board = match border {
        Some(border) => {
            let warped = proc::warp_board_by_border(&settings.proc, &border, &img)?;
            let board = proc::find_stones(&settings.proc, &warped, 19)?;
            Some(board)
        }
        None => None,
    };
    Ok(board)
}

pub fn calibrate_by(photo_dir: &str, count: u32) -> Result<()> {
    let cc = proc::calibrate_by(photo_dir, count)?;
    cc.save(DEFAULT_CAMERA_CALIBRATION_PATH)?;
    Ok(())
}

pub fn test_calibration(filename: &str) -> Result<()> {
    let camera_calibration = proc::CameraCalibration::load(DEFAULT_CAMERA_CALIBRATION_PATH)?;
    let img = imgcodecs::imread(filename, imgcodecs::IMREAD_COLOR)?;
    let undistorted = proc::undistord(&img, &camera_calibration)?;
    highgui::named_window("Calibrated", highgui::WINDOW_NORMAL)?;
    highgui::imshow("Calibrated", &undistorted)?;
    highgui::wait_key(0)?;

    Ok(())
}

fn copy_dir_all(
    src: impl AsRef<std::path::Path>,
    dst: impl AsRef<std::path::Path>,
) -> std::io::Result<()> {
    std::fs::create_dir_all(&dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;

        let ty = entry.file_type()?;

        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }

    Ok(())
}

fn timestamp() -> String {
    let now = Local::now();
    format!("{}", now.format("%F_%T%.3f"))
}
