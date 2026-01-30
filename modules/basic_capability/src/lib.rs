use serial_client::SerialClient;

use pyroduct::{module, FromRow, DeepRef, ToRow};

#[module(output = "response")]
fn serial_command(command: &str, wait_response: bool) -> Result<String, String> {
    let serial = SerialClient {
        port_path: "/dev/ttyUSB0".to_string(),
    }.register()?;
    
    serial.open()?;
    
    let bytes_written = serial.write_line(command.to_string())? as u32;
    
    let response = if wait_response {
        serial.read_line()?
    } else {
        String::new()
    };
    
    serial.close()?;
    
    Ok(response)
}