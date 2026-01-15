use std::thread;
use std::time::Duration;
use tracing::info;

// Import the client-side bindings from our capabilities
use proto_serial_client::SerialHandle;

#[unsafe(no_mangle)]
pub extern "C" fn exter_call(input_ptr: *mut u8, input_len: usize) -> u64 {
    #[derive(::pyroduct::FromRow, ::pyroduct::DeepRef)]
    struct __Input {
        port: String,
        baud: u32,
        command: String,
    }

    #[derive(::pyroduct::ToRow)]
    struct __Output {
        output: Vec<u8>,
    }

    let call = |input: __InputRef| {
        call(&input.port, input.baud, &input.command).map(|result| __Output { output: result })
    };

    ::pyroduct::module::call::<__Input, __Output, _>(input_ptr, input_len, call)
}

/// Execute a command on a serial terminal
// This is a more complicated IO
fn call(port: &str, baud: u32, command: &str) -> Result<Vec<u8>, String> {
    info!("[Module 3] Received request: {},{}|{}", port, baud, command);

    // Connect to serial terminal
    info!(
        "[Module 3] Opening serial terminal at '{}' baud {}",
        port, baud
    );
    let serial = SerialHandle::open(port.to_string(), baud)?;

    // Send the command with newline (terminal expects \n to execute)
    let command_with_newline = format!("{}\n", command);
    info!("[Module 3] Executing command: '{}'", command);
    let write_res = serial.write(command_with_newline.as_bytes())?;

    // Give the terminal time to process and respond
    // (WASM can't sleep natively, but we can do multiple reads)
    info!(
        "[Module 3] Wrote {} bytes Waiting for terminal response...",
        write_res
    );

    let mut output = Vec::new();
    let mut empty_reads = 0;
    let max_empty_reads = 5;

    // Try reading multiple times to get complete output
    for attempt in 0..10 {
        let read_res = serial.read(128)?;

        if read_res.is_empty() {
            empty_reads += 1;
            if empty_reads >= max_empty_reads {
                break;
            }
        } else {
            empty_reads = 0;
            output.extend(read_res);
        }

        if attempt < 9 {
            thread::sleep(Duration::from_millis(100));
        }
    }

    Ok(output)
}
