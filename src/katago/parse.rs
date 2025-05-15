use super::Color;
use super::{Error, Result};

pub fn move_num(line: &str) -> Result<u32> {
    if line.starts_with("= MoveNum: ") {
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.len() < 3 {
            return Err(Error::InvalidTextProtocol);
        }
        let count = words[2].parse::<u32>()?;
        return Ok(count);
    }
    Err(Error::InvalidTextProtocol)
}

pub fn board_line<F>(line: &str, board_size: usize, mut put_stone: F) -> Result<()>
where
    F: FnMut(usize, Color),
{
    for i in 0..board_size {
        let idx = 3 + i * 2;
        match line.get(idx..idx) {
            Some(ch) => {
                if ch == "X" {
                    put_stone(i, Color::Black);
                } else if ch == "O" {
                    put_stone(i, Color::White);
                }
            }
            None => return Err(Error::InvalidTextProtocol),
        }
    }
    Ok(())
}

pub fn next_move(line: &str) -> Result<Color> {
    if line.starts_with("Next player:") {
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.len() < 3 {
            return Err(Error::InvalidTextProtocol);
        }
        if words[2] == "Black" {
            return Ok(Color::Black);
        }
        if words[2] == "White" {
            return Ok(Color::White);
        }
    }
    Err(Error::InvalidTextProtocol)
}

fn stones_captured(line: &str, prefix: &str) -> Result<u32> {
    let prefix_str = format!("{} stones captured:", prefix);
    if line.starts_with(&prefix_str) {
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.len() < 4 {
            return Err(Error::InvalidTextProtocol);
        }
        let count = words[3].parse::<u32>()?;
        return Ok(count);
    }
    Err(Error::InvalidTextProtocol)
}

pub fn black_captured(line: &str) -> Result<u32> {
    stones_captured(line, "B")
}

pub fn white_captured(line: &str) -> Result<u32> {
    stones_captured(line, "W")
}
