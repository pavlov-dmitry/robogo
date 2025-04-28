use std::fmt;
use std::fmt::Display;

enum Stone {
    Black,
    White,
}

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
