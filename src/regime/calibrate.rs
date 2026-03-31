use super::Error;
use super::vision;

pub fn exec(photo_dir: String, count: u32) -> Result<(), Error> {
    vision::calibrate_by(&photo_dir, count)?;
    Ok(())
}
