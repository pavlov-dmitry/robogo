use std::str::FromStr;

use crate::arm::go_to_stone;
use crate::arm::play;
use crate::board;

use super::Error;
use super::arm;
use super::arm::XY;
use super::arm::arduino_port::ArduinoPort;
use super::arm::arm_positioner::{ArmPositioner, Motor, MotorsState};
use super::config::Config;
use clap::{Parser, ValueEnum};
use clap_repl::ClapEditor;

use serialport::{self, SerialPortType};

type Result = std::result::Result<(), Error>;

pub fn exec(portname: Option<String>) -> Result {
    match portname {
        Some(portname) => port_process(portname),
        None => show_aviableports(),
    }
}

fn show_aviableports() -> Result {
    let mut config = Config::new();
    let _ = config.save_motor_speeds(&config.get_motor_speeds());

    println!("Aviable ports:");
    let infos = ArduinoPort::available_ports()?;
    for info in infos {
        print!("name: {}", info.port_name);
        print!(" type: ");
        match info.port_type {
            SerialPortType::UsbPort(usb) => {
                print!("USB ");
                if let Some(product) = usb.product {
                    print!(" product: {product}");
                }
                if let Some(manufacturer) = usb.manufacturer {
                    print!(" manufacturer: {manufacturer}");
                }
            }
            SerialPortType::BluetoothPort => {
                print!("BLUETOOTH");
            }
            SerialPortType::PciPort => {
                print!("PCI");
            }
            SerialPortType::Unknown => {
                print!("UNKNOWN");
            }
        }
        println!("");
    }
    Ok(())
}

#[derive(Debug, Clone, ValueEnum)]
enum StoneNames {
    /// верхний левый угол доски относительно робота
    TopLeft,
    /// верхняя точка доски относительно робота
    Top,
    /// верхний правый угол доски относительно робота
    TopRight,
    /// левая точка доски относительно робота
    Left,
    /// централья точка доски относительно робота
    Center,
    /// правая точка доски относительно робота
    Right,
    /// нижний левый угол доски относительно робота
    BottomLeft,
    /// нижняя точка по центру доски относительно робота
    Bottom,
    /// нижний правый угол доски относительно робота
    BottomRight,
}

#[derive(Debug, Clone, ValueEnum)]
enum PointNames {
    /// верхний левый угол доски относительно робота
    TopLeft,
    /// верхний правый угол доски относительно робота
    TopRight,
    /// нижний левый угол доски относительно робота
    BottomLeft,
    /// нижний правый угол доски относительно робота
    BottomRight,
    /// высота на которой роботу перемещаться над камнями
    Move,
    /// высота на которой находятся камни в левом верхнем углу
    StoneTopLeft,
    /// высота на которой находятся камни в верху доски
    StoneTop,
    /// высота на которой находятся камни в верхнем правом углу
    StoneTopRight,
    /// высота на которой находятся камни с левой стороны
    StoneLeft,
    /// высота на которой находятся камни по центру
    StoneCenter,
    /// высота на которой находятся камни с правой стороны
    StoneRight,
    /// высота на которой находятся камни в нижнем левом углу
    StoneBottomLeft,
    /// высота на которой находятся камни в низу доски
    StoneBottom,
    /// высота на которой находятся камни в нижнем правом углу
    StoneBottomRight,
    /// высота на которой находят чаши (чтобы класть в них камни)
    Bowl,
    /// Точка ожидания хода
    Park,
    /// Точка откуда роботу брать свои камни
    AiStones,
    /// Точка откуда роботу можно взять камени противника
    HimanStones,
    /// место куда сбрасывать пленных робота
    AiPrisoners,
    /// место куда сбрасывать пленных человека
    HumanPrisoners,
    /// место чаши робота
    AiBowl,
    /// место чаши человека
    HumanBowl,
    /// положение руки где пальчики ровно к камням
    Turn,
}

#[derive(Debug, Parser)]
#[command(name = "")] // делает вывод ошибки более корректным
enum CheckArmCommand {
    /// Переместить мотор X на заданнаое количество шагов
    X { steps: u32 },

    /// Переместить мотор X на заданнаое количество шагов в обратную сторону
    Xr { steps: u32 },

    /// Задать мотору X стартовую скорость
    Xss { speed: u32 },

    /// Задать мотору X максимальную скорость
    Xms { speed: u32 },

    /// Задать мотору X ускорение
    Xa { acceleration: u32 },

    /// Переместить мотор Y на заданнаое количество шагов
    Y { steps: u32 },

    /// Переместить мотор Y на заданнаое количество шагов в обратную сторону
    Yr { steps: u32 },

    /// Задать мотору Y стартовую скорость
    Yss { speed: u32 },

    /// Задать мотору Y максимальную скорость
    Yms { speed: u32 },

    /// Задать мотору Y ускорение
    Ya { acceleration: u32 },

    /// Переместить мотор Z на заданнаое количество шагов
    Z { steps: u32 },

    /// Переместить мотор Z на заданнаое количество шагов в обратную сторону
    Zr { steps: u32 },

    /// Задать мотору Z стартовую скорость
    Zss { speed: u32 },

    /// Задать мотору Z максимальную скорость
    Zms { speed: u32 },

    /// Задать мотору Z ускорение
    Za { acceleration: u32 },

    /// Узнать текущее положение моторов
    State,

    /// Сохранить текущее положение в список точек
    Save { value: PointNames },

    /// Переместиться в сохранённую точку
    Move { value: PointNames },

    /// Закрыть клешню руки
    Lock,

    /// Открыть клешню руки
    Unlock,

    /// Подвернуть клешню руки от -45 до +45 градусов
    Turn { degree: i16 },

    /// Позиционировать в 0
    Zero,

    /// переместиться на позицию камня
    Go { pos: String },

    /// Взять камень на позиции
    Take { pos: String },

    /// Положить камень на позицию
    Put { pos: String },

    /// проиграть заготовку
    Play,

    /// Выйти
    Quit,
}

fn port_process(portname: String) -> Result {
    let board_size = 19;
    let mut config = Config::new();
    let mut points = config.get_points();
    let mut arm_math = arm::math::Math::new(&points, board_size);

    // println!("");
    // println!("TL: {:?}", points.top_left);
    // println!(
    //     "TL: x_degree:{} y_degree:{}",
    //     steps_to_degree(points.top_left.x),
    //     steps_to_degree(points.top_left.y)
    // );
    // println!("TR: {:?}", points.top_right);
    // println!(
    //     "TR: x_degree:{} y_degree:{}",
    //     steps_to_degree(points.top_right.x),
    //     steps_to_degree(points.top_right.y)
    // );
    // println!("BL: {:?}", points.bottom_left);
    // println!(
    //     "BL: x_degree:{} y_degree:{}",
    //     steps_to_degree(points.bottom_left.x),
    //     steps_to_degree(points.bottom_left.y)
    // );
    // println!("BR: {:?}", points.bottom_right);
    // println!(
    //     "BR: x_degree:{} y_degree:{}",
    //     steps_to_degree(points.bottom_right.x),
    //     steps_to_degree(points.bottom_right.y)
    // );
    // println!("");

    // let mut a1 = arm_math.cell_to_xy(0, 0);
    // println!("A1: {:?}", a1);
    // let q19 = arm_math.cell_to_xy(18, 18);
    // println!("Q19: {:?}", q19);
    // let a19 = arm_math.cell_to_xy(0, 18);
    // println!("A19: {:?}", a19);
    // let q1 = arm_math.cell_to_xy(18, 0);
    // println!("Q1: {:?}", q1);

    let mut arm = ArmPositioner::new(&portname, 40)?;
    //let mut points = Vec::<MotorsState>::new();

    let motor_speeds = config.get_motor_speeds();
    arm.set_start_speed(&Motor::X, motor_speeds.x_speed);
    arm.set_max_speed(&Motor::X, motor_speeds.x_max_speed);
    arm.set_acceleration(&Motor::X, motor_speeds.x_acceleration);

    arm.set_start_speed(&Motor::Y, motor_speeds.y_speed);
    arm.set_max_speed(&Motor::Y, motor_speeds.y_max_speed);
    arm.set_acceleration(&Motor::Y, motor_speeds.y_acceleration);

    arm.set_start_speed(&Motor::Z, motor_speeds.z_speed);
    arm.set_max_speed(&Motor::Z, motor_speeds.z_max_speed);
    arm.set_acceleration(&Motor::Z, motor_speeds.z_acceleration);

    let repl = ClapEditor::<CheckArmCommand>::builder().build();

    println!("Порт подключен, можно вводить команды. help для списка комманд");
    repl.repl(|cmd| {
        let answer = match cmd {
            // базовые команды мотора X
            CheckArmCommand::X { steps } => arm.move_steps(&Motor::X, steps as i32).apply_move(),
            CheckArmCommand::Xr { steps } => {
                arm.move_steps(&Motor::X, steps as i32 * -1).apply_move()
            }
            CheckArmCommand::Xss { speed } => arm.set_start_speed(&Motor::X, speed).apply_move(),
            CheckArmCommand::Xms { speed } => arm.set_max_speed(&Motor::X, speed).apply_move(),
            CheckArmCommand::Xa { acceleration } => {
                arm.set_acceleration(&Motor::X, acceleration).apply_move()
            }

            // базовые команды мотора Y
            CheckArmCommand::Y { steps } => arm.move_steps(&Motor::Y, steps as i32).apply_move(),
            CheckArmCommand::Yr { steps } => {
                arm.move_steps(&Motor::Y, steps as i32 * -1).apply_move()
            }
            CheckArmCommand::Yss { speed } => arm.set_start_speed(&Motor::Y, speed).apply_move(),
            CheckArmCommand::Yms { speed } => arm.set_max_speed(&Motor::Y, speed).apply_move(),
            CheckArmCommand::Ya { acceleration } => {
                arm.set_acceleration(&Motor::Y, acceleration).apply_move()
            }

            // базовые команды мотора Z
            CheckArmCommand::Z { steps } => arm.move_steps(&Motor::Z, steps as i32).apply_move(),
            CheckArmCommand::Zr { steps } => {
                arm.move_steps(&Motor::Z, steps as i32 * -1).apply_move()
            }
            CheckArmCommand::Zss { speed } => arm.set_start_speed(&Motor::Z, speed).apply_move(),
            CheckArmCommand::Zms { speed } => arm.set_max_speed(&Motor::Z, speed).apply_move(),
            CheckArmCommand::Za { acceleration } => {
                arm.set_acceleration(&Motor::Z, acceleration).apply_move()
            }

            // команды работы со списком запомннных точек
            CheckArmCommand::State => {
                println!("{}", &to_string(&arm.get_motors_state()));
                Ok(())
            }

            CheckArmCommand::Save { value } => {
                let pnt = arm.get_motors_state();
                let to_xy = |p: MotorsState| XY {
                    x: p.x.pos,
                    y: p.y.pos,
                };

                match value {
                    PointNames::TopLeft => points.top_left = to_xy(pnt),
                    PointNames::TopRight => points.top_right = to_xy(pnt),
                    PointNames::BottomLeft => points.bottom_left = to_xy(pnt),
                    PointNames::BottomRight => points.bottom_right = to_xy(pnt),
                    PointNames::Move => points.move_z = pnt.z.pos,
                    PointNames::StoneTopLeft => points.stone.top_left = pnt.z.pos,
                    PointNames::StoneTop => points.stone.top = pnt.z.pos,
                    PointNames::StoneTopRight => points.stone.top_right = pnt.z.pos,
                    PointNames::StoneRight => points.stone.right = pnt.z.pos,
                    PointNames::StoneCenter => points.stone.center = pnt.z.pos,
                    PointNames::StoneLeft => points.stone.left = pnt.z.pos,
                    PointNames::StoneBottomLeft => points.stone.bottom_left = pnt.z.pos,
                    PointNames::StoneBottom => points.stone.bottom = pnt.z.pos,
                    PointNames::StoneBottomRight => points.stone.bottom_right = pnt.z.pos,
                    PointNames::Bowl => points.bowl_z = pnt.z.pos,
                    PointNames::Park => points.park = to_xy(pnt),
                    PointNames::AiStones => points.ai_new_stones = to_xy(pnt),
                    PointNames::HimanStones => points.human_new_stones = to_xy(pnt),
                    PointNames::AiPrisoners => points.ai_prisoners = to_xy(pnt),
                    PointNames::HumanPrisoners => points.human_prisoners = to_xy(pnt),
                    PointNames::AiBowl => points.ai_bowl = to_xy(pnt),
                    PointNames::HumanBowl => points.human_bowl = to_xy(pnt),
                    PointNames::Turn => points.turn = to_xy(pnt),
                }
                match config.save_points(&points) {
                    Ok(_) => {
                        arm_math = arm::math::Math::new(&points, board_size);
                        println!("Положение сохранено.");
                    }
                    Err(e) => {
                        println!("Ошибка сохранения файла. Описание: {}", e);
                    }
                }
                Ok(())
            }

            CheckArmCommand::Move { value } => {
                let move_to_xy = |arm: &mut ArmPositioner, xy: XY| {
                    arm.move_to(&Motor::X, xy.x)
                        .move_to(&Motor::Y, xy.y)
                        .apply_move()
                };
                let move_z =
                    |arm: &mut ArmPositioner, z: i32| arm.move_to(&Motor::Z, z).apply_move();

                match value {
                    PointNames::TopLeft => move_to_xy(&mut arm, points.top_left),
                    PointNames::TopRight => move_to_xy(&mut arm, points.top_right),
                    PointNames::BottomLeft => move_to_xy(&mut arm, points.bottom_left),
                    PointNames::BottomRight => move_to_xy(&mut arm, points.bottom_right),
                    PointNames::Move => move_z(&mut arm, points.move_z),
                    PointNames::StoneTopLeft => move_z(&mut arm, points.stone.top_left),
                    PointNames::StoneTop => move_z(&mut arm, points.stone.top),
                    PointNames::StoneTopRight => move_z(&mut arm, points.stone.top_right),
                    PointNames::StoneLeft => move_z(&mut arm, points.stone.left),
                    PointNames::StoneCenter => move_z(&mut arm, points.stone.center),
                    PointNames::StoneRight => move_z(&mut arm, points.stone.right),
                    PointNames::StoneBottomLeft => move_z(&mut arm, points.stone.bottom_left),
                    PointNames::StoneBottom => move_z(&mut arm, points.stone.bottom),
                    PointNames::StoneBottomRight => move_z(&mut arm, points.stone.bottom_right),
                    PointNames::Bowl => move_z(&mut arm, points.bowl_z),
                    PointNames::Park => move_to_xy(&mut arm, points.park),
                    PointNames::AiStones => move_to_xy(&mut arm, points.ai_new_stones),
                    PointNames::HimanStones => move_to_xy(&mut arm, points.human_new_stones),
                    PointNames::AiPrisoners => move_to_xy(&mut arm, points.ai_prisoners),
                    PointNames::HumanPrisoners => move_to_xy(&mut arm, points.human_prisoners),
                    PointNames::AiBowl => move_to_xy(&mut arm, points.ai_bowl),
                    PointNames::HumanBowl => move_to_xy(&mut arm, points.human_bowl),
                    PointNames::Turn => move_to_xy(&mut arm, points.turn),
                }
            }

            CheckArmCommand::Lock => arm.apply_lock(),

            CheckArmCommand::Unlock => arm.apply_unlock(),

            CheckArmCommand::Turn { degree } => arm.apply_turn_hand(degree),

            CheckArmCommand::Zero => arm.apply_zero(),

            CheckArmCommand::Go { pos } => match board::Pos::from_str(&pos) {
                Ok(pos) => go_to_stone(pos, &mut arm, &arm_math),
                Err(_) => {
                    println!("Неверный формат позиции камня.");
                    Ok(())
                }
            },

            CheckArmCommand::Take { pos } => match board::Pos::from_str(&pos) {
                Ok(pos) => arm::take_stone(pos, &mut arm, &points, &arm_math),
                Err(_) => {
                    println!("Неверный формат позиции камня.");
                    Ok(())
                }
            },

            CheckArmCommand::Put { pos } => match board::Pos::from_str(&pos) {
                Ok(pos) => arm::put_stone(pos, &mut arm, &points, &arm_math),
                Err(_) => {
                    println!("Неверный формат позиции камня.");
                    Ok(())
                }
            },

            CheckArmCommand::Quit => {
                std::process::exit(0);
            }

            CheckArmCommand::Play => play(&mut arm, &points, &arm_math),
        };
        match answer {
            Ok(()) => {}
            Err(e) => {
                println!("Произошла ошибка: {e}");
                std::process::exit(1);
            }
        }
    });

    Ok(())
}

fn to_string(motors_state: &MotorsState) -> String {
    format!(
        "X: {}  Y: {}  Z: {}",
        motors_state.x.pos, motors_state.y.pos, motors_state.z.pos
    )
}
