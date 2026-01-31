use serial_client::{SerialClient, SerialClientMethods};

#[pyroduct::module(output = response)]
fn serial_command(command: &str, wait_response: bool) -> Result<String, String> {
    let serial = SerialClient {
        port_path: "/dev/ttyUSB0".to_string(),
    }.register()?;
    
    serial.open()?;
    
    serial.write_line(command.to_string())? as u32;
    
    let response = if wait_response {
        serial.read_line()?
    } else {
        String::new()
    };
    
    serial.close()?;
    
    Ok(response)
}