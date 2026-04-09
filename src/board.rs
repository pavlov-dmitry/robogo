use std::fmt;
use std::fmt::Display;
use std::str::FromStr;
use thiserror::Error;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Color {
    Black,
    White,
}

#[derive(PartialEq, Eq, Clone)]
pub struct Cell(Option<Color>);

impl Cell {
    pub fn empty() -> Cell {
        Cell(None)
    }

    pub fn black_stone() -> Cell {
        Cell(Some(Color::Black))
    }

    pub fn white_stone() -> Cell {
        Cell(Some(Color::White))
    }
}

impl From<Color> for Cell {
    fn from(color: Color) -> Self {
        Cell(Some(color))
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Pos {
    x: usize,
    y: usize,
}

impl Pos {
    pub fn new(x: usize, y: usize) -> Pos {
        Pos { x: x, y: y }
    }
}
#[derive(Clone)]
pub enum Move {
    Stone(Pos),
    Pass,
    Resign,
}

#[derive(Error, Debug)]
#[error("ошибка парсинга позиции на доске")]
pub struct ParsePositionError;

static Y_LETTERS: &str = "ABCDEFGHJKLMNOPQRST";

impl FromStr for Pos {
    type Err = ParsePositionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > 1 {
            let char = s.chars().nth(0).expect("WTF: len() > 1");
            let found = Y_LETTERS.chars().enumerate().find(|(_, c)| *c == char);
            if let Some((pos, _)) = found {
                let x = pos;
                let num_str = s.get(1..).ok_or_else(|| ParsePositionError)?;
                let y = num_str.parse::<i32>().map_err(|_| ParsePositionError)?;
                if let Ok(y) = usize::try_from(y - 1) {
                    return Ok(Pos { x, y });
                }
            }
        }
        Err(ParsePositionError)
    }
}

#[derive(Clone, PartialEq)]
pub struct Board {
    board: Vec<Cell>,
    size: usize,
}

impl Default for Board {
    fn default() -> Self {
        Board::new_with_size(19)
    }
}

impl Board {
    pub fn new_with_size(size: usize) -> Board {
        let mut res = Board {
            board: Vec::new(),
            size: size,
        };
        res.board.resize_with(size * size, Cell::empty);
        return res;
    }

    fn pos2idx(&self, pos: Pos) -> usize {
        if pos.x >= self.size {
            panic!(
                "invalid x position where board size={} and x={}",
                self.size, pos.x
            );
        }
        if pos.y >= self.size {
            panic!(
                "invalid y position where board size={} and y={}",
                self.size, pos.y
            );
        }
        pos.y * self.size + pos.x
    }
    pub fn set(&mut self, pos: Pos, cell: Cell) {
        let idx = self.pos2idx(pos);
        self.board[idx] = cell;
    }
}

pub enum Action {
    Add(Pos, Color),
    Remove(Pos, Color),
}

pub fn diff(from: &Board, to: &Board) -> Vec<Action> {
    let mut res = Vec::new();
    if from.size != to.size {
        return res;
    }

    for y in 0..from.size {
        for x in 0..from.size {
            let pos = Pos::new(x, y);
            let idx = from.pos2idx(pos);
            let Cell(f) = &from.board[idx];
            let Cell(t) = &to.board[idx];
            if f != t {
                if let Some(stone) = f {
                    res.push(Action::Remove(pos, stone.clone()));
                }
                if let Some(stone) = t {
                    res.push(Action::Add(pos, stone.clone()));
                }
            }
        }
    }
    res
}

pub fn has_diff(from: &Board, to: &Board) -> bool {
    if from.size != to.size {
        return true;
    }
    !diff(from, to).is_empty()
}

impl Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cell(Some(stone)) => match stone {
                Color::Black => write!(f, "B"),
                Color::White => write!(f, "W"),
            },
            Cell(None) => write!(f, "."),
        }
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in (0..self.size).rev() {
            write!(f, "{:>2}| ", row + 1)?;
            for col in 0..self.size {
                let idx = self.pos2idx(Pos::new(col, row));
                write!(f, "{} ", self.board[idx])?;
            }
            writeln!(f)?;
        }
        write!(f, "    ")?;
        for _ in 0..self.size {
            write!(f, "__")?;
        }
        writeln!(f)?;
        write!(f, "    ")?;
        for col in 0..self.size {
            let ch = Y_LETTERS.chars().nth(col).expect("invalid to column by Y");
            write!(f, "{} ", ch)?;
        }
        writeln!(f)?;
        Ok(())
    }
}

impl Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let letter = Y_LETTERS
            .chars()
            .nth(self.x)
            .expect("Invalid X, more than expected");
        write!(f, "{}{}", letter, self.y + 1)
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Move::Pass => write!(f, "pass"),
            Move::Resign => write!(f, "resign"),
            Move::Stone(pos) => write!(f, "{pos}"),
        }
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Color::Black => write!(f, "Black"),
            Color::White => write!(f, "White"),
        }
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Add(pos, stone) => write!(f, "Add to {} {} stone", pos, stone),
            Action::Remove(pos, stone) => write!(f, "Remove from {} {} stone", pos, stone),
        }
    }
}

#[derive(Error, Debug)]
#[error("Ошибка парсинга текстового представления поля")]
pub struct BoardParseError;

impl FromStr for Board {
    type Err = BoardParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let board_size = s
            .get(..2)
            .ok_or(BoardParseError)?
            .trim()
            .parse::<usize>()
            .map_err(|_| BoardParseError)?;

        let mut board = Board::new_with_size(board_size);
        let mut parse_line = |line: &str, y: usize| {
            for i in 0..board_size {
                let idx = 4 + i * 2;
                match line.get(idx..idx + 1) {
                    Some(ch) => {
                        let cell = if ch == "B" {
                            Cell::black_stone()
                        } else if ch == "W" {
                            Cell::white_stone()
                        } else {
                            Cell::empty()
                        };
                        board.set(Pos::new(i, y), cell);
                    }
                    None => return Err(BoardParseError),
                }
            }
            Ok(())
        };
        for line_idx in 0..board_size {
            match s.lines().nth(line_idx) {
                Some(line) => {
                    parse_line(line, board_size - line_idx - 1)?;
                }
                None => return Err(BoardParseError),
            }
        }
        Ok(board)
    }
}
