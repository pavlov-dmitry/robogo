use super::Points;
use super::XY;
use super::scara_kinematics::ScaraKinematics;

use std::f64::consts::PI;

// точка на миллиметровой сетки плоскости перед роботом
#[derive(Debug, Default, Clone, Copy)]
struct XYPoint {
    x: f64,
    y: f64,
}
pub struct Math {
    kinematics: ScaraKinematics,
    points: Points,
    top_left: XYPoint,
    top_right: XYPoint,
    bottom_left: XYPoint,
    bottom_right: XYPoint,
    board_size: u8,
}

static MOTOR_DEGREE_IN_STEP: f64 = 1.8;
static MULTYSTEPPING: f64 = 1. / 8.;
static REDUCTOR_RATIO: f64 = 12. / 72.;
static DEGREE_IN_STEP: f64 = MOTOR_DEGREE_IN_STEP * MULTYSTEPPING * REDUCTOR_RATIO;

static Y_ANGLE_OFFSET_DEGREE: f64 = 9.0;

impl Math {
    pub fn new(pnts: &Points, board_size: u8) -> Self {
        let kinematics = ScaraKinematics::new(320., 274.591);
        let xy_to_mm = |pnt: XY| {
            let (x, y) = kinematics.forward(
                steps_to_rad(pnt.x),
                steps_to_rad(pnt.y) + degree_to_rad(Y_ANGLE_OFFSET_DEGREE),
            );
            XYPoint { x, y }
        };
        let tl = xy_to_mm(pnts.top_left);
        let tr = xy_to_mm(pnts.top_right);
        let bl = xy_to_mm(pnts.bottom_left);
        let br = xy_to_mm(pnts.bottom_right);

        let calc_distance = |a: XYPoint, b: XYPoint| {
            let width = b.x - a.x;
            let height = b.y - a.y;
            (width * width + height * height).sqrt()
        };
        let tltr = calc_distance(tl, tr);
        println!("tltr {tltr}");
        let blbr = calc_distance(bl, br);
        println!("blbr: {blbr}");

        let bltl = calc_distance(bl, tl);
        println!("bltl: {bltl}");

        let brtr = calc_distance(br, tr);
        println!("brtr: {brtr}");

        Math {
            kinematics: kinematics,
            points: pnts.clone(),
            top_left: tl,
            top_right: tr,
            bottom_left: bl,
            bottom_right: br,
            board_size: board_size,
        }
    }

    pub fn cell_to_xy(&self, x: u8, y: u8) -> XY {
        let x_coeff: f64 = x as f64 / (self.board_size - 1) as f64;
        let y_from = get_point_by_coeff(&self.bottom_left, &self.bottom_right, x_coeff);
        let y_to = get_point_by_coeff(&self.top_left, &self.top_right, x_coeff);

        let y_coeff: f64 = y as f64 / (self.board_size - 1) as f64;
        let pnt = get_point_by_coeff(&y_from, &y_to, y_coeff);
        let mut degrees = self
            .kinematics
            .inverse(pnt.x, pnt.y)
            .expect("invalid kinematics math");

        if degrees.theta2 > PI {
            degrees.theta2 -= 2. * PI;
        }

        let x_steps = rad_to_steps(degrees.theta1);
        let y_steps = rad_to_steps(degrees.theta2 - degree_to_rad(Y_ANGLE_OFFSET_DEGREE));
        XY {
            x: x_steps,
            y: y_steps,
        }
    }

    pub fn calc_turn_for(&self, pos: XY) -> f64 {
        let diff_steps = pos.y - self.points.turn.y;
        let diff_degree = diff_steps as f64 * DEGREE_IN_STEP;
        let angle = diff_degree % 90.;
        let angle = if angle < 0. { angle + 90. } else { angle };
        let angle = if angle > 45. { angle - 90. } else { angle };
        angle
    }

    pub fn calc_stone_z(&self, board_x: u8, board_y: u8) -> i32 {
        let half_board_size = self.board_size / 2;
        let stones = &self.points.stone;
        let (x, y, top_left, top_right, bottom_left, bottom_right) = if board_x < half_board_size {
            if board_y < half_board_size {
                (
                    board_x,
                    board_y,
                    stones.left,
                    stones.center,
                    stones.bottom_left,
                    stones.bottom,
                )
            } else {
                (
                    board_x,
                    board_y - half_board_size,
                    stones.top_left,
                    stones.top,
                    stones.left,
                    stones.center,
                )
            }
        } else {
            if board_y < half_board_size {
                (
                    board_x - half_board_size,
                    board_y,
                    stones.center,
                    stones.right,
                    stones.bottom,
                    stones.bottom_right,
                )
            } else {
                (
                    board_x - half_board_size,
                    board_y - half_board_size,
                    stones.top,
                    stones.top_right,
                    stones.center,
                    stones.right,
                )
            }
        };
        let x_coeff = x as f64 / half_board_size as f64;
        let top = get_value_by_coeff(top_left, top_right, x_coeff);
        let bottom = get_value_by_coeff(bottom_left, bottom_right, x_coeff);
        let y_coeff = y as f64 / half_board_size as f64;
        get_value_by_coeff(bottom, top, y_coeff)
    }
}

fn get_value_by_coeff(from: i32, to: i32, coeff: f64) -> i32 {
    let len = to - from;
    from + (len as f64 * coeff) as i32
}

fn get_point_by_coeff(from: &XYPoint, to: &XYPoint, coeff: f64) -> XYPoint {
    let x_length = to.x - from.x;
    let y_length = to.y - from.y;
    XYPoint {
        x: from.x + x_length * coeff,
        y: from.y + y_length * coeff,
    }
}

/// Преобразование радиан в градусы
pub fn rad_to_degree(rad: f64) -> f64 {
    rad * 180.0 / PI
}

/// Преобразование градусов в радианы
pub fn degree_to_rad(deg: f64) -> f64 {
    deg * PI / 180.0
}

pub fn steps_to_degree(steps: i32) -> f64 {
    steps as f64 * DEGREE_IN_STEP
}

pub fn degree_to_steps(degree: f64) -> i32 {
    (degree / DEGREE_IN_STEP) as i32
}

pub fn steps_to_rad(steps: i32) -> f64 {
    degree_to_rad(steps_to_degree(steps))
}

pub fn rad_to_steps(rad: f64) -> i32 {
    degree_to_steps(rad_to_degree(rad))
}
