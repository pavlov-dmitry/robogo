use super::Error;
use super::vision;

pub fn exec() -> Result<(), Error> {
    vision::camera_mode()?;
    Ok(())
}
