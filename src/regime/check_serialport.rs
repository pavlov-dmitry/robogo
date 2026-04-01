use super::Error;
use super::arm::arduino_port::ArduinoPort;

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

fn port_process(portname: String) -> Result {
    let mut port = ArduinoPort::new(&portname, 20)?;
    println!("Port opened. type a commnd to port and wait for answer.");

    loop {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;

        if !line.is_empty() {
            let answer = port.send_cmd(&line)?;
            println!("{answer}");
        }
    }
}
