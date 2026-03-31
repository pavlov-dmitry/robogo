use super::Error;
use super::vision;

pub fn exec(photo_filename: String) -> Result<(), Error> {
    vision::test_calibration(&photo_filename)?;
    Ok(())
}
