use super::Error;
use super::vision;

pub fn exec() -> Result<(), Error> {
    let mut vision = vision::Vision::new(vision::Settings::default())?;
    vision.spawn();

    loop {
        if let Some(msg) = vision.step() {
            match msg {
                vision::Msg::Board(brd) => println!("{brd}"),
                vision::Msg::Error(e) => return Err(Error::from(e)),
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
