use opencv::Result;
mod vision;

fn main() -> Result<()> {
    let img = vision::load_as_bw_from("/home/deck/development/robogo_tests/0/4.jpg")?;
    let vision_settings = vision::Settings::default();
    let border_polygon = vision::find_board_border(&vision_settings, &img)?;
    let border = border_polygon.expect("Не найдено поле");
    let warped_img = vision::warp_board_by_border(&vision_settings, &border, &img)?;
    vision::find_stones(&vision_settings, &warped_img, 19)?;
    Ok(())
}
