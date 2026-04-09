use opencv::{
    Result, calib3d,
    core::{self, Point, Point2f, Point2i, Point3f, Rect, Scalar, Size, Vec3b, Vector},
    imgcodecs, imgproc,
    prelude::*,
};

use super::{Board, Cell, Pos};

static CAMERA_MATRIX: &str = "camera_matrix";
static DISTORTION_COEFFICIENTS: &str = "distortion_coefficients";
static VISION_DUMP_DIR: &str = "./vision_dump/";

type Polygon = Vector<Point>;
pub type Error = opencv::Error;

#[derive(Clone)]
pub struct Settings {
    min_board_border_perimeter: f64,
    board_width: i32,
    board_height: i32,
    stones_left_shift: i32,
    stones_right_shift: i32,
    stones_top_shift: i32,
    stones_bottom_shift: i32,
    stones_read_square_size: i32,
    white_stone_threshold: u8,
    black_stone_threshol: u8,
    board_hue_value_from: u8,
    board_hue_value_to: u8,
}

impl Settings {
    pub fn default() -> Settings {
        Settings {
            min_board_border_perimeter: 3200.,
            board_width: 1000,
            board_height: 1000,
            stones_left_shift: 17,
            stones_right_shift: 16,
            stones_top_shift: 5,
            stones_bottom_shift: 18,
            stones_read_square_size: 12,
            white_stone_threshold: 190,
            black_stone_threshol: 60,
            board_hue_value_from: 5,
            board_hue_value_to: 32,
        }
    }
}

pub fn convert_to_grayscale(img: &Mat) -> Result<Mat> {
    // Конвертация в grayscale
    let mut gray = Mat::default();
    imgproc::cvt_color(&img, &mut gray, imgproc::COLOR_BGR2GRAY, 0)?;
    return Ok(gray);
}

// Если и возвращает то это полигон с 4мя точками
pub fn find_board_border(
    settings: &Settings,
    img: &Mat,
    is_dump_steps: bool,
) -> Result<Option<Polygon>> {
    // бинаризация по порогу
    let mut binary = Mat::default();
    imgproc::adaptive_threshold(
        &img,
        &mut binary,
        255.0,
        imgproc::ADAPTIVE_THRESH_MEAN_C,
        imgproc::THRESH_BINARY_INV,
        31,
        7.,
    )?;
    if is_dump_steps {
        opencv::imgcodecs::imwrite(
            &(String::from(VISION_DUMP_DIR) + "binary.jpg"),
            &binary,
            &core::Vector::default(),
        )?;
    }

    // Поиск контуров
    let mut contours: Vector<Polygon> = Vector::new();
    imgproc::find_contours(
        &binary,
        &mut contours,
        imgproc::RETR_LIST,
        imgproc::CHAIN_APPROX_SIMPLE,
        core::Point::default(),
    )?;

    // ищем самый большой четырёхуголник
    let mut best_perimeter = std::f64::MIN;
    let mut best_polygon: Option<Polygon> = Option::None;
    for contour in &contours {
        let perimeter = imgproc::arc_length(&contour, true)?;
        // сразу отсекаем полигоны раные всей картинке
        if perimeter as i32 == (img.cols() * 2 + img.rows() * 2) {
            continue;
        }
        let mut polygon: Vector<Point> = Vector::new();
        // апроксимация полигонов
        imgproc::approx_poly_dp(&contour, &mut polygon, 0.002 * perimeter, true)?;
        // поиск четрехугольника
        if polygon.len() == 4 && imgproc::is_contour_convex(&polygon)? {
            // полигоны с нулевой точкой это полигоны на весь экран, такое нам не нужно
            let has_zero_zero_pnt = polygon.iter().any(|pnt| pnt.x == 0 && pnt.y == 0);
            if has_zero_zero_pnt {
                continue;
            }
            // с самым большим периметром
            if perimeter > settings.min_board_border_perimeter && perimeter > best_perimeter {
                best_perimeter = perimeter;
                best_polygon = Some(polygon);
            }
        }
    }

    if is_dump_steps {
        let mut all = Mat::default();
        imgproc::cvt_color(&img, &mut all, imgproc::COLOR_GRAY2BGR, 0)?;

        imgproc::draw_contours(
            &mut all,
            &contours,
            -1,                                // Индекс контура (-1 = все контуры)
            Scalar::new(0.0, 255.0, 0.0, 0.0), // Зелёный цвет
            2,                                 // Толщина линии
            imgproc::LINE_8,
            &Mat::default(),
            std::i32::MAX,
            Point::default(),
        )?;
        opencv::imgcodecs::imwrite(
            &(String::from(VISION_DUMP_DIR) + "all_contours.jpg"),
            &all,
            &core::Vector::default(),
        )?;

        let mut img_with_border = Mat::default();
        imgproc::cvt_color(&img, &mut img_with_border, imgproc::COLOR_GRAY2BGR, 0)?;
        if let Some(poly) = &best_polygon {
            // Рисуем контуры на исходном изображении
            let mut polygon_for_draw: Vector<Vector<Point>> = Vector::new();
            polygon_for_draw.push(poly.clone());
            imgproc::draw_contours(
                &mut img_with_border,
                &polygon_for_draw,
                -1,                                // Индекс контура (-1 = все контуры)
                Scalar::new(0.0, 255.0, 0.0, 0.0), // Зелёный цвет
                2,                                 // Толщина линии
                imgproc::LINE_8,
                &Mat::default(),
                std::i32::MAX,
                Point::default(),
            )?;
        }
        opencv::imgcodecs::imwrite(
            &(String::from(VISION_DUMP_DIR) + "border.jpg"),
            &img_with_border,
            &core::Vector::default(),
        )?;
        opencv::imgcodecs::imwrite(
            &(String::from(VISION_DUMP_DIR) + "origin.jpg"),
            &img,
            &core::Vector::default(),
        )?;
    }
    Ok(best_polygon)
}

pub fn warp_board_by_border(settings: &Settings, border: &Polygon, img: &Mat) -> Result<Mat> {
    //так как полигон может быть непонятно как развернут то и в который мы придем надо тоже развернуть, возможны проблемы с ромбом
    let poly_f: Vector<Point2f> = border
        .iter()
        .map(|p| Point2f::new(p.x as f32, p.y as f32))
        .collect();

    let (sum_x, sum_y) = poly_f
        .iter()
        .fold((0., 0.), |(sx, sy), p| (sx + p.x, sy + p.y));

    let mean_x = sum_x / border.len() as f32;
    let mean_y = sum_y / border.len() as f32;

    let dst_poly: Vector<Point2f> = poly_f
        .iter()
        .map(|p| {
            let x = if p.x > mean_x {
                settings.board_width as f32
            } else {
                0.
            };
            let y = if p.y > mean_y {
                settings.board_height as f32
            } else {
                0.
            };
            Point2f::new(x, y)
        })
        .collect();

    let transform_matrix = imgproc::get_perspective_transform(&poly_f, &dst_poly, core::DECOMP_LU)?;
    let mut warped = Mat::default();

    imgproc::warp_perspective(
        &img,
        &mut warped,
        &transform_matrix,
        Size::new(settings.board_width, settings.board_height),
        imgproc::INTER_LINEAR,
        core::BORDER_CONSTANT,
        Scalar::default(),
    )?;

    Ok(warped)
}

pub struct TimeMeasure {
    time: std::time::SystemTime,
}

impl TimeMeasure {
    pub fn tick() -> TimeMeasure {
        TimeMeasure {
            time: std::time::SystemTime::now(),
        }
    }

    pub fn print_elapsed_ms_and_tick(&mut self, name: &str) {
        match self.time.elapsed() {
            Ok(t) => println!("{name}: {} ms.", t.as_millis()),
            Err(_) => println!("\"{name}\" error measure time (maybe clock sync)"),
        }
        self.time = std::time::SystemTime::now();
    }
}

fn square_mean_hsv(bgr: &Mat, center: Point2i, radius: i32) -> Result<Vec3b> {
    let min_x = std::cmp::max(center.x - radius, 0);
    let max_x = std::cmp::min(center.x + radius, bgr.cols());
    let min_y = std::cmp::max(center.y - radius, 0);
    let max_y = std::cmp::min(center.y + radius, bgr.rows());
    let roi = Rect::new(min_x, min_y, max_x - min_x, max_y - min_y);
    let region = Mat::roi(bgr, roi)?;
    let mean = core::mean_def(&region)?;
    let bgr_pixel = Mat::new_rows_cols_with_default(1, 1, core::CV_8UC3, mean)?;
    let mut hsv_pixel = Mat::default();
    imgproc::cvt_color(&bgr_pixel, &mut hsv_pixel, imgproc::COLOR_BGR2HSV, 0)?;
    let hsv_values = hsv_pixel.at_2d::<Vec3b>(0, 0)?;
    Ok(*hsv_values)
}

pub fn read_stones(
    settings: &Settings,
    img: &Mat,
    board_size: usize,
    is_dump_steps: bool,
) -> Result<Board> {
    let mut board = Board::new_with_size(board_size);

    let mut debug_img: Option<(Mat, Mat, Mat)> = if is_dump_steps {
        Some((img.clone(), img.clone(), img.clone()))
    } else {
        None
    };

    let horz_shift = settings.stones_left_shift + settings.stones_right_shift;
    let horz_size = img.cols() - horz_shift as i32;
    let vert_shift = settings.stones_top_shift + settings.stones_bottom_shift;
    let vert_size = img.rows() - vert_shift as i32;
    let horz_step = horz_size as f32 / board_size as f32;
    let vert_step = vert_size as f32 / board_size as f32;

    for x in 0..board_size {
        for y in 0..board_size {
            let radius = settings.stones_read_square_size;
            let center_x =
                x as f32 * horz_step + horz_step / 2. + settings.stones_left_shift as f32;
            let center_y = y as f32 * vert_step + vert_step / 2. + settings.stones_top_shift as f32;
            let center = core::Point::new(center_x as i32, center_y as i32);
            let hsv = square_mean_hsv(&img, center, radius)?;
            let value = hsv[2];
            let hue = hsv[0];

            let pos_y = board_size - y - 1;
            // если мы видем цвет доски, то камни там не считываем
            if hue < settings.board_hue_value_from || settings.board_hue_value_to < hue {
                if value <= settings.black_stone_threshol {
                    board.set(Pos::new(x, pos_y), Cell::black_stone());
                } else if value >= settings.white_stone_threshold {
                    board.set(Pos::new(x, pos_y), Cell::white_stone());
                }
            }
            if let Some((h, s, v)) = &mut debug_img {
                let rect =
                    core::Rect::new(center.x - radius, center.y - radius, radius * 2, radius * 2);
                //рисуем прямоуголники в которых считали
                imgproc::rectangle(
                    h,
                    rect,
                    core::Scalar::new(0.0, 0.0, 255.0, 0.0),
                    1,
                    imgproc::LINE_8,
                    0,
                )?;
                imgproc::rectangle(
                    s,
                    rect,
                    core::Scalar::new(0.0, 0.0, 255.0, 0.0),
                    1,
                    imgproc::LINE_8,
                    0,
                )?;
                imgproc::rectangle(
                    v,
                    rect,
                    core::Scalar::new(0.0, 0.0, 255.0, 0.0),
                    1,
                    imgproc::LINE_8,
                    0,
                )?;
                // Подписываем значение
                imgproc::put_text(
                    h,
                    &format!("{}", hsv[0]),
                    core::Point::new(center.x - 10, center.y),
                    imgproc::FONT_HERSHEY_SIMPLEX,
                    0.4,
                    core::Scalar::new(255.0, 0.0, 255.0, 0.0),
                    1,
                    imgproc::LINE_AA,
                    false,
                )?;
                // Подписываем значение
                imgproc::put_text(
                    s,
                    &format!("{}", hsv[1]),
                    core::Point::new(center.x - 10, center.y),
                    imgproc::FONT_HERSHEY_SIMPLEX,
                    0.4,
                    core::Scalar::new(255.0, 0.0, 255.0, 0.0),
                    1,
                    imgproc::LINE_AA,
                    false,
                )?;
                // Подписываем значение
                imgproc::put_text(
                    v,
                    &format!("{}", hsv[2]),
                    core::Point::new(center.x - 10, center.y),
                    imgproc::FONT_HERSHEY_SIMPLEX,
                    0.4,
                    core::Scalar::new(255.0, 0.0, 255.0, 0.0),
                    1,
                    imgproc::LINE_AA,
                    false,
                )?;
            }
        }
    }
    if let Some((h, s, v)) = &debug_img {
        opencv::imgcodecs::imwrite(
            &(String::from(VISION_DUMP_DIR) + "stones_H.jpg"),
            &h,
            &core::Vector::default(),
        )?;
        opencv::imgcodecs::imwrite(
            &(String::from(VISION_DUMP_DIR) + "stones_S.jpg"),
            &s,
            &core::Vector::default(),
        )?;
        opencv::imgcodecs::imwrite(
            &(String::from(VISION_DUMP_DIR) + "stones_V.jpg"),
            &v,
            &core::Vector::default(),
        )?;
    }
    Ok(board)
}

pub struct CameraCalibration {
    camera_matrix: Mat,
    dist_coeffs: Mat,
}

impl CameraCalibration {
    pub fn new() -> Self {
        CameraCalibration {
            camera_matrix: Mat::default(),
            dist_coeffs: Mat::default(),
        }
    }

    pub fn save(&self, filename: &str) -> Result<()> {
        let mut fs = core::FileStorage::new(
            filename,
            core::FileStorage_WRITE | core::FileStorage_FORMAT_JSON,
            "",
        )?;

        fs.write_mat(CAMERA_MATRIX, &self.camera_matrix)?;
        fs.write_mat(DISTORTION_COEFFICIENTS, &self.dist_coeffs)?;
        fs.release()?;
        Ok(())
    }

    pub fn load(filename: &str) -> Result<CameraCalibration> {
        let fs = core::FileStorage::new(
            filename,
            core::FileStorage_READ | core::FileStorage_FORMAT_JSON,
            "",
        )?;

        Ok(CameraCalibration {
            camera_matrix: fs.get(CAMERA_MATRIX)?.mat()?,
            dist_coeffs: fs.get(DISTORTION_COEFFICIENTS)?.mat()?,
        })
    }
}

pub fn calibrate_by(photo_dir: &str, images_count: u32) -> Result<CameraCalibration> {
    let board_size = core::Size::new(9, 6);
    let square_size = 25.0;

    let mut object_points: Vector<Vector<Point3f>> = Vector::new();
    let mut image_points: Vector<Vector<Point2f>> = Vector::new();

    let mut obj_corners: Vector<Point3f> = Vector::new();
    for y in 0..board_size.height {
        for x in 0..board_size.width {
            obj_corners.push(Point3f::new(
                x as f32 * square_size,
                y as f32 * square_size,
                0.0,
            ));
        }
    }

    let mut success_calibrations = 0;
    let mut cols = 0;
    let mut rows = 0;

    for i in 1..=images_count {
        let filename = format!("{photo_dir}/{i}.jpg");
        let img = imgcodecs::imread(&filename, imgcodecs::IMREAD_COLOR)?;
        let img = convert_to_grayscale(&img)?;
        println!("{filename} загружен");
        cols = img.cols();
        rows = img.rows();

        if img.empty() {
            println!("Не удалось загрузить {filename}");
            continue;
        }

        let mut corners: Vector<Point2f> = Vector::new();
        let found = calib3d::find_chessboard_corners(
            &img,
            board_size,
            &mut corners,
            calib3d::CALIB_CB_ADAPTIVE_THRESH + calib3d::CALIB_CB_NORMALIZE_IMAGE,
        )?;
        println!("Попытка найти шахматную доску: {found}");

        if found {
            imgproc::corner_sub_pix(
                &img,
                &mut corners,
                core::Size::new(11, 11),
                core::Size::new(-1, -1),
                core::TermCriteria::new(
                    core::TermCriteria_EPS + core::TermCriteria_MAX_ITER,
                    30,
                    0.001,
                )?,
            )?;

            object_points.push(obj_corners.clone());
            image_points.push(corners);
            success_calibrations += 1;
        }
    }

    println!("Успешно обработано {success_calibrations} изображений");

    let mut camera_calibration = CameraCalibration::new();
    let mut rvecs = Vector::<Mat>::new();
    let mut tvecs = Vector::<Mat>::new();

    calib3d::calibrate_camera_def(
        &object_points,
        &image_points,
        core::Size::new(cols, rows),
        &mut camera_calibration.camera_matrix,
        &mut camera_calibration.dist_coeffs,
        &mut rvecs,
        &mut tvecs,
    )?;

    println!("Калибровка завершена");

    Ok(camera_calibration)
}

pub fn undistord(img: &Mat, camera_calibration: &CameraCalibration) -> Result<Mat> {
    let mut undistorted = Mat::default();
    calib3d::undistort_def(
        &img,
        &mut undistorted,
        &camera_calibration.camera_matrix,
        &camera_calibration.dist_coeffs,
    )?;
    Ok(undistorted)
}
