use std::collections::HashMap;
use std::io::{Result, BufRead, BufReader, Write, stdin, stdout};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::{eprintln, println, thread, writeln};
use owo_colors::OwoColorize;

// Defines a type.
// Arc (Atomical Reference Counter) allows for safe sharing across multiple threads.
// Mutex ensures only one thread can modifuy the HashMap at a time, preventing data races.
// HashMap tracks Agent IDs (usize) mapped to their respective TcpStream connections.
type AgentList = Arc<Mutex<HashMap<usize, TcpStream>>>;

fn main() -> Result<()> {
    // Binds the TCP listener to all local interfaces on port 9110
    let addr = "0.0.0.0:9110";
    let listener = TcpListener::bind(addr)?;
    println!("{} Controller listing for agents on {addr}", "[*]".green().bold());

    // Initialize our shared thread-safe hashmap for connected agents
    let agents: AgentList = Arc::new(Mutex::new(HashMap::new()));
    let agents_clone = Arc::clone(&agents);

    // --- Thread 1: Accept incoming agent connections ---
    thread::spawn(move || {
        let mut next_id = 1;
        for incoming in listener.incoming() {
            match incoming {
                Ok(stream) => {
                    let peer = stream.peer_addr().unwrap();
                    println!("\n{} Agent #{} connected from {peer}", "[*]".green().bold(), next_id);

                    // Spawn a dedicated thread to read output from *this specific* agent
                    let agent_stream = stream.try_clone().unwrap();
                    let id = next_id;

                    // Clone agents reference for this reader thread
                    let agents_reader_ref = Arc::clone(&agents_clone);
                    thread::spawn(move || {
                        let mut reader = BufReader::new(agent_stream);
                        let mut line = String::new();
                        loop {
                            line.clear();
                            match reader.read_line(&mut line) {
                                Ok(0) => {
                                    println!("\n{} Agent #{} disconncted", "[*]".red().bold(), id);
                                    let mut map = agents_reader_ref.lock().unwrap();
                                    map.remove(&id);
                                    break;
                                }
                                Ok(_) => {
                                    println!("(Agent #{}): {}", id, line);
                                    let _ = stdout().flush();
                                }
                                Err(_) => break,
                            }
                        }
                    });

                    // Store the stream in our shared map
                    let mut map = agents_clone.lock().unwrap();
                    map.insert(next_id, stream);
                    next_id += 1;
                }
                Err(e) => eprintln!("Connection failed: {e}"),
            }
        }
    });

    // --- Main Thread: Controller command line interface ---
    println!("Type 'list' to see agents, or '<id> <command>' to talk to a specific agent.");

    for line_result in stdin().lock().lines() {
        let line = line_result?;
        let trimmed = line.trim();

        if trimmed == "list" {
            let map = agents.lock().unwrap();
            println!("--- Active Agents ({}) ---", map.len());
            for id in map.keys() {
                println!("   -> Agent #{}", id);
            }
            continue;
        }

        // Simple parsing: split by space to get "ID" and "COMMAND"
        // e.g., "1 hello" -> id = 1, message = "hello"
        let mut parse = trimmed.splitn(2, ' ');
        let id_str = match parse.next() {
            Some(s) => s,
            None => continue,
        };

        let message = match parse.next() {
            Some(m) => m,
            None => continue,
        };

        let target_id: usize = match id_str.parse() {
            Ok(n) => n,
            Err(_) => {
                println!("Invalid format. Use: <agent_id> <message>");
                continue;
            }
        };

        // Find the agent in our hashmap and write to its stream
        let mut map = agents.lock().unwrap();
        if let Some(stream) = map.get_mut(&target_id) {
            if let Err(e) = writeln!(stream, "{}", message) {
                eprintln!("Failed to send to Agent #{}: {}", target_id, e);
            }
        } else {
            println!("Agent #{} not found", target_id);
        }
    }

    Ok(())
}
