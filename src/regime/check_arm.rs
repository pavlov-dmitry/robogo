use super::Error;
use super::arm::arduino_port::ArduinoPort;
use super::arm::arm_positioner::{ArmPositioner, Motor, MotorsState};
use easy_repl::{CommandStatus, Repl, command, repl::ReplBuilder};

use std::cell::RefCell;
use std::rc::Rc;

use serialport::{self, SerialPortType};

type Result = std::result::Result<(), Error>;

#[derive(Default)]
struct RegimeState {
    points: Vec<MotorsState>,
}

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

fn port_process(portname: String) -> Result {
    let arm = ArmPositioner::new(&portname, 20)?;
    let arm_ptr = Rc::new(RefCell::new(arm));

    let regime = RegimeState::default();
    let regime_ptr = Rc::new(RefCell::new(regime));

    let repl_builder = create_commands(Repl::builder(), arm_ptr.clone(), regime_ptr.clone());

    let mut repl = repl_builder.build().expect("failed to build Repl");
    println!("Порт открыт. Команда 'help' покажет все возможные команды.");
    repl.run().expect("Critival REPL error");
    Ok(())
}

fn create_commands(
    repl: ReplBuilder,
    arm_ptr: Rc<RefCell<ArmPositioner>>,
    regime_ptr: Rc<RefCell<RegimeState>>,
) -> ReplBuilder {
    let repl = create_standart_motor_moves_and_speed_commands(&Motor::X, repl, arm_ptr.clone());
    let repl = create_standart_motor_moves_and_speed_commands(&Motor::Y, repl, arm_ptr.clone());
    let repl = create_standart_motor_moves_and_speed_commands(&Motor::Z, repl, arm_ptr.clone());

    let arm = arm_ptr.clone();
    let repl = repl.add(
        "state",
        command! {
            "Выводит текущую позицию всех моторов",
            () => || {
                let arm = arm.borrow();
                let motors_state = arm.get_motors_state();
                println!("{}", &to_string(&motors_state));
                Ok(CommandStatus::Done)
            }
        },
    );

    let arm = arm_ptr.clone();
    let regime = regime_ptr.clone();
    let repl = repl.add(
        "save",
        command! {
            "Сохраняет текущее положение моторов как одну из точек для повторений позиций",
            () => || {
                let arm = arm.borrow();
                let motors_state = arm.get_motors_state();
                let mut regime = regime.borrow_mut();
                regime.points.push(motors_state);
                println!("Сохранено в позицию {}", regime.points.len() - 1);
                Ok(CommandStatus::Done)
            }
        },
    );

    let regime = regime_ptr.clone();
    let repl = repl.add(
        "list",
        command! {
            "Выводит список соранённых точек",
            () => || {
                let mut regime = regime.borrow_mut();
                if regime.points.is_empty() {
                    println!("Cписок точек пуст.");
                }
                else {
                    for (idx, pnt) in regime.points.iter().enumerate() {
                        println!("{}: {}", idx + 1, to_string(pnt));
                    }
                }
                Ok(CommandStatus::Done)
            }
        },
    );

    let regime = regime_ptr.clone();
    let repl = repl.add(
        "remove",
        command! {
            "Удаляет точку из списка созранённых точек",
            (idx: usize) => |idx| {
                let idx = idx - 1;
                let mut regime = regime.borrow_mut();
                if idx < regime.points.len() {
                    regime.points.remove(idx);
                    println!("Удалена точка с индексом {idx}");
                }
                else {
                    println!("Некорректный индекс точки.");
                }
                Ok(CommandStatus::Done)
            }
        },
    );

    let regime = regime_ptr.clone();
    let arm = arm_ptr.clone();
    let repl = repl.add(
        "loop",
        command! {
            "N раз переместиться по всем созранённым точкам по кругу",
            (n: usize) => |n| {
                let regime = regime.borrow();
                if regime.points.is_empty() {
                    println!("Список точек пуст.");
                    return Ok(CommandStatus::Done);
                }
                let mut arm = arm.borrow_mut();
                for i in 0..n {
                    println!("Цикл {}/{n}", i + 1);
                    for (idx, pnt) in regime.points.iter().enumerate() {
                        arm.move_to(&Motor::X, pnt.x.pos)
                            .move_to(&Motor::Y, pnt.y.pos)
                            .move_to(&Motor::Z, pnt.z.pos)
                            .apply()?;
                        println!("Пришёл в точку {}", idx + 1);
                    }
                }
                println!("Завершено!");
                Ok(CommandStatus::Done)
            }
        },
    );

    repl
}

fn create_standart_motor_moves_and_speed_commands<'a>(
    motor: &'a Motor,
    repl: ReplBuilder<'a>,
    arm_ptr: Rc<RefCell<ArmPositioner>>,
) -> ReplBuilder<'a> {
    let arm = arm_ptr.clone();
    let repl = repl.add(
        &format!("{motor}"),
        command! {
            &format!("Сделать мотором \'{motor}\' какое-то количество шагов(если отрицательное то в обратную сторону)"),
            (steps: i32) => |steps| {
                let mut arm_ptr = arm.borrow_mut();
                arm_ptr.move_steps(motor, steps).apply()?;
                Ok(CommandStatus::Done)
            }
        },
    );

    let arm = arm_ptr.clone();
    let repl = repl.add(
        &format!("{motor}mv"),
        command! {
            &format!("Переместить мотор \'{motor}\' в определённую глобальную позицию"),
            (pos: i32) => |pos| {
                let mut arm_ptr = arm.borrow_mut();
                arm_ptr.move_to(motor, pos).apply()?;
                Ok(CommandStatus::Done)
            }
        },
    );

    let arm = arm_ptr.clone();
    let repl = repl.add(
        &format!("ss{motor}"),
        command! {
            &format!("Задать стартовую скорость мотору \'{motor}\'"),
            (speed: u32) => |speed| {
                let mut arm_ptr = arm.borrow_mut();
                arm_ptr.set_start_speed(motor, speed).apply()?;
                Ok(CommandStatus::Done)
            }
        },
    );

    let arm = arm_ptr.clone();
    let repl = repl.add(
        &format!("ms{motor}"),
        command! {
            &format!("Задать максимальную скорость мотору \'{motor}\'"),
            (speed: u32) => |speed| {
                let mut arm_ptr = arm.borrow_mut();
                arm_ptr.set_max_speed(motor, speed).apply()?;
                Ok(CommandStatus::Done)
            }
        },
    );

    let arm = arm_ptr.clone();
    let repl = repl.add(
        &format!("a{motor}"),
        command! {
            &format!("Задать ускорение мотору '{motor}\'"),
            (acceleration: u32) => |acceleration| {
                let mut arm_ptr = arm.borrow_mut();
                arm_ptr.set_acceleration(motor, acceleration).apply()?;
                Ok(CommandStatus::Done)
            }
        },
    );

    repl
}

fn to_string(motors_state: &MotorsState) -> String {
    format!(
        "X: {}  Y: {}  Z: {}",
        motors_state.x.pos, motors_state.y.pos, motors_state.z.pos
    )
}
