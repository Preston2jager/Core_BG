use std::fs::File;
use std::io::{BufRead, BufReader, Write, Read};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::Duration;
use std::os::windows::process::CommandExt;
use crate::app::log_msg;

const CREATE_NO_WINDOW: u32 = 0x08000000;

const SETTINGS_FILE: &str = "ssh_settings.txt";

pub struct GpuSshMonitor {
    gpu_usage: Arc<AtomicU32>,
    is_connected: Arc<std::sync::atomic::AtomicBool>,
}

struct SshConfig {
    host: String,
    user: String,
    port: u16,
    key_path: String,
    password: String,
    remote_command: String,
    ssh_command: Option<String>,
}

impl GpuSshMonitor {
    pub fn new() -> Self {
        log_msg("GpuSshMonitor::new: Initializing remote GPU SSH monitor");
        
        let gpu_usage = Arc::new(AtomicU32::new(0));
        let gpu_usage_clone = gpu_usage.clone();
        let is_connected = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let is_connected_clone = is_connected.clone();
        
        // Ensure default settings exist
        ensure_default_settings();
        
        // Spawn background polling thread
        thread::spawn(move || {
            log_msg("GPU SSH Monitor background thread started");
            
            while !should_exit_app() {
                is_connected_clone.store(false, Ordering::Relaxed);
                let config = match load_ssh_config() {
                    Ok(c) => c,
                    Err(e) => {
                        log_msg(&format!("GPU SSH Monitor: Error loading settings: {}. Retrying in 5 seconds...", e));
                        thread::sleep(Duration::from_secs(5));
                        continue;
                    }
                };
                
                if config.host.is_empty() || config.host == "127.0.0.1" {
                    log_msg("GPU SSH Monitor: Connection host is empty or default (127.0.0.1). Please update ssh_settings.txt. Retrying in 10 seconds...");
                    thread::sleep(Duration::from_secs(10));
                    continue;
                }
                
                log_msg(&format!("GPU SSH Monitor: Attempting connection to {}@{}", config.user, config.host));
                
                let mut askpass_path = None;
                if !config.password.is_empty() {
                    let temp_path = std::env::current_dir()
                        .unwrap_or_else(|_| std::path::PathBuf::from("."))
                        .join("askpass_temp.bat");
                    
                    if let Ok(mut f) = File::create(&temp_path) {
                        let content = format!("@echo off\necho {}\n", config.password);
                        let _ = f.write_all(content.as_bytes());
                        askpass_path = Some(temp_path);
                    }
                }

                let mut child = match spawn_ssh_process(&config, askpass_path.as_ref()) {
                    Ok(child) => child,
                    Err(e) => {
                        log_msg(&format!("GPU SSH Monitor: Failed to spawn SSH process: {}. Retrying in 5 seconds...", e));
                        if let Some(ref path) = askpass_path {
                            let _ = std::fs::remove_file(path);
                        }
                        thread::sleep(Duration::from_secs(5));
                        continue;
                    }
                };
                
                log_msg("GPU SSH Monitor: SSH process spawned successfully, reading stream...");
                
                // Spawn a helper thread to read and log stderr messages (for diagnostic purposes)
                if let Some(stderr) = child.stderr.take() {
                    thread::spawn(move || {
                        let reader = BufReader::new(stderr);
                        for line in reader.lines() {
                            if let Ok(text) = line {
                                log_msg(&format!("GPU SSH Monitor [stderr]: {}", text.trim()));
                            }
                        }
                    });
                }
                
                if let Some(stdout) = child.stdout.take() {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines() {
                        if should_exit_app() {
                            break;
                        }
                        
                        match line {
                            Ok(text) => {
                                let trimmed = text.trim();
                                if trimmed.is_empty() {
                                    continue;
                                }
                                
                                // Clean up non-digits to handle "%" or spaces
                                let clean_digits: String = trimmed.chars()
                                    .filter(|c| c.is_digit(10) || *c == '.')
                                    .collect();
                                
                                if let Ok(val) = clean_digits.parse::<f32>() {
                                    let usage = val.clamp(0.0, 100.0) as u32;
                                    gpu_usage_clone.store(usage, Ordering::Relaxed);
                                    is_connected_clone.store(true, Ordering::Relaxed);
                                }
                            }
                            Err(e) => {
                                log_msg(&format!("GPU SSH Monitor: Error reading stdout: {}", e));
                                break;
                            }
                        }
                    }
                }
                
                is_connected_clone.store(false, Ordering::Relaxed);
                
                // If we get here, the stream finished or broke. Clean up.
                log_msg("GPU SSH Monitor: SSH stream disconnected or process exited.");
                let _ = child.kill();
                let _ = child.wait();
                
                if let Some(ref path) = askpass_path {
                    let _ = std::fs::remove_file(path);
                }
                
                if should_exit_app() {
                    break;
                }
                
                log_msg("GPU SSH Monitor: Reconnecting in 5 seconds...");
                thread::sleep(Duration::from_secs(5));
            }
            
            log_msg("GPU SSH Monitor background thread exiting");
        });
        
        Self { gpu_usage, is_connected }
    }
    
    pub fn refresh(&mut self) {
        // Asynchronous updates from background thread, so refresh is a no-op
    }
    
    pub fn get_overall_usage(&self) -> f32 {
        self.gpu_usage.load(Ordering::Relaxed) as f32
    }
    
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Relaxed)
    }
}

fn should_exit_app() -> bool {
    if let Ok(state) = crate::app::STATE.lock() {
        state.should_exit
    } else {
        false
    }
}

fn ensure_default_settings() {
    let path = std::path::Path::new(SETTINGS_FILE);
    if !path.exists() {
        log_msg("GPU SSH Monitor: Settings file not found, generating template");
        if let Ok(mut file) = File::create(path) {
            let template = "\
# SSH Settings for StarCore GPU Monitoring Variant
# 
# Instructions:
# 1. Provide target host IP/domain and user.
# 2. Key authentication is recommended. Specify key_path if not loaded in your SSH agent.
# 3. Target server must have nvidia-smi installed.

host=127.0.0.1
user=gpumonitor
port=22
password=
key_path=
remote_command=nvidia-smi -i 0 --query-gpu=utilization.gpu --format=csv,noheader,nounits -lms 200

# Optional: Override the entire ssh command execution.
# If specified (without prefix '#'), this exact command will be run in cmd.exe.
# ssh_command=ssh -o StrictHostKeyChecking=accept-new -p 22 user@host \"nvidia-smi -i 0 --query-gpu=utilization.gpu --format=csv,noheader,nounits -lms 200\"
";
            let _ = file.write_all(template.as_bytes());
        }
    }
}

fn load_ssh_config() -> Result<SshConfig, String> {
    let mut file = File::open(SETTINGS_FILE).map_err(|e| format!("Failed to open config: {}", e))?;
    let mut content = String::new();
    file.read_to_string(&mut content).map_err(|e| format!("Failed to read config: {}", e))?;
    
    let mut config = SshConfig {
        host: String::new(),
        user: String::new(),
        port: 22,
        key_path: String::new(),
        password: String::new(),
        remote_command: "nvidia-smi -i 0 --query-gpu=utilization.gpu --format=csv,noheader,nounits -lms 200".to_string(),
        ssh_command: None,
    };
    
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        
        let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
        if parts.len() == 2 {
            let key = parts[0].trim();
            let val = parts[1].trim();
            match key {
                "host" => config.host = val.to_string(),
                "user" => config.user = val.to_string(),
                "port" => {
                    if let Ok(p) = val.parse::<u16>() {
                        config.port = p;
                    }
                }
                "key_path" => config.key_path = val.to_string(),
                "password" => config.password = val.to_string(),
                "remote_command" => config.remote_command = val.to_string(),
                "ssh_command" => config.ssh_command = Some(val.to_string()),
                _ => {}
            }
        }
    }
    
    Ok(config)
}

fn spawn_ssh_process(config: &SshConfig, askpass_path: Option<&std::path::PathBuf>) -> std::io::Result<std::process::Child> {
    if let Some(ref custom_cmd) = config.ssh_command {
        log_msg(&format!("GPU SSH Monitor: Spawning custom command: {}", custom_cmd));
        Command::new("cmd")
            .args(&["/C", custom_cmd])
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    } else {
        let mut args = Vec::new();
        
        // Add port argument
        args.push("-p".to_string());
        args.push(config.port.to_string());
        
        // Add key file if provided
        if !config.key_path.is_empty() {
            args.push("-i".to_string());
            args.push(config.key_path.clone());
        }
        
        // Add default options to prevent interactive prompts where possible
        args.push("-o".to_string());
        args.push("StrictHostKeyChecking=accept-new".to_string());
        
        args.push("-o".to_string());
        if askpass_path.is_some() {
            args.push("BatchMode=no".to_string());
        } else {
            args.push("BatchMode=yes".to_string());
        }
        
        // User & Host
        let target = format!("{}@{}", config.user, config.host);
        args.push(target);
        
        // Remote command to run
        args.push(config.remote_command.clone());
        
        log_msg(&format!("GPU SSH Monitor: Spawning ssh client with args: {:?}", args));
        
        let mut cmd = Command::new("ssh");
        cmd.args(&args)
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
            
        if let Some(path) = askpass_path {
            cmd.env("SSH_ASKPASS", path.to_str().unwrap_or(""));
            cmd.env("SSH_ASKPASS_REQUIRE", "force");
            cmd.env("DISPLAY", "dummy");
        }
        
        cmd.spawn()
    }
}
