use super::Error;

use serialport::{self, SerialPortType};
use std::io;

type Result = std::result::Result<(), Error>;

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

fn port_process(portname: String) -> std::result::Result<(), Error> {
    let mut port = serialport::new(portname, 9600).open()?;
    println!("Port opened. type a commnd to port and wait for answer.");

    loop {
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;

        if !line.is_empty() {
            writeln!(&mut port, "{line}")?;

            port.read_to_string(&mut line)?;
            println!("{line}");
        }
    }
}
