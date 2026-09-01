use serde::{Deserialize, Serialize};
use std::fmt::Display;

use super::arduino_port::{self, ArduinoPort};

pub type Result<T> = arduino_port::Result<T>;
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct SpeedSettings {
    pub x_speed: u32,
    pub x_max_speed: u32,
    pub x_acceleration: u32,

    pub y_speed: u32,
    pub y_max_speed: u32,
    pub y_acceleration: u32,

    pub z_speed: u32,
    pub z_max_speed: u32,
    pub z_acceleration: u32,
}

#[derive(Default, Clone)]
pub struct MotorState {
    pub pos: i32,
}
#[derive(Default, Clone)]
pub struct MotorsState {
    pub x: MotorState,
    pub y: MotorState,
    pub z: MotorState,
}

pub struct ArmPositioner {
    motors_state: MotorsState,
    arduino_port: ArduinoPort,
    next_motor_state: MotorsState,
    current_cmd: MoveCmd,
}

pub enum Motor {
    X,
    Y,
    Z,
}

impl ArmPositioner {
    pub fn new(portname: &str, timeout_secs: u64) -> Result<ArmPositioner> {
        Ok(ArmPositioner {
            motors_state: MotorsState::default(),
            arduino_port: ArduinoPort::new(portname, timeout_secs)?,
            next_motor_state: MotorsState::default(),
            current_cmd: MoveCmd::default(),
        })
    }

    fn get_cmd(&mut self, motor: &Motor) -> &mut MotorCmd {
        match motor {
            Motor::X => &mut self.current_cmd.x,
            Motor::Y => &mut self.current_cmd.y,
            Motor::Z => &mut self.current_cmd.z,
        }
    }

    // перемещает мотор на шаги относительно текущей позиции
    pub fn move_steps(&mut self, motor: &Motor, steps: i32) -> &mut Self {
        let current_pos = self.get_state(&motor).pos;
        let next_state = self.get_mut_next_state(&motor);
        next_state.pos = current_pos + steps;

        let cmd = self.get_cmd(motor);
        cmd.pos = Some(steps);
        self
    }

    // перемещает мотор в опередёлнную глобоальную позицию
    pub fn move_to(&mut self, motor: &Motor, pos: i32) -> &mut Self {
        let current_pos = self.get_state(motor).pos;
        let next_state = self.get_mut_next_state(motor);
        next_state.pos = pos;
        let steps = next_state.pos - current_pos;

        let cmd = self.get_cmd(motor);
        cmd.pos = Some(steps);
        self
    }

    pub fn set_start_speed(&mut self, motor: &Motor, speed: u32) -> &mut Self {
        let cmd = self.get_cmd(motor);
        cmd.start_speed = Some(speed);
        self
    }

    pub fn set_max_speed(&mut self, motor: &Motor, speed: u32) -> &mut Self {
        let cmd = self.get_cmd(motor);
        cmd.max_speed = Some(speed);
        self
    }

    pub fn set_acceleration(&mut self, motor: &Motor, acceleration: u32) -> &mut Self {
        let cmd = self.get_cmd(motor);
        cmd.acceleration = Some(acceleration);
        self
    }

    pub fn apply_move(&mut self) -> Result<()> {
        let cmd_str = self.current_cmd.to_string();
        self.arduino_port.send_cmd(&cmd_str)?;
        self.current_cmd = MoveCmd::default();
        self.motors_state = self.next_motor_state.clone();
        Ok(())
    }

    pub fn get_motors_state(&self) -> MotorsState {
        self.motors_state.clone()
    }

    pub fn apply_lock(&mut self) -> Result<()> {
        self.arduino_port.send_cmd("lock")?;
        Ok(())
    }

    pub fn apply_unlock(&mut self) -> Result<()> {
        self.arduino_port.send_cmd("unlock")?;
        Ok(())
    }

    pub fn apply_turn_hand(&mut self, degree: i16) -> Result<()> {
        self.arduino_port.send_cmd(&format!("turn {degree}"))?;
        Ok(())
    }

    pub fn apply_zero(&mut self) -> Result<()> {
        self.arduino_port.send_cmd("zero")?;
        self.motors_state = MotorsState::default();
        self.next_motor_state = MotorsState::default();
        Ok(())
    }

    fn get_state(&self, motor: &Motor) -> &MotorState {
        match motor {
            Motor::X => &self.motors_state.x,
            Motor::Y => &self.motors_state.y,
            Motor::Z => &self.motors_state.z,
        }
    }

    fn get_mut_next_state(&mut self, motor: &Motor) -> &mut MotorState {
        match motor {
            Motor::X => &mut self.next_motor_state.x,
            Motor::Y => &mut self.next_motor_state.y,
            Motor::Z => &mut self.next_motor_state.z,
        }
    }
}

#[derive(Default)]
struct MotorCmd {
    pos: Option<i32>, // передвинуть мотор на количество шагов, если отрицательное то в обратную сторону
    start_speed: Option<u32>, //стартовая скорость мотора
    max_speed: Option<u32>, //масимальная сокрость мотора
    acceleration: Option<u32>, // ускорение набора скорости
}

fn make_string_cmds(cmds: &MotorCmd, motor: Motor) -> String {
    let mut result = String::new();
    if let Some(pos) = cmds.pos {
        result += &format!("{motor}{pos} ");
    }
    if let Some(start_speed) = cmds.start_speed {
        result += &format!("{motor}ss{start_speed} ");
    }
    if let Some(max_speed) = cmds.max_speed {
        result += &format!("{motor}ms{max_speed} ");
    }
    if let Some(acceleration) = cmds.acceleration {
        result += &format!("{motor}a{acceleration} ");
    }
    result
}

#[derive(Default)]
struct MoveCmd {
    x: MotorCmd,
    y: MotorCmd,
    z: MotorCmd,
}

impl ToString for MoveCmd {
    fn to_string(&self) -> String {
        let mut result = String::new();
        result += &make_string_cmds(&self.x, Motor::X);
        result += &make_string_cmds(&self.y, Motor::Y);
        result += &make_string_cmds(&self.z, Motor::Z);
        result
    }
}

impl Display for Motor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Motor::X => write!(f, "X"),
            Motor::Y => write!(f, "Y"),
            Motor::Z => write!(f, "Z"),
        }
    }
}
