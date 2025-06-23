use opencv::{
    Result, calib3d,
    core::{self, Point, Point2f, Point2i, Point3f, Scalar, Size, Vector},
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
    expected_stone_radius: i32,
    stones_left_shift: i32,
    stones_right_shift: i32,
    stones_top_shift: i32,
    stones_bottom_shift: i32,
    stones_read_square_size: i32,
    grid_left_shift: i32,
    grid_right_shift: i32,
    grid_top_shift: i32,
    grid_bottom_shift: i32,
}

impl Settings {
    pub fn default() -> Settings {
        Settings {
            min_board_border_perimeter: 3200.,
            board_width: 1000,
            board_height: 1000,
            expected_stone_radius: 25,
            stones_left_shift: 17,
            stones_right_shift: 16,
            stones_top_shift: 5,
            stones_bottom_shift: 18,
            stones_read_square_size: 20,
            grid_left_shift: 17,
            grid_right_shift: 16,
            grid_top_shift: 15,
            grid_bottom_shift: 15,
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

fn square_mean(gray: &Mat, center: Point2i, radius: i32) -> u8 {
    let min_x = std::cmp::max(center.x - radius, 0);
    let max_x = std::cmp::min(center.x + radius, gray.cols());
    let min_y = std::cmp::max(center.y - radius, 0);
    let max_y = std::cmp::min(center.y + radius, gray.rows());
    let mut summ: usize = 0;
    let mut count: usize = 0;
    for x in min_x..max_x {
        for y in min_y..max_y {
            unsafe {
                if let Ok(val) = gray.at_2d_unchecked::<u8>(y, x) {
                    summ += *val as usize;
                    count += 1;
                }
            }
        }
    }
    return (summ / count) as u8;
}

fn gen_and_write_grids(img: &Mat, settings: &Settings, board_size: usize) -> Result<()> {
    // сетка камней
    let mut stones_img = Mat::default();
    imgproc::cvt_color(img, &mut stones_img, imgproc::COLOR_GRAY2BGR, 0)?;
    let stones_x_step = (img.cols() - settings.stones_left_shift - settings.stones_right_shift)
        as f32
        / board_size as f32;
    let stones_y_step = (img.rows() - settings.stones_top_shift - settings.stones_right_shift)
        as f32
        / board_size as f32;

    let mut grid_img = Mat::default();
    imgproc::cvt_color(img, &mut grid_img, imgproc::COLOR_GRAY2BGR, 0)?;
    let grid_x_step = (img.cols() - settings.grid_left_shift - settings.grid_right_shift) as f32
        / board_size as f32;
    let grid_y_step = (img.rows() - settings.grid_top_shift - settings.grid_bottom_shift) as f32
        / board_size as f32;

    for i in 0..board_size {
        let stones_xp1 = Point::new(
            settings.stones_left_shift + (stones_x_step * i as f32 + stones_x_step / 2.) as i32,
            settings.stones_top_shift,
        );
        let stones_xp2 = Point::new(stones_xp1.x, img.rows() - settings.stones_bottom_shift);
        let stones_yp1 = Point::new(
            settings.stones_left_shift,
            settings.stones_top_shift + (stones_y_step * i as f32 + stones_y_step / 2.) as i32,
        );
        let stones_yp2 = Point::new(img.cols() - settings.stones_right_shift, stones_yp1.y);
        imgproc::line(
            &mut stones_img,
            stones_xp1,
            stones_xp2,
            Scalar::new(0.0, 255.0, 0.0, 0.0), // Зелёный цвет
            1,
            imgproc::LINE_4,
            0,
        )?;
        imgproc::line(
            &mut stones_img,
            stones_yp1,
            stones_yp2,
            Scalar::new(0.0, 255.0, 0.0, 0.0), // Зелёный цвет
            1,
            imgproc::LINE_4,
            0,
        )?;
        let grid_xp1 = Point::new(
            settings.grid_left_shift + (grid_x_step * i as f32 + grid_x_step / 2.) as i32,
            settings.grid_top_shift,
        );
        let grid_xp2 = Point::new(grid_xp1.x, img.rows() - settings.grid_bottom_shift);
        let grid_yp1 = Point::new(
            settings.grid_left_shift,
            settings.grid_top_shift + (grid_y_step * i as f32 + grid_y_step / 2.) as i32,
        );
        let grid_yp2 = Point::new(img.cols() - settings.grid_right_shift, grid_yp1.y);
        imgproc::line(
            &mut grid_img,
            grid_xp1,
            grid_xp2,
            Scalar::new(0.0, 255.0, 0.0, 0.0), // Зелёный цвет
            1,
            imgproc::LINE_4,
            0,
        )?;
        imgproc::line(
            &mut grid_img,
            grid_yp1,
            grid_yp2,
            Scalar::new(0.0, 255.0, 0.0, 0.0), // Зелёный цвет
            1,
            imgproc::LINE_4,
            0,
        )?;
    }
    opencv::imgcodecs::imwrite(
        &(String::from(VISION_DUMP_DIR) + "expected_stones_marks.jpg"),
        &stones_img,
        &core::Vector::default(),
    )?;
    opencv::imgcodecs::imwrite(
        &(String::from(VISION_DUMP_DIR) + "expected_grid_marks.jpg"),
        &grid_img,
        &core::Vector::default(),
    )?;
    Ok(())
}

pub fn find_stones(
    settings: &Settings,
    img: &Mat,
    board_size: usize,
    is_dump_steps: bool,
) -> Result<Vec<(usize, usize)>> {
    let mut blurred = Mat::default();
    imgproc::gaussian_blur_def(&img, &mut blurred, core::Size::new(5, 5), 5.)?;
    //определяем края
    let mut binary = Mat::default();
    imgproc::canny(&blurred, &mut binary, 30., 80., 3, true)?;
    if is_dump_steps {
        opencv::imgcodecs::imwrite(
            &(String::from(VISION_DUMP_DIR) + "binary_stones.jpg"),
            &binary,
            &core::Vector::default(),
        )?;
    }

    //сохраняем сетки которые для камней и разметки
    if is_dump_steps {
        gen_and_write_grids(img, settings, board_size)?;
    }

    //находим прямые линии по вертикали и горизонтали
    let mut lines = Vector::<core::Vec4i>::new();
    let mut line_detector = imgproc::create_line_segment_detector_def()?;
    line_detector.detect_def(&binary, &mut lines)?;

    let mut img_lines = Mat::default();
    if is_dump_steps {
        imgproc::cvt_color(&binary, &mut img_lines, imgproc::COLOR_GRAY2BGR, 0)?;
    }

    let min_line_len = (settings.expected_stone_radius as f32 * 0.2) as i32;
    let grid_x_step = (settings.board_width - settings.grid_left_shift - settings.grid_right_shift)
        as f32
        / board_size as f32;
    let grid_y_step = (settings.board_height - settings.grid_top_shift - settings.grid_bottom_shift)
        as f32
        / board_size as f32;
    for line in lines {
        let p1 = Point2i::new(line[0], line[1]);
        let p2 = Point2i::new(line[2], line[3]);
        let x_diff = (p1.x - p2.x).abs();
        let y_diff = (p1.y - p2.y).abs();

        let line_len = x_diff.max(y_diff);
        if line_len < min_line_len {
            continue;
        }
        // выбираем из всез линий только горизонтальные и вертикальные
        let maximum_shift = (settings.expected_stone_radius as f64 * 0.12) as i32;
        if x_diff > maximum_shift && y_diff > maximum_shift {
            continue;
        }

        let x_mean = (p1.x + p2.x) / 2;
        let y_mean = (p1.y + p2.y) / 2;
        let is_vert = y_diff > x_diff;
        // проверяем что линия близка к нашей сетки. Если это не так, то пропускаем её
        let threshold = settings.expected_stone_radius / 3;
        if is_vert {
            let x = ((x_mean - settings.grid_left_shift) as f32 % grid_x_step) - grid_x_step / 2.;
            if x.abs() as i32 > threshold {
                continue;
            }
        } else {
            let y = ((y_mean - settings.grid_top_shift) as f32 % grid_y_step) - grid_y_step / 2.;
            if y.abs() as i32 > threshold {
                continue;
            }
        }

        if is_dump_steps {
            imgproc::line(
                &mut img_lines,
                p1,
                p2,
                Scalar::new(0.0, 255.0, 0.0, 0.0), // Зелёный цвет
                3,
                imgproc::LINE_4,
                0,
            )?;
        }
        // вычитаем эти линии из нашей картинки
        imgproc::line(
            &mut binary,
            p1,
            p2,
            Scalar::new(0.0, 0.0, 0.0, 0.0),
            3,
            imgproc::LINE_4,
            0,
        )?;
    }
    if is_dump_steps {
        opencv::imgcodecs::imwrite(
            &(String::from(VISION_DUMP_DIR) + "img_lines.jpg"),
            &img_lines,
            &core::Vector::default(),
        )?;
        opencv::imgcodecs::imwrite(
            &(String::from(VISION_DUMP_DIR) + "before_circles.jpg"),
            &binary,
            &core::Vector::default(),
        )?;
    }

    //попытка восстноваить круги и элипсы перед их распознованием
    //    let kernel = imgproc::get_structuring_element(
    //        imgproc::MORPH_ELLIPSE,
    //        Size::new(3, 3),
    //        Point::default(),
    //    )?;
    //    let mut morphology = Mat::default();
    //    imgproc::morphology_ex(
    //        &binary,
    //        &mut morphology,
    //        imgproc::MORPH_CLOSE,
    //        &kernel,
    //        Point::default(),
    //        1,
    //        core::BORDER_CONSTANT,
    //        Scalar::default(),
    //    )?;
    //    if is_dump_steps {
    //        opencv::imgcodecs::imwrite(
    //            &(String::from(VISION_DUMP_DIR) + "after_morphology.jpg"),
    //            &morphology,
    //            &core::Vector::default(),
    //        )?;
    //    }

    // теперь пытаемся расслаблено найти кружки здесь
    let mut circles = Vector::<Point3f>::new();
    imgproc::hough_circles(
        &binary,
        &mut circles,
        imgproc::HOUGH_GRADIENT,
        1.5,
        settings.expected_stone_radius as f64 * 1.6,
        30.,
        30.,
        (settings.expected_stone_radius as f64 * 0.8) as i32,
        (settings.expected_stone_radius as f64 * 1.2) as i32,
    )?;

    let mut img_circles = Mat::default();
    if is_dump_steps {
        imgproc::cvt_color(&img, &mut img_circles, imgproc::COLOR_GRAY2BGR, 0)?;
    }
    let mut result = Vec::<(usize, usize)>::new();
    let x_step = (settings.board_width - settings.stones_left_shift - settings.stones_right_shift)
        as f32
        / board_size as f32;
    let y_step = (settings.board_height - settings.stones_top_shift - settings.stones_bottom_shift)
        as f32
        / board_size as f32;
    for circle in circles {
        let x = (circle.x.max(settings.stones_left_shift as f32)
            - settings.stones_left_shift as f32)
            / x_step;
        let y = (circle.y.max(settings.stones_top_shift as f32) - settings.stones_top_shift as f32)
            / y_step;
        result.push((x as usize, y as usize));
        if is_dump_steps {
            imgproc::circle(
                &mut img_circles,
                Point::new(circle.x as i32, circle.y as i32),
                circle.z as i32,
                Scalar::new(0.0, 255.0, 0.0, 0.0), // Зелёный цвет
                2,
                imgproc::LINE_AA,
                0,
            )?;
        }
    }
    if is_dump_steps {
        opencv::imgcodecs::imwrite(
            &(String::from(VISION_DUMP_DIR) + "img_circles.jpg"),
            &img_circles,
            &core::Vector::default(),
        )?;
    }
    Ok(result)
}

pub fn read_stones(
    settings: &Settings,
    img: &Mat,
    stones: Vec<(usize, usize)>,
    board_size: usize,
    is_dump_steps: bool,
) -> Result<Board> {
    let mut board = Board::new_with_size(board_size);

    let mut debug_img: Option<Mat> = if is_dump_steps {
        Some(img.clone())
    } else {
        None
    };

    let horz_shift = settings.stones_left_shift + settings.stones_right_shift;
    let horz_size = img.cols() - horz_shift as i32;
    let vert_shift = settings.stones_top_shift + settings.stones_bottom_shift;
    let vert_size = img.rows() - vert_shift as i32;
    let horz_step = horz_size as f32 / board_size as f32;
    let vert_step = vert_size as f32 / board_size as f32;

    for (x, y) in stones {
        let radius = settings.stones_read_square_size;
        let center_x = x as f32 * horz_step + horz_step / 2. + settings.stones_left_shift as f32;
        let center_y = y as f32 * vert_step + vert_step / 2. + settings.stones_top_shift as f32;
        let center = core::Point::new(center_x as i32, center_y as i32);
        let value = square_mean(&img, center, radius);

        let pos_y = board_size - y - 1;
        let threshold = u8::MAX / 2;
        if value < threshold {
            board.set(Pos::new(x, pos_y), Cell::black_stone());
        } else {
            board.set(Pos::new(x, pos_y), Cell::white_stone());
        }
        if let Some(image) = &mut debug_img {
            let rect =
                core::Rect::new(center.x - radius, center.y - radius, radius * 2, radius * 2);
            //рисуем прямоуголники в которых считали
            imgproc::rectangle(
                image,
                rect,
                core::Scalar::new(0.0, 0.0, 255.0, 0.0),
                1,
                imgproc::LINE_8,
                0,
            )?;
            // Подписываем значение
            imgproc::put_text(
                image,
                &format!("{value}"),
                core::Point::new(center.x - 20, center.y),
                imgproc::FONT_HERSHEY_SIMPLEX,
                0.4,
                core::Scalar::new(255.0, 0.0, 255.0, 0.0),
                1,
                imgproc::LINE_AA,
                false,
            )?;
        }
    }
    if let Some(image) = &debug_img {
        opencv::imgcodecs::imwrite(
            &(String::from(VISION_DUMP_DIR) + "stones.jpg"),
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
