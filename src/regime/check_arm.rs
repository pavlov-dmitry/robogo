use super::Error;
use super::arm::arduino_port::ArduinoPort;
use super::arm::arm_positioner::{ArmPositioner, Motor, MotorsState};
use clap::Parser;
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
    Save,

    /// Показать список сохранённых точек
    List,

    /// Удалить точку из списка сохранённых
    Remove { idx: usize },

    /// Переместиться в сохранённую точку
    Move { idx: usize },

    /// Пройтись по всем точкам n раз по кругу
    Loop { n: usize },

    /// Закрыть клешню руки
    Lock,

    /// Открыть клешню руки
    Unlock,

    /// Подвернуть клешню руки от -45 до +45 градусов
    Turn { degree: i16 },

    /// Выйти
    Quit,
}

fn port_process(portname: String) -> Result {
    let mut arm = ArmPositioner::new(&portname, 20)?;
    let mut points = Vec::<MotorsState>::new();

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

            CheckArmCommand::Save => {
                let pnt = arm.get_motors_state();
                points.push(pnt);
                println!("Положение сохранени под индексом {}", points.len());
                Ok(())
            }

            CheckArmCommand::List => {
                for (idx, pnt) in points.iter().enumerate() {
                    println!("{}: {}", idx + 1, &to_string(&pnt));
                }
                Ok(())
            }

            CheckArmCommand::Remove { idx } => {
                let idx = idx - 1;
                if idx < points.len() {
                    points.remove(idx);
                    println!("Удалена");
                } else {
                    println!("Неверный индекс.")
                }
                Ok(())
            }

            CheckArmCommand::Move { idx } => {
                let idx = idx - 1;
                match points.get(idx) {
                    Some(pnt) => arm
                        .move_to(&Motor::X, pnt.x.pos)
                        .move_to(&Motor::Y, pnt.y.pos)
                        .move_to(&Motor::Z, pnt.z.pos)
                        .apply_move(),
                    None => {
                        println!("Неверный индекс точки");
                        Ok(())
                    }
                }
            }

            CheckArmCommand::Loop { n } => {
                let mut result = Ok(());
                'all: for circle_idx in 0..n {
                    println!("Цыкл {}", circle_idx + 1);
                    for (idx, pnt) in points.iter().enumerate() {
                        println!("Иду к точке {}", idx + 1);
                        let answer = arm
                            .move_to(&Motor::X, pnt.x.pos)
                            .move_to(&Motor::Y, pnt.y.pos)
                            .move_to(&Motor::Z, pnt.z.pos)
                            .apply_move();
                        if let Err(e) = answer {
                            result = Err(e);
                            break 'all;
                        }
                    }
                }
                println!("Конец циклов");
                result
            }

            CheckArmCommand::Lock => arm.apply_lock(),

            CheckArmCommand::Unlock => arm.apply_unlock(),

            CheckArmCommand::Turn { degree } => arm.apply_turn_hand(degree),

            CheckArmCommand::Quit => {
                std::process::exit(0);
            }
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
