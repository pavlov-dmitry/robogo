use super::Error;
use super::board;
use super::vision;

use std::str::FromStr;

pub fn exec(tests_dir: &str) -> Result<(), Error> {
    let mut success_count = 0;
    let mut failed_tests = Vec::new();

    for entry in std::fs::read_dir(tests_dir)? {
        let entry = entry?;
        let entry_type = entry.file_type()?;

        // проходимся только директориям
        if entry_type.is_dir() {
            // имя теста это имя директории
            let name = format!("{}", entry.file_name().display());
            println!("Test {}", name);

            let settings = vision::Settings::default();
            let photo_filename = format!("{}/photo.jpg", entry.path().as_os_str().display());
            let board_from_vision = vision::parse_board_from(&photo_filename, &settings, false)?;
            let board_filename = format!("{}/board.txt", entry.path().as_os_str().display());
            let is_board_file_exists = std::fs::exists(&board_filename)?;

            let test_success = match board_from_vision {
                Some(vision_board) => {
                    println!("VISION:\n{}", vision_board);
                    if is_board_file_exists {
                        let board_txt = std::fs::read_to_string(board_filename)?;
                        let board = board::Board::from_str(&board_txt)?;
                        println!("SOURCE BOARD:\n{}", board);
                        let diff = board::diff(&board, &vision_board);
                        if !diff.is_empty() {
                            println!("DIFF: ");
                            for d in &diff {
                                println!("  {d}");
                            }
                        }
                        diff.is_empty()
                    } else {
                        println!("SOURCE BOARD DO NOT EXISTS");
                        false
                    }
                }
                None => {
                    println!("VISION: None");
                    println!("Source board exists: {is_board_file_exists}");
                    !is_board_file_exists
                }
            };

            // подсчитывам количество пройденных и не пройденных тестов
            if test_success {
                success_count += 1;
                println!("Test {} success.", name);
            } else {
                println!("Test {} FAILED!", name);
                failed_tests.push(name);
            }
            println!("---------------------------------------------\n");
        }
    }
    println!(
        "All tests finished. {success_count} success, {} failed.",
        failed_tests.len()
    );
    if !failed_tests.is_empty() {
        println!("failed tests list:");
        for name in failed_tests {
            println!("  {name}");
        }
    }
    Ok(())
}
