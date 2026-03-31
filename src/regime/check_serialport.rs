use std::fmt::Display;

use serialport::{self, SerialPort, SerialPortType};

type Result = std::result::Result<(), serialport::Error>;

pub fn exec(portname: Option<String>) -> Result {
    match portname {
        Some(portname) => port_process(portname),
        None => show_aviableports(),
    }
}

fn show_aviableports() -> Result {
    println!("Aviable ports:");
    let infos = serialport::available_ports()?;
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
    todo!();
}
