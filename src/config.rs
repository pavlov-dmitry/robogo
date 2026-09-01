// Отвечает за сохранение и загрузку конфигов других модулей
use super::arm::Points;
use super::arm::arm_positioner;
use serde::Serialize;
use std::fs::{self, File};

pub struct Config {
    motor_speeds: arm_positioner::SpeedSettings,
    points: Points,
}

pub type Result<T> = std::io::Result<T>;

static MOTOR_SPEEDS_FILENAME: &str = "./cfg/motor_speeds.json";
static POINTS_FILENAME: &str = "./cfg/points.json";

fn write_to_file<T: Serialize>(filename: &str, val: &T) -> Result<()> {
    let mut file = File::create(filename)?;
    serde_json::to_writer_pretty(&mut file, &val)?;
    Ok(())
}

impl Config {
    pub fn new() -> Self {
        let motor_speed_file = fs::read_to_string(MOTOR_SPEEDS_FILENAME).unwrap_or(String::new());
        let points_file = fs::read_to_string(POINTS_FILENAME).unwrap_or(String::new());

        Config {
            motor_speeds: serde_json::from_str(&motor_speed_file)
                .unwrap_or(arm_positioner::SpeedSettings::default()),
            points: serde_json::from_str(&points_file).unwrap_or(Points::default()),
        }
    }

    pub fn get_motor_speeds(&self) -> arm_positioner::SpeedSettings {
        self.motor_speeds.clone()
    }

    pub fn save_motor_speeds(&mut self, speeds: &arm_positioner::SpeedSettings) -> Result<()> {
        self.motor_speeds = speeds.clone();
        write_to_file(MOTOR_SPEEDS_FILENAME, &self.motor_speeds)?;
        Ok(())
    }

    pub fn get_points(&self) -> Points {
        self.points.clone()
    }

    pub fn save_points(&mut self, points: &Points) -> Result<()> {
        self.points = points.clone();
        write_to_file(POINTS_FILENAME, &self.points)?;
        Ok(())
    }
}
