use serialport;

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
        println!("name: {}", info.port_name);
    }
    Ok(())
}

fn port_process(portname: String) -> Result {
    todo!();
}
