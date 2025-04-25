use opencv::{
    Result, core,
    core::{Point, Point2f, Scalar, Size, Vector},
    highgui, imgproc,
    prelude::*,
};

fn main() -> Result<()> {
    // Загрузка изображения
    let img = opencv::imgcodecs::imread(
        "/home/deck/development/robogo_tests/0/6.jpg",
        opencv::imgcodecs::IMREAD_COLOR,
    )?;
    if img.empty() {
        panic!("Не удалось загрузить изображение!");
    }

    // Конвертация в grayscale
    let mut gray = Mat::default();
    imgproc::cvt_color(&img, &mut gray, imgproc::COLOR_BGR2GRAY, 0)?;
    opencv::imgcodecs::imwrite("gray.jpg", &gray, &core::Vector::default())?;

    // Бинаризация (чёрно-белое изображение)
    let mut binary = Mat::default();
    imgproc::threshold(&gray, &mut binary, 70.0, 255.0, imgproc::THRESH_BINARY_INV)?;
    opencv::imgcodecs::imwrite("binary.jpg", &binary, &core::Vector::default())?;

    // Поиск контуров
    let mut contours: Vector<Vector<Point>> = Vector::new();
    imgproc::find_contours(
        &binary,
        &mut contours,
        imgproc::RETR_EXTERNAL,
        imgproc::CHAIN_APPROX_SIMPLE,
        core::Point::default(),
    )?;

    let mut result = img.clone();

    let mut best_area = std::f64::MIN;
    let mut best_polygon: Option<Vector<Point>> = Option::None;
    for contour in contours {
        let perimeter = imgproc::arc_length(&contour, true)?;
        let mut polygon: Vector<Point> = Vector::new();
        imgproc::approx_poly_dp(&contour, &mut polygon, 0.05 * perimeter, true)?;
        if polygon.len() == 4 {
            let area = imgproc::contour_area(&polygon, false)?;
            if area > 2000. && area > best_area {
                best_area = area;
                best_polygon = Some(polygon);
            }
        }
    }

    let poly = best_polygon.expect("Не найден контурный прямоуголник");
    println!("poly: {:?}", poly);
    let poly_f: Vector<Point2f> = poly
        .iter()
        .map(|p| Point2f::new(p.x as f32, p.y as f32))
        .collect();
    println!("polyf: {:?}", poly_f);

    let width = 1000;
    let height = 1000;
    let dst_poly = Vector::from_slice(&[
        Point2f::new(0., 0.),
        Point2f::new(width as f32, 0.),
        Point2f::new(width as f32, height as f32),
        Point2f::new(0., height as f32),
    ]);
    let transform_matrix = imgproc::get_perspective_transform(&poly_f, &dst_poly, core::DECOMP_LU)?;
    let mut warped_image = Mat::default();
    imgproc::warp_perspective(
        &img,
        &mut warped_image,
        &transform_matrix,
        Size::new(width, height),
        imgproc::INTER_LINEAR,
        core::BORDER_CONSTANT,
        Scalar::default(),
    )?;

    // // Рисуем контуры на исходном изображении
    // let mut polygon_for_draw: Vector<Vector<Point>> = Vector::new();
    // polygon_for_draw.push(poly.clone());
    // imgproc::draw_contours(
    //     &mut result,
    //     &polygon_for_draw,
    //     -1,                                // Индекс контура (-1 = все контуры)
    //     Scalar::new(0.0, 255.0, 0.0, 0.0), // Зелёный цвет
    //     2,                                 // Толщина линии
    //     imgproc::LINE_8,
    //     &Mat::default(),
    //     std::i32::MAX,
    //     Point::default(),
    // )?;

    // // Сохраняем результат
    // opencv::imgcodecs::imwrite("output.jpg", &result, &Vector::default())?;

    // Показываем результат (опционально)
    highgui::named_window("Contours", highgui::WINDOW_NORMAL)?;
    highgui::imshow("Contours", &warped_image)?;
    highgui::wait_key(0)?;

    Ok(())
}
