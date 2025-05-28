use opencv::{
    Result, calib3d,
    core::{self, Point, Point2f, Point3f, Scalar, Size, Vector},
    imgcodecs, imgproc,
    prelude::*,
};

use super::{Board, Cell, Pos};

static CAMERA_MATRIX: &str = "camera_matrix";
static DISTORTION_COEFFICIENTS: &str = "distortion_coefficients";

type Polygon = Vector<Point>;
pub type Error = opencv::Error;

#[derive(Clone)]
pub struct Settings {
    binary_threshold: f64,
    min_board_border_perimeter: f64,
    board_width: i32,
    board_height: i32,
    stones_left_shift: f64,
    stones_right_shift: f64,
    stones_top_shift: f64,
    stones_bottom_shift: f64,
    stone_radius: i32,
    white_stone_threshold: u8,
    black_stone_threshold: u8,
    min_color_threshold: u8,
    pub is_dump_steps: bool,
    pub dump_dir: String,
}

impl Settings {
    pub fn default() -> Settings {
        Settings {
            binary_threshold: 100.,
            min_board_border_perimeter: 2500.,
            board_width: 1000,
            board_height: 1000,
            stones_left_shift: 17.,
            stones_right_shift: 16.,
            stones_top_shift: 5.,
            stones_bottom_shift: 17.,
            stone_radius: 13,
            white_stone_threshold: 190,
            black_stone_threshold: 60,
            min_color_threshold: 16,
            is_dump_steps: true,
            dump_dir: String::from("./vision_dump/"),
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
pub fn find_board_border(settings: &Settings, img: &Mat) -> Result<Option<Polygon>> {
    let gray = convert_to_grayscale(img)?;
    // бинаризация по порогу
    let mut binary = Mat::default();
    imgproc::threshold(
        &gray,
        &mut binary,
        settings.binary_threshold,
        255.0,
        imgproc::THRESH_BINARY_INV,
    )?;
    if settings.is_dump_steps {
        opencv::imgcodecs::imwrite(
            &(settings.dump_dir.clone() + "binary.jpg"),
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
    for contour in contours {
        let perimeter = imgproc::arc_length(&contour, true)?;
        // сразу отсекаем полигоны раные всей картинке
        if perimeter as i32 == (gray.cols() * 2 + gray.rows() * 2) {
            continue;
        }
        let mut polygon: Vector<Point> = Vector::new();
        // апроксимация полигонов
        imgproc::approx_poly_dp(&contour, &mut polygon, 0.005 * perimeter, true)?;
        // поиск четрехугольника
        if polygon.len() == 4 && imgproc::is_contour_convex(&polygon)? {
            // полигоны с нулевой точкой это полигоны на весь экран, такое нам не нужно
            let has_zero_zero_pnt = polygon.iter().any(|pnt| pnt.x == 0 && pnt.y == 0);
            if has_zero_zero_pnt {
                continue;
            }
            let perimeter = imgproc::arc_length(&polygon, false)?;
            // с самым большим периметром
            if perimeter > settings.min_board_border_perimeter && perimeter > best_perimeter {
                best_perimeter = perimeter;
                best_polygon = Some(polygon);
            }
        }
    }

    if settings.is_dump_steps {
        let mut img_with_border = Mat::default();
        imgproc::cvt_color(&gray, &mut img_with_border, imgproc::COLOR_GRAY2BGR, 0)?;
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
            &(settings.dump_dir.clone() + "border.jpg"),
            &img_with_border,
            &core::Vector::default(),
        )?;
        opencv::imgcodecs::imwrite(
            &(settings.dump_dir.clone() + "origin.jpg"),
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

pub fn find_stones(settings: &Settings, img: &Mat, board_size: usize) -> Result<Board> {
    let mut board = Board::new_with_size(board_size);
    // Создаём маску для круглой области
    let mut mask = Mat::zeros(img.rows(), img.cols(), core::CV_8UC1)?.to_mat()?;

    let mut debug_img: Option<Mat> = if settings.is_dump_steps {
        Some(img.clone())
    } else {
        None
    };

    let mut lab = Mat::default();
    imgproc::cvt_color(&img, &mut lab, imgproc::COLOR_BGR2Lab, 0)?;

    let horz_shift = settings.stones_left_shift + settings.stones_right_shift;
    let horz_size = img.cols() - horz_shift as i32;
    let vert_shift = settings.stones_top_shift + settings.stones_bottom_shift;
    let vert_size = img.rows() - vert_shift as i32;
    let horz_step = horz_size as f64 / board_size as f64;
    let vert_step = vert_size as f64 / board_size as f64;

    for x in 0..board_size {
        for y in 0..board_size {
            let radius = settings.stone_radius; // Радиус круга
            let center_x = x as f64 * horz_step + horz_step / 2. + settings.stones_left_shift;
            let center_y = y as f64 * vert_step + vert_step / 2. + settings.stones_top_shift;
            let center = core::Point::new(center_x as i32, center_y as i32);
            mask.set_to(&Scalar::all(0.0), &core::no_array())?;
            imgproc::circle(
                &mut mask,
                center,
                radius,
                core::Scalar::all(255.0),
                -1, // Заливка
                imgproc::LINE_8,
                0,
            )?;
            let mean = core::mean(&lab, &mask)?;
            let l = mean[0] as u8;
            let a = mean[1] as u8;
            let b = mean[2] as u8;
            let a = a as f64 - 128.;
            let b = b as f64 - 128.;
            let color = (a * a + b * b).sqrt() as u8;

            let pos_y = board_size - y - 1;
            if l < settings.black_stone_threshold && color <= settings.min_color_threshold {
                board.set(Pos::new(x, pos_y), Cell::black_stone());
            } else if l > settings.white_stone_threshold && color <= settings.min_color_threshold {
                board.set(Pos::new(x, pos_y), Cell::white_stone());
            } else {
                board.set(Pos::new(x, pos_y), Cell::empty());
            }

            if let Some(image) = &mut debug_img {
                //рисуем кружочки
                imgproc::circle(
                    image,
                    center,
                    radius,
                    core::Scalar::new(0.0, 0.0, 255.0, 0.0),
                    1,
                    imgproc::LINE_8,
                    0,
                )?;
                // Подписываем значение
                imgproc::put_text(
                    image,
                    &format!("{}/{}", l, color),
                    core::Point::new(center.x - 20, center.y),
                    imgproc::FONT_HERSHEY_SIMPLEX,
                    0.35,
                    core::Scalar::new(255.0, 0.0, 255.0, 0.0),
                    1,
                    imgproc::LINE_AA,
                    false,
                )?;
            }
        }
    }
    if let Some(image) = &debug_img {
        opencv::imgcodecs::imwrite(
            &(settings.dump_dir.clone() + "stones.jpg"),
            &image,
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
