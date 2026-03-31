use super::Error;
use super::listen;
// режим тестирования озвучивания
pub fn exec() -> Result<(), Error> {
    println!("Creating listen...");
    let mut listen = listen::Listen::new(listen::VoiceCommandsSettings::default());
    listen.spawn();
    println!("Listen started. Press Ctrl+C for exit.");
    loop {
        match listen.step() {
            Some(msg) => match msg {
                listen::Msg::Text(txt) => println!("{txt}"),
                listen::Msg::Err(e) => println!("Error {:?}", e),
                listen::Msg::Cmd(cmd) => {
                    println!("Voice Commnd: {:?}", cmd);
                }
            },
            None => std::thread::sleep(std::time::Duration::from_millis(10)),
        }
    }
}
