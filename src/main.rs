mod board;
mod game;
mod katago;
mod speech;
mod vision;

#[derive(Debug)]
enum Error {
    Katago(katago::Error),
    Speech(speech::Error),
    Vision(vision::Error),
    Game(game::Error),
}

type Result<T> = std::result::Result<T, Error>;

impl From<katago::Error> for Error {
    fn from(value: katago::Error) -> Self {
        Error::Katago(value)
    }
}
impl From<speech::Error> for Error {
    fn from(value: speech::Error) -> Self {
        Error::Speech(value)
    }
}
impl From<vision::Error> for Error {
    fn from(value: vision::Error) -> Self {
        Error::Vision(value)
    }
}
impl From<game::Error> for Error {
    fn from(value: game::Error) -> Self {
        Error::Game(value)
    }
}

fn main() -> Result<()> {
    let mut game = game::Game::new();
    let mut katago = katago::Katago::new(katago::Settings::default())?;
    let mut vision = vision::Vision::new(vision::Settings::default())?;
    let mut speech = speech::Speech::new(speech::Settings::default());

    vision.spawn();
    katago.spawn();

    loop {
        let mut nothing_todo = true;
        //обработка подсисетмы зрения
        if let Some(msg) = vision.step() {
            nothing_todo = false;
            match msg {
                vision::Msg::Board(brd) => game.on_human_update_board(brd),
                vision::Msg::Error(e) => return Err(Error::from(e)),
            }
        }
        // обработка подсистемы AI
        if let Some(msg) = katago.step() {
            nothing_todo = false;
            match msg {
                katago::Msg::State(state) => game.on_ai_state_update(state),
                katago::Msg::Move(mv) => game.on_ai_move(mv),
                katago::Msg::Error(e) => return Err(Error::from(e)),
            }
        }
        // обработка подсистемы ведения игры
        if let Some(msg) = game.step() {
            nothing_todo = false;
            match msg {
                game::Msg::WrongStones(ws) => speech.say_for(&ws),
                game::Msg::HumanPlay(color, mv) => {
                    speech.say("Ага.");
                    katago.play(color, mv);
                }
                game::Msg::NeedAiMove(cl) => katago.genmove_for(cl),
                game::Msg::Speech(s) => speech.say(&s),
                game::Msg::Error(e) => return Err(Error::from(e)),
                game::Msg::GameFinished => {
                    speech.say("Спасибо за игру.");
                    break;
                }
            }
        }

        speech.step();
        if nothing_todo {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    Ok(())
}
