mod board;
mod katago;
mod vision;

use board::Board;
use katago::Katago;
use opencv::{Result, core::Vector, highgui, prelude::*, videoio};

use std::path::Path;
use std::{fs, io};

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    //    let load_board_from = |filename| -> Result<board::Board> {
    //        let img = opencv::imgcodecs::imread(filename, opencv::imgcodecs::IMREAD_COLOR)?;
    //        if img.empty() {
    //            panic!("Не удалось загрузить изображение!");
    //        }
    //        let vision_settings = vision::Settings::default();
    //        let border_polygon = vision::find_board_border(&vision_settings, &img)?;
    //        let border = border_polygon.expect("Не найдено поле");
    //        let warped_img = vision::warp_board_by_border(&vision_settings, &border, &img)?;
    //        let board = vision::find_stones(&vision_settings, &warped_img, 19)?;
    //        Ok(board)
    //    };

    //    let board = load_board_from("/home/deck/development/robogo_tests/0/6.jpg").expect("ooops");
    //    println!("{}", board);

    //    let mut cam = videoio::VideoCapture::new(0, videoio::CAP_ANY)?;
    //    if !cam.is_opened()? {
    //        panic!("Не удалось открыть камеру");
    //    }
    //    let width_success = cam.set(videoio::CAP_PROP_FRAME_WIDTH, 1920.0)?;
    //    let height_success = cam.set(videoio::CAP_PROP_FRAME_HEIGHT, 1080.0)?;
    //    println!("width: {}   height: {}", width_success, height_success);

    //    highgui::named_window("Camera", highgui::WINDOW_NORMAL)?;
    //    let mut frame = Mat::default();

    //    let mut state = Board::default();
    //    let mut counter = 0;

    //    loop {
    //        cam.read(&mut frame)?;
    //        if frame.empty() {
    //            continue;
    //        }

    //        opencv::imgcodecs::imwrite("./original.jpg", &frame, &Vector::default())?;

    //        let vision_settings = vision::Settings::default();
    //        let img = vision::convert_to_grayscale(&frame)?;
    //        let border_polygon = vision::find_board_border(&vision_settings, &img)?;
    //        if let Some(border) = border_polygon {
    //            let warped_img = vision::warp_board_by_border(&vision_settings, &border, &img)?;
    //            let board = vision::find_stones(&vision_settings, &warped_img, 19)?;

    //            let actions = board::diff(&state, &board);
    //            if !actions.is_empty() {
    //                if actions.len() > 1 {
    //                    counter += 1;
    //                    let out_dir = format!("./error_{}", counter);
    //                    let _ = copy_dir_all("./vision_dump", &out_dir);
    //                }
    //                println!("____________________________________________________");
    //                for action in actions {
    //                    println!("{}", action);
    //                }
    //            }
    //            state = board;
    //        } else {
    //            //println!("____________________________________________________");
    //            //println!("Не найдено поле")
    //        }

    //        highgui::imshow("Camera", &frame)?;

    //        if highgui::wait_key(10)? == 27 {
    //            break;
    //        }
    //    }

    let mut katago = Katago::new(katago::Settings::default()).expect("error create katago engine");
    println!("katago started.");
    katago.wait_gtp_ready().expect("error wait for ready");
    println!("gtp ready");
    let response = katago.send("version").expect("cannot send command");
    println!("answer: {}", response);
    let response = katago.send("showboard").expect("write to process error");
    println!("answer: {}", response);

    Ok(())
}
