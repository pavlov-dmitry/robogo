pub mod arduino_port;
pub mod arm_positioner;
pub mod math;

mod scara_kinematics;

use super::board;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct XY {
    pub x: i32,
    pub y: i32,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct StoneZ {
    pub top_left: i32,
    pub top: i32,
    pub top_right: i32,
    pub left: i32,
    pub center: i32,
    pub right: i32,
    pub bottom_left: i32,
    pub bottom: i32,
    pub bottom_right: i32,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Points {
    pub bottom_left: XY,
    pub top_left: XY,
    pub top_right: XY,
    pub bottom_right: XY,
    pub stone: StoneZ,
    pub move_z: i32,
    pub park: XY,
    pub ai_new_stones: XY,
    pub human_new_stones: XY,
    pub ai_prisoners: XY,
    pub human_prisoners: XY,
    pub ai_bowl: XY,
    pub human_bowl: XY,
    pub bowl_z: i32,
    pub turn: XY,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Ошибка обмена с Arduino")]
    ArduinoPort(#[from] arduino_port::Error),
}

pub fn take_stone(
    pos: board::Pos,
    arm_positioner: &mut arm_positioner::ArmPositioner,
    points: &Points,
    math: &math::Math,
) -> arm_positioner::Result<()> {
    let xy = math.cell_to_xy(pos.x as u8, pos.y as u8);
    arm_positioner
        .move_to(&arm_positioner::Motor::Z, points.move_z)
        .apply_move()?;
    arm_positioner
        .move_to(&arm_positioner::Motor::X, xy.x)
        .move_to(&arm_positioner::Motor::Y, xy.y)
        .apply_move()?;
    arm_positioner.apply_unlock()?;
    let degree = math.calc_turn_for(xy);
    arm_positioner.apply_turn_hand(degree as i16)?;
    println!("{degree}");
    let stone_z = math.calc_stone_z(pos.x as u8, pos.y as u8);
    arm_positioner
        .move_to(&arm_positioner::Motor::Z, stone_z)
        .apply_move()?;
    arm_positioner.apply_lock()?;
    arm_positioner
        .move_to(&arm_positioner::Motor::Z, points.move_z)
        .apply_move()?;
    Ok(())
}

pub fn go_to_stone(
    pos: board::Pos,
    arm_positioner: &mut arm_positioner::ArmPositioner,
    math: &math::Math,
) -> arm_positioner::Result<()> {
    let xy = math.cell_to_xy(pos.x as u8, pos.y as u8);
    arm_positioner
        .move_to(&arm_positioner::Motor::X, xy.x)
        .move_to(&arm_positioner::Motor::Y, xy.y)
        .apply_move()?;
    let degree = math.calc_turn_for(xy);
    arm_positioner.apply_turn_hand(degree as i16)
}

pub fn put_stone(
    pos: board::Pos,
    arm_positioner: &mut arm_positioner::ArmPositioner,
    points: &Points,
    math: &math::Math,
) -> arm_positioner::Result<()> {
    let xy = math.cell_to_xy(pos.x as u8, pos.y as u8);
    arm_positioner
        .move_to(&arm_positioner::Motor::Z, points.move_z)
        .apply_move()?;
    arm_positioner
        .move_to(&arm_positioner::Motor::X, xy.x)
        .move_to(&arm_positioner::Motor::Y, xy.y)
        .apply_move()?;
    let degree = math.calc_turn_for(xy);
    arm_positioner.apply_turn_hand(degree as i16)?;
    let stone_z = math.calc_stone_z(pos.x as u8, pos.y as u8);
    arm_positioner
        .move_to(&arm_positioner::Motor::Z, stone_z)
        .apply_move()?;
    arm_positioner.apply_unlock()?;
    arm_positioner
        .move_to(&arm_positioner::Motor::Z, points.move_z)
        .apply_move()?;
    Ok(())
}

pub fn play(
    mut arm: &mut arm_positioner::ArmPositioner,
    points: &Points,
    math: &math::Math,
) -> arm_positioner::Result<()> {
    arm.apply_zero()?;

    arm.move_to(&arm_positioner::Motor::Z, points.move_z)
        .apply_move()?;

    arm.move_to(&arm_positioner::Motor::X, points.park.x)
        .move_to(&arm_positioner::Motor::Y, points.park.y)
        .apply_move()?;

    take_stone(board::Pos { x: 0, y: 18 }, &mut arm, &points, &math)?;
    put_stone(board::Pos { x: 8, y: 10 }, &mut arm, &points, &math)?;

    take_stone(board::Pos { x: 0, y: 0 }, &mut arm, &points, &math)?;
    put_stone(board::Pos { x: 8, y: 8 }, &mut arm, &points, &math)?;

    take_stone(board::Pos { x: 18, y: 18 }, &mut arm, &points, &math)?;
    put_stone(board::Pos { x: 10, y: 10 }, &mut arm, &points, &math)?;

    take_stone(board::Pos { x: 18, y: 0 }, &mut arm, &points, &math)?;
    put_stone(board::Pos { x: 10, y: 8 }, &mut arm, &points, &math)?;

    take_stone(board::Pos { x: 0, y: 9 }, &mut arm, &points, &math)?;
    put_stone(board::Pos { x: 8, y: 9 }, &mut arm, &points, &math)?;

    take_stone(board::Pos { x: 9, y: 18 }, &mut arm, &points, &math)?;
    put_stone(board::Pos { x: 9, y: 10 }, &mut arm, &points, &math)?;

    take_stone(board::Pos { x: 18, y: 9 }, &mut arm, &points, &math)?;
    put_stone(board::Pos { x: 10, y: 9 }, &mut arm, &points, &math)?;

    take_stone(board::Pos { x: 9, y: 0 }, &mut arm, &points, &math)?;
    put_stone(board::Pos { x: 9, y: 8 }, &mut arm, &points, &math)?;

    take_stone(board::Pos { x: 8, y: 17 }, &mut arm, &points, &math)?;
    put_stone(board::Pos { x: 9, y: 9 }, &mut arm, &points, &math)?;

    arm.move_to(&arm_positioner::Motor::X, points.park.x)
        .move_to(&arm_positioner::Motor::Y, points.park.y)
        .apply_move()?;

    Ok(())
}
