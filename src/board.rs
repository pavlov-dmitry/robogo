use std::fmt;
use std::fmt::Display;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Stone {
    Black,
    White,
}

#[derive(PartialEq, Eq)]
pub struct Cell(Option<Stone>);

impl Cell {
    pub fn empty() -> Cell {
        Cell(None)
    }

    pub fn black_stone() -> Cell {
        Cell(Some(Stone::Black))
    }

    pub fn white_stone() -> Cell {
        Cell(Some(Stone::White))
    }
}

#[derive(Clone, Copy)]
pub struct Position {
    x: usize,
    y: usize,
}

impl Position {
    pub fn new(x: usize, y: usize) -> Position {
        Position { x: x, y: y }
    }
}

pub struct Board {
    board: Vec<Cell>,
    size: usize,
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
    pub fn default() -> Board {
        Board::new_with_size(19)
    }

    pub fn pos2idx(&self, pos: Position) -> usize {
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
    pub fn set(&mut self, pos: Position, cell: Cell) {
        let idx = self.pos2idx(pos);
        self.board[idx] = cell;
    }
}

pub enum Action {
    Add(Position, Stone),
    Remove(Position, Stone),
}

pub fn diff(from: &Board, to: &Board) -> Vec<Action> {
    let mut res = Vec::new();
    if from.size != to.size {
        return res;
    }

    for y in 0..from.size {
        for x in 0..from.size {
            let pos = Position::new(x, y);
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

impl Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cell(Some(stone)) => match stone {
                Stone::Black => write!(f, "B"),
                Stone::White => write!(f, "W"),
            },
            Cell(None) => write!(f, "."),
        }
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in 0..self.size {
            write!(f, "{:>2}| ", row + 1)?;
            for col in 0..self.size {
                let idx = self.pos2idx(Position::new(col, row));
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
            let ch = char::from_u32(65 + col as u32).expect("invalid to char conversion");
            write!(f, "{} ", ch)?;
        }
        writeln!(f)?;
        Ok(())
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let letter = char::from_u32(65 + self.x as u32).expect("invalid to char conversion");
        write!(f, "{}{}", letter, self.y + 1)
    }
}

impl Display for Stone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Stone::Black => write!(f, "Black stone"),
            Stone::White => write!(f, "White stone"),
        }
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Add(pos, stone) => write!(f, "Add to {} {}", pos, stone),
            Action::Remove(pos, stone) => write!(f, "Remove from {} {}", pos, stone),
        }
    }
}
