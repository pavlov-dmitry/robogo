use super::board::{self, Action, Board, Move};
use super::katago;

use std::collections::LinkedList;

#[derive(Default)]
pub struct WrongStones {
    invalid: Vec<board::Pos>,
    missing: Vec<board::Pos>,
}

pub enum Msg {
    WrongStones(WrongStones),
    HumanPlay(board::Color, Move),
    NeedAiMove(board::Color),
    Speech(String),
    Error(Error),
    GameFinished,
}

#[derive(Debug)]
pub enum Error {
    UnpredictableAiState,
}

enum State {
    WaitingHumanMove,
    WaitingKatagoAcceptHumanMove,
    InvalidPostitionAfterAiAcceptedMove,
    WaitingAiMove,
    WaitingAiStoneOnBoard,
    Finished,
}

struct BoardDiff {
    human_stones_added: Vec<board::Pos>,
    human_stones_removed: Vec<board::Pos>,
    ai_stones_added: Vec<board::Pos>,
    ai_stones_removed: Vec<board::Pos>,
}

impl BoardDiff {
    fn is_empty(&self) -> bool {
        self.human_stones_added.is_empty()
            && self.human_stones_removed.is_empty()
            && self.ai_stones_added.is_empty()
            && self.ai_stones_removed.is_empty()
    }
}

pub struct Game {
    ai_color: board::Color,
    human_color: board::Color,
    ai_state: katago::State,
    state: State,
    current_board: Board,
    msgs: LinkedList<Msg>,
}

impl Game {
    pub fn new() -> Game {
        Game {
            ai_color: board::Color::White,
            human_color: board::Color::Black,
            ai_state: katago::State::default(),
            state: State::WaitingHumanMove,
            current_board: board::Board::default(),
            msgs: LinkedList::new(),
        }
    }

    pub fn on_human_update_board(&mut self, board: Box<Board>) {
        self.current_board = *board;
        match self.state {
            State::WaitingHumanMove => self.check_human_move(),
            State::WaitingKatagoAcceptHumanMove => {}
            State::InvalidPostitionAfterAiAcceptedMove => {
                if self.check_on_exact_or_msg() {
                    self.send(Msg::NeedAiMove(self.ai_color));
                    self.set_state(State::WaitingAiMove);
                }
            }
            State::WaitingAiMove => {
                let _ = self.check_on_exact_or_msg();
            }
            State::WaitingAiStoneOnBoard => {
                if self.check_on_exact_or_msg() {
                    self.set_state(State::WaitingHumanMove);
                }
            }
            State::Finished => {}
        }
    }
    pub fn on_ai_move(&mut self, mv: Move) {
        if let Move::Resign = mv {
            self.set_state(State::Finished);
            self.send(Msg::GameFinished);
        }
    }
    pub fn on_ai_state_update(&mut self, state: katago::State) {
        self.ai_state = state;
        match self.state {
            State::WaitingHumanMove => self.send(Msg::Error(Error::UnpredictableAiState)),
            State::WaitingKatagoAcceptHumanMove => {
                if self.check_on_exact_or_msg() {
                    self.send(Msg::NeedAiMove(self.ai_color));
                    self.set_state(State::WaitingAiMove);
                } else {
                    self.set_state(State::InvalidPostitionAfterAiAcceptedMove);
                }
            }
            State::InvalidPostitionAfterAiAcceptedMove => {
                self.send(Msg::Error(Error::UnpredictableAiState))
            }
            State::WaitingAiMove => {
                if self.check_on_exact_or_msg() {
                    self.set_state(State::WaitingHumanMove);
                } else {
                    self.set_state(State::WaitingAiStoneOnBoard);
                }
            }
            State::WaitingAiStoneOnBoard => self.send(Msg::Error(Error::UnpredictableAiState)),
            State::Finished => {}
        }
    }
    pub fn step(&mut self) -> Option<Msg> {
        self.msgs.pop_front()
    }

    fn send(&mut self, msg: Msg) {
        self.msgs.push_back(msg);
    }

    fn set_state(&mut self, state: State) {
        self.state = state;
    }

    fn get_board_diff(&self, from: &Board, to: &Board) -> BoardDiff {
        let actions = board::diff(from, to);
        let added_pos = |color: board::Color, actions: &Vec<Action>| {
            actions
                .iter()
                .filter_map(|a| {
                    if let Action::Add(pos, cl) = a {
                        if *cl == color {
                            Some(pos.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<board::Pos>>()
        };

        let removed_pos = |color: board::Color, actions: &Vec<Action>| {
            actions
                .iter()
                .filter_map(|a| {
                    if let Action::Remove(pos, cl) = a {
                        if *cl == color {
                            Some(pos.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<board::Pos>>()
        };
        BoardDiff {
            human_stones_added: added_pos(self.human_color, &actions),
            human_stones_removed: removed_pos(self.human_color, &actions),
            ai_stones_added: added_pos(self.ai_color, &actions),
            ai_stones_removed: removed_pos(self.ai_color, &actions),
        }
    }
    fn check_human_move(&mut self) {
        let board_diff = self.get_board_diff(&self.ai_state.board, &self.current_board);
        if board_diff.human_stones_added.len() == 1
            && board_diff.human_stones_removed.len() == 0
            && board_diff.ai_stones_added.len() == 0
        {
            let pos = board_diff.human_stones_added[0].clone();
            self.send(Msg::HumanPlay(self.human_color, Move::Stone(pos)));
            self.set_state(State::WaitingKatagoAcceptHumanMove);
        } else {
            let mut wrong_stones = WrongStones::default();
            if board_diff.human_stones_added.len() > 1 {
                wrong_stones
                    .invalid
                    .extend(board_diff.human_stones_added.iter());
            }
            wrong_stones
                .invalid
                .extend(board_diff.ai_stones_added.iter());
            wrong_stones
                .missing
                .extend(board_diff.human_stones_removed.iter());
            self.send(Msg::WrongStones(wrong_stones));
        }
    }

    fn check_on_exact_or_msg(&mut self) -> bool {
        let board_diff = self.get_board_diff(&self.ai_state.board, &self.current_board);
        if board_diff.is_empty() {
            true
        } else {
            let mut wrong_stones = WrongStones::default();
            wrong_stones
                .invalid
                .extend(board_diff.human_stones_added.iter());
            wrong_stones
                .invalid
                .extend(board_diff.ai_stones_added.iter());
            wrong_stones
                .missing
                .extend(board_diff.human_stones_removed.iter());
            wrong_stones
                .missing
                .extend(board_diff.ai_stones_removed.iter());
            self.send(Msg::WrongStones(wrong_stones));
            false
        }
    }
}
