use crate::listen;

use super::board::{self, Action, Board, Move};
use super::katago;
use super::speech::ToSpeech;

use std::collections::LinkedList;
use std::fmt::Display;
use thiserror::Error;

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

#[derive(Error, Debug)]
pub enum Error {
    #[error("Неожиданное обновление состояния ИИ")]
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
    last_update_sended: std::time::SystemTime,
    last_ai_move: Move,
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
            last_update_sended: std::time::SystemTime::now(),
            last_ai_move: Move::Pass,
        }
    }

    pub fn on_human_update_board(&mut self, board: Box<Board>) {
        self.current_board = *board;
        println!("{}", self.current_board);
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
                    self.send(Msg::Speech(String::from("Спасибо.")));
                    self.set_state(State::WaitingHumanMove);
                }
            }
            State::Finished => {}
        }
    }

    pub fn on_ai_move(&mut self, mv: Move) {
        self.last_ai_move = mv.clone();
        println!("ai move: {}", self.last_ai_move);
        self.say_about_last_move();
        if let Move::Resign = mv {
            self.set_state(State::Finished);
            self.send(Msg::Speech(String::from(
                "В этот раз ты победил. Я сдаюсь. Да, да, ты правильно услышал. Я сдаюсь.",
            )));
            self.send(Msg::GameFinished);
        }
    }

    pub fn on_voice_cmd(&mut self, cmd: listen::VoiceCmd) {
        match self.state {
            State::WaitingHumanMove => {
                match cmd {
                    listen::VoiceCmd::Pass => {
                        self.send(Msg::HumanPlay(self.human_color, Move::Pass));
                    }
                    listen::VoiceCmd::Resign => {
                        //self.send(Msg::HumanPlay(self.human_color, Move::Resign));
                        self.set_state(State::Finished);
                        self.send(Msg::Speech(String::from(
                            "Сдался? Так рано? Ну ладно. Правда я сильный?",
                        )));
                        self.send(Msg::GameFinished);
                    }
                    _ => {} // игнорируем другие пока
                }
            }
            _ => {} // обрабатываем голосовые команды только в рожидании хода человека
        }
    }

    fn say_about_last_move(&mut self) {
        if let Move::Stone(pos) = &self.last_ai_move {
            let s = format!("Мой ход {pos}");
            self.send(Msg::Speech(s));
            self.last_update_sended = std::time::SystemTime::now();
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
                self.set_state(State::WaitingAiStoneOnBoard);
            }
            State::WaitingAiStoneOnBoard => self.send(Msg::Error(Error::UnpredictableAiState)),
            State::Finished => {}
        }
    }
    pub fn step(&mut self) -> Option<Msg> {
        if let Ok(elapsed) = self.last_update_sended.elapsed() {
            if elapsed > std::time::Duration::from_secs(10) {
                match self.state {
                    State::WaitingHumanMove => {
                        self.check_human_move();
                        self.last_update_sended = std::time::SystemTime::now();
                    }
                    State::WaitingAiStoneOnBoard => {
                        self.say_about_last_move();
                        self.check_on_ai_move();
                    }
                    State::InvalidPostitionAfterAiAcceptedMove => {
                        self.check_on_exact_or_msg();
                    }
                    _ => {}
                }
            }
        }
        self.msgs.pop_front()
    }

    fn send(&mut self, msg: Msg) {
        self.msgs.push_back(msg);
    }

    fn set_state(&mut self, state: State) {
        self.state = state;
        println!("state: {}", self.state);
    }

    fn get_board_diff(&self, from: &Board, to: &Board) -> BoardDiff {
        let actions = board::diff(from, to);
        for a in &actions {
            println!("{a}");
        }
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
        // если ещё так и не похадил
        if board_diff.is_empty() {
            return;
        }

        // елси добавился только один камень человека(здесь доупстимо убирания камней ai)
        if board_diff.human_stones_added.len() == 1
            && board_diff.human_stones_removed.len() == 0
            && board_diff.ai_stones_added.len() == 0
        {
            let pos = board_diff.human_stones_added[0].clone();
            self.send(Msg::HumanPlay(self.human_color, Move::Stone(pos)));
            self.set_state(State::WaitingKatagoAcceptHumanMove);
        } else {
            // какая-то фигня на доске, сообщим об этом
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
            self.last_update_sended = std::time::SystemTime::now();
        }
    }

    fn check_on_ai_move(&mut self) {
        let board_diff = self.get_board_diff(&self.ai_state.board, &self.current_board);
        // если не хватает только камня ai то ничего не говорим
        if board_diff.ai_stones_removed.len() == 1
            && board_diff.ai_stones_added.is_empty()
            && board_diff.human_stones_added.is_empty()
            && board_diff.human_stones_removed.is_empty()
        {
            if let Move::Stone(pos) = self.last_ai_move {
                if board_diff.ai_stones_removed[0] == pos {
                    return;
                }
            }
        }
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
        self.send(Msg::WrongStones(wrong_stones));
        self.last_update_sended = std::time::SystemTime::now();
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
            self.last_update_sended = std::time::SystemTime::now();
            false
        }
    }
}

impl ToSpeech for WrongStones {
    fn to_speech(&self) -> String {
        let mut result = String::new();
        if !self.invalid.is_empty() {
            if self.invalid.len() == 1 {
                result.push_str("Лишние камень ");
            } else {
                result.push_str("Лишние камни ");
            }
            for pos in &self.invalid {
                result.push_str(format!("{pos} ").as_str());
            }
        }
        if !self.missing.is_empty() {
            if self.missing.len() == 1 {
                result.push_str("Нехватает камня на ");
            } else {
                result.push_str("Нехватвает камней на ");
            }
            for pos in &self.missing {
                result.push_str(format!("{pos} ").as_str());
            }
        }
        result
    }
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::WaitingHumanMove => write!(f, "waiting human move"),
            State::WaitingKatagoAcceptHumanMove => write!(f, "waiting katago accept human move"),
            State::InvalidPostitionAfterAiAcceptedMove => {
                write!(f, "invalid position after ai accepted move")
            }
            State::WaitingAiMove => write!(f, "waiting ai move"),
            State::WaitingAiStoneOnBoard => write!(f, "waiting ai stone on board"),
            State::Finished => write!(f, "finished"),
        }
    }
}
