use opencv::{
    Result, core,
    core::{Point, Point2f, Scalar, Size, Vector},
    imgproc,
    prelude::*,
};

type Polygon = Vector<Point>;

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
    is_dump_steps: bool,
    dump_dir: String,
}

impl Settings {
    pub fn default() -> Settings {
        Settings {
            binary_threshold: 70.,
            min_board_border_perimeter: 2500.,
            board_width: 1000,
            board_height: 1000,
            stones_left_shift: 23.,
            stones_right_shift: 20.,
            stones_top_shift: 17.,
            stones_bottom_shift: 23.,
            stone_radius: 17,
            white_stone_threshold: 190,
            black_stone_threshold: 60,
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

pub fn load_as_bw_from(filename: &str) -> Result<Mat> {
    let img = opencv::imgcodecs::imread(filename, opencv::imgcodecs::IMREAD_COLOR)?;
    if img.empty() {
        panic!("Не удалось загрузить изображение!");
    }

    convert_to_grayscale(&img)
}

// Если и возвращает то это полигон с 4мя точками
pub fn find_board_border(settings: &Settings, gray: &Mat) -> Result<Option<Polygon>> {
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
        let mut polygon: Vector<Point> = Vector::new();
        // апроксимация полигонов
        imgproc::approx_poly_dp(&contour, &mut polygon, 0.05 * perimeter, true)?;
        // поиск четрехугольника
        if polygon.len() == 4 {
            let perimeter = imgproc::arc_length(&polygon, false)?;
            // с самым большим периметром
            if perimeter > settings.min_board_border_perimeter && perimeter > best_perimeter {
                best_perimeter = perimeter;
                best_polygon = Some(polygon);
            }
        }
    }

    if settings.is_dump_steps {
        let mut img = Mat::default();
        imgproc::cvt_color(&gray, &mut img, imgproc::COLOR_GRAY2BGR, 0)?;
        if let Some(poly) = &best_polygon {
            // Рисуем контуры на исходном изображении
            let mut polygon_for_draw: Vector<Vector<Point>> = Vector::new();
            polygon_for_draw.push(poly.clone());
            imgproc::draw_contours(
                &mut img,
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
            &img,
            &core::Vector::default(),
        )?;
    }
    Ok(best_polygon)
}

pub fn warp_board_by_border(settings: &Settings, border: &Polygon, img: &Mat) -> Result<Mat> {
    let poly_f: Vector<Point2f> = border
        .iter()
        .map(|p| Point2f::new(p.x as f32, p.y as f32))
        .collect();
    let dst_poly = Vector::from_slice(&[
        Point2f::new(0., 0.),
        Point2f::new(settings.board_width as f32, 0.),
        Point2f::new(settings.board_width as f32, settings.board_height as f32),
        Point2f::new(0., settings.board_height as f32),
    ]);

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

pub fn find_stones(settings: &Settings, img: &Mat, board_size: i32) -> Result<()> {
    // Создаём маску для круглой области
    let mut mask = Mat::zeros(img.rows(), img.cols(), core::CV_8UC1)?.to_mat()?;

    let mut debug_img: Option<Mat> = if settings.is_dump_steps {
        let mut image = Mat::default();
        imgproc::cvt_color(&img, &mut image, imgproc::COLOR_GRAY2BGR, 0)?;
        Some(image)
    } else {
        None
    };

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
            let mean = core::mean(&img, &mask)?;
            let dominant_gray = mean[0] as u8;

            if dominant_gray < settings.black_stone_threshold {
                //black stone
            } else if dominant_gray > settings.white_stone_threshold {
                //white stone
            } else {
                //no stone
            }

            if let Some(image) = &mut debug_img {
                //рисуем кружочки
                imgproc::circle(
                    image,
                    center,
                    radius,
                    core::Scalar::new(0.0, 0.0, 255.0, 0.0), // Красная рамка
                    1,
                    imgproc::LINE_8,
                    0,
                )?;
                // Подписываем значение
                imgproc::put_text(
                    image,
                    &format!("{}", dominant_gray),
                    core::Point::new(center.x - 10, center.y),
                    imgproc::FONT_HERSHEY_SIMPLEX,
                    0.4,
                    core::Scalar::new(0.0, 0.0, 255.0, 0.0),
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
    Ok(())
}
