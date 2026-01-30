use serial_client::SerialClient;

use pyroduct::{module, FromRow, DeepRef, ToRow};

#[derive(FromRow, DeepRef)]
struct SerialCommand {
    command: String,
    wait_response: bool,
}

#[derive(ToRow)]
struct SerialResponse {
    sent: bool,
    response: String,
    bytes_written: u32,
}

#[module(output = SerialResponse)]
fn serial_command(input: &SerialCommandRef<'_>) -> Result<SerialResponse, String> {
    let serial = SerialClient {
        port_path: "/dev/ttyUSB0".to_string(),
    }.register()?;
    
    serial.open()?;
    
    let bytes_written = serial.write_line(input.command.to_string())? as u32;
    
    let response = if input.wait_response {
        serial.read_line()?
    } else {
        String::new()
    };
    
    serial.close()?;
    
    Ok(SerialResponse {
        sent: true,
        response,
        bytes_written,
    })
}