use super::Error;
use super::vision;

pub fn exec(photo: &str, dump_files: Option<bool>) -> Result<(), Error> {
    let is_dump_steps = dump_files.is_none_or(|v| v);
    let settings = vision::Settings::default();
    let board = vision::parse_board_from(photo, &settings, is_dump_steps)?;
    match board {
        Some(brd) => println!("{brd}"),
        None => println!("Доска не найдена."),
    }
    Ok(())
}
