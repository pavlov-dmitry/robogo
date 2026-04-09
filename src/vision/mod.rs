mod proc;
use chrono::Local;

use std::string::String;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;
use thiserror::Error;

use super::board::{self, Board, Cell, Pos, has_diff};

use opencv::core::{Mat, Vector};
use opencv::highgui;
use opencv::imgcodecs;
use opencv::prelude::*;
use opencv::videoio::{self, VideoCapture};

use proc::TimeMeasure;

static DEFAULT_CAMERA_CALIBRATION_PATH: &str = "./etc/camera_calibration.json";

#[derive(Clone)]
pub struct Settings {
    pub proc: proc::Settings,
    save_more_than_one_stone_diffs: bool,
    stable_board_frames_count: usize,
}

impl Settings {
    pub fn default() -> Self {
        Settings {
            proc: proc::Settings::default(),
            save_more_than_one_stone_diffs: true,
            stable_board_frames_count: 7,
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
#[derive(Error, Debug)]
pub enum Error {
    #[error("Ошибка обработки кадра")]
    ProcError(#[from] proc::Error),
    #[error("Ошибка подключения к камере")]
    CameraNotOpened,
    #[error("Ошибка настройки камеры")]
    CameraSetParamsError,
    #[error("Поток работы с камерой аварийно завершил свою работу")]
    ThreadDisconnected,
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

    fn read_board(
        camera: &mut VideoCapture,
        settings: &Settings,
        is_dump_steps: bool,
    ) -> opencv::Result<Option<(Board, Mat)>> {
        let mut frame = Mat::default();
        camera.read(&mut frame)?;
        let gray_frame = proc::convert_to_grayscale(&frame)?;
        if gray_frame.empty() {
            return Ok(None);
        }

        if let Some(border) = proc::find_board_border(&settings.proc, &gray_frame, is_dump_steps)? {
            let warped_img = proc::warp_board_by_border(&settings.proc, &border, &frame)?;
            let board = proc::read_stones(&settings.proc, &warped_img, 19, is_dump_steps)?;
            return Ok(Some((board, frame)));
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
        let mut stable_board_frames_count: usize = 0;

        loop {
            if quit_rx.try_recv().is_ok() {
                break;
            }
            match Vision::read_board(&mut camera, &settings, false) {
                Ok(maybe_board) => match maybe_board {
                    Some((brd, frame)) => {
                        let diff = board::diff(&last_board, &brd);

                        if diff.is_empty() {
                            stable_board_frames_count += 1;
                        } else {
                            let is_only_one_stone_added = diff.len() == 1
                                && match diff[0] {
                                    board::Action::Add(_, _) => true,
                                    _ => false,
                                };
                            // специальный режим если изменений больше одного камня отбросить фотографии в отдеьлную папку
                            if settings.save_more_than_one_stone_diffs && !is_only_one_stone_added {
                                let filename = format!(
                                    "./more_than_one_stone_diffs/photo_{}.jpg",
                                    timestamp()
                                );
                                let _ = opencv::imgcodecs::imwrite(
                                    &filename,
                                    &frame,
                                    &Vector::default(),
                                );
                            }

                            last_board = brd;
                            stable_board_frames_count = 0;
                        }
                    }
                    None => {
                        stable_board_frames_count = 0;
                    }
                },
                Err(e) => {
                    let _ = tx.send(VisionMsg::Error(e));
                    break;
                }
            }
            if stable_board_frames_count >= settings.stable_board_frames_count
                && has_diff(&sended_board, &last_board)
            {
                stable_board_frames_count = 0;
                sended_board = last_board.clone();

                if let Err(_) = tx.send(VisionMsg::Board(Box::new(sended_board.clone()))) {
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
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
                    TryRecvError::Disconnected => Some(Msg::Error(Error::ThreadDisconnected)),
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

pub fn parse_board_from(
    photo_filename: &str,
    settings: &Settings,
    is_dump_steps: bool,
) -> Result<Option<Board>> {
    let mut time_measure = TimeMeasure::tick();
    let img = imgcodecs::imread(photo_filename, imgcodecs::IMREAD_COLOR)?;
    time_measure.print_elapsed_ms_and_tick("image read");
    let gray = proc::convert_to_grayscale(&img)?;
    time_measure.print_elapsed_ms_and_tick("convert to gray");

    let mut full_proc_time_measure = TimeMeasure::tick();

    let border = proc::find_board_border(&settings.proc, &gray, is_dump_steps)?;
    time_measure.print_elapsed_ms_and_tick("find board border");

    let board = match border {
        Some(border) => {
            let warped = proc::warp_board_by_border(&settings.proc, &border, &img)?;
            time_measure.print_elapsed_ms_and_tick("warp board");

            let board = proc::read_stones(&settings.proc, &warped, 19, is_dump_steps)?;
            time_measure.print_elapsed_ms_and_tick("read stones");

            Some(board)
        }
        None => None,
    };
    full_proc_time_measure.print_elapsed_ms_and_tick("full proc time");
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

fn timestamp() -> String {
    let now = Local::now();
    format!("{}", now.format("%F_%T%.3f"))
}
