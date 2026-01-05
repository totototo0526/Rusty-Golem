use std::fs;
use std::io::{self, Write};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use chrono::{Local, NaiveTime};
use serde::Deserialize;
use reqwest::blocking::Client;

#[derive(Deserialize, Debug)]
struct Config {
    server_bat_path: String,
    start_time: String,
    end_time: String,
    discord_webhook_url: String,
}

fn load_config() -> Config {
    let content = fs::read_to_string("config.toml").expect("Failed to read config.toml");
    toml::from_str(&content).expect("Failed to parse config.toml")
}

fn send_discord_message(url: &str, message: &str) {
    let client = Client::new();
    let payload = serde_json::json!({
        "content": message
    });
    
    // Ignore errors for now to avoid crashing the main loop
    let _ = client.post(url).json(&payload).send();
}

fn start_server(path: &str) -> io::Result<Child> {
    // On Windows, running a .bat file often requires using "cmd /C"
    // But sometimes it works directly. Since the user is on Windows, 
    // we should try to execute it in a way that works for .bat.
    // Usually: Command::new("cmd").args(&["/C", path])...
    
    Command::new("cmd")
        .args(&["/C", path])
        .stdin(Stdio::piped()) // Capture stdin to send commands later
        .stdout(Stdio::inherit()) // Let the user see the server output in the terminal
        .stderr(Stdio::inherit())
        .spawn()
}

fn send_command(child: &mut Child, command: &str) {
    if let Some(stdin) = child.stdin.as_mut() {
        let _ = writeln!(stdin, "{}", command);
    }
}

fn stop_server(child: &mut Child) {
    send_command(child, "stop");
    // Wait a bit for it to stop gracefully
    // In a real production app we might want to wait on child.wait() with a timeout, 
    // but std::process doesn't have a simple timeout wait. 
    // We will just let the main loop handle the cleanup or wait endlessly if that's safer.
    // For now, let's just send stop and let the watchdog/loop handle the rest.
    let _ = child.wait(); 
}

fn main() {
    let config = load_config();
    println!("Loaded config: {:?}", config);
    
    // Parse times
    let start_time = NaiveTime::parse_from_str(&config.start_time, "%H:%M").expect("Invalid start_time format");
    let end_time = NaiveTime::parse_from_str(&config.end_time, "%H:%M").expect("Invalid end_time format");
    
    let mut server_process: Option<Child> = None;
    
    // Warning states
    let mut warned_10_min = false;
    let mut warned_5_min = false;
    let mut warned_1_min = false;
    
    // Watchdog history
    let mut crash_timestamps: Vec<chrono::DateTime<Local>> = Vec::new();

    send_discord_message(&config.discord_webhook_url, "Rusty-Golem started.");

    loop {
        let now = Local::now();
        let current_time = now.time();
        
        let is_running_time = if start_time <= end_time {
             current_time >= start_time && current_time < end_time
        } else {
             current_time >= start_time || current_time < end_time
        };
        
        let mut is_alive = false;
        if let Some(child) = server_process.as_mut() {
            match child.try_wait() {
                Ok(Some(_)) => is_alive = false,
                Ok(None) => is_alive = true,
                Err(_) => is_alive = false,
            }
        }
        
        if !is_alive {
            if is_running_time {
                 // Check watchdog limits
                 crash_timestamps.retain(|&t| (now - t).num_minutes() <= 5);
                 
                 if crash_timestamps.len() >= 3 {
                      println!("Watchdog: Too many crashes (3 in 5 mins). Stopping auto-restart.");
                      send_discord_message(&config.discord_webhook_url, "Watchdog: Server crashed 3 times. Giving up.");
                      thread::sleep(Duration::from_secs(60));
                      continue; 
                 }
                 
                 println!("Starting server...");
                 send_discord_message(&config.discord_webhook_url, "Starting Minecraft Server...");
                 
                 match start_server(&config.server_bat_path) {
                     Ok(child) => {
                         server_process = Some(child);
                         crash_timestamps.push(now);
                         // Reset warnings
                         warned_10_min = false;
                         warned_5_min = false;
                         warned_1_min = false;
                     }
                     Err(e) => {
                         println!("Failed to start: {}", e);
                         crash_timestamps.push(now);
                     }
                 }
            } else {
                 if server_process.is_some() {
                     server_process = None;
                 }
            }
        } else {
             // Alive
             if !is_running_time {
                 println!("Time to stop. Stopping server...");
                 send_discord_message(&config.discord_webhook_url, "Stopping Minecraft Server (Schedule)...");
                 if let Some(mut child) = server_process.take() {
                      stop_server(&mut child);
                 }
             } else {
                 let minutes_left = if start_time <= end_time {
                      (end_time - current_time).num_minutes()
                 } else {
                      if current_time < end_time {
                          (end_time - current_time).num_minutes()
                      } else {
                          (end_time - current_time).num_minutes() + 24 * 60
                      }
                 };
                 
                 if minutes_left == 10 && !warned_10_min {
                      if let Some(child) = server_process.as_mut() {
                          send_command(child, "say Server will stop in 10 minutes!");
                          warned_10_min = true;
                      }
                 }
                 else if minutes_left == 5 && !warned_5_min {
                      if let Some(child) = server_process.as_mut() {
                          send_command(child, "say Server will stop in 5 minutes!");
                          warned_5_min = true;
                      }
                 }
                 else if minutes_left == 1 && !warned_1_min {
                      if let Some(child) = server_process.as_mut() {
                          send_command(child, "say Server will stop in 1 minute!");
                          warned_1_min = true;
                      }
                 }
             }
        }
        
        thread::sleep(Duration::from_secs(10));
    }
}
