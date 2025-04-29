mod board;
mod vision;

use opencv::Result;

fn main() -> Result<()> {
    let load_board_from = |filename| -> Result<board::Board> {
        let img = vision::load_as_bw_from(filename)?;
        let vision_settings = vision::Settings::default();
        let border_polygon = vision::find_board_border(&vision_settings, &img)?;
        let border = border_polygon.expect("Не найдено поле");
        let warped_img = vision::warp_board_by_border(&vision_settings, &border, &img)?;
        let board = vision::find_stones(&vision_settings, &warped_img, 19)?;
        Ok(board)
    };

    let board1 =
        load_board_from("/home/deck/development/robogo_tests/0/2.jpg").expect("cant load 1");
    let board2 =
        load_board_from("/home/deck/development/robogo_tests/0/6.jpg").expect("cant load 2");

    let actions = board::diff(&board1, &board2);
    println!("From:");
    println!("{}", board1);
    println!("Actions:");
    for a in actions {
        println!("{}", a);
    }
    println!("Result:");
    println!("{}", board2);

    Ok(())
}
