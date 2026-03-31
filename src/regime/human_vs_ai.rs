use super::Error;
use super::game;
use super::katago;
use super::listen;
use super::speech;
use super::vision;

pub fn exec() -> Result<(), Error> {
    let mut game = game::Game::new();
    let mut katago = katago::Katago::new(katago::Settings::default())?;
    let mut vision = vision::Vision::new(vision::Settings::default())?;
    let mut speech = speech::Speech::new(speech::Settings::default());
    let mut listen = listen::Listen::new(listen::VoiceCommandsSettings::default());

    vision.spawn();
    katago.spawn();
    listen.spawn();

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
        if let Some(msg) = listen.step() {
            nothing_todo = false;
            match msg {
                listen::Msg::Text(txt) => println!("Human say: {txt}"),
                listen::Msg::Cmd(cmd) => game.on_voice_cmd(cmd),
                listen::Msg::Err(e) => return Err(Error::from(e)),
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

        // обработка сообщений от подсистемы синтеза речи
        if let Some(msg) = speech.step() {
            match msg {
                speech::Msg::Error(e) => return Err(Error::from(e)),
            }
        }

        if nothing_todo {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    loop {
        println!("Game finished! Press Ctrl+C");
        speech.step();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    //Ok(())
}
