//! Daemon lifecycle management
//!
//! Handles PID file management, log file setup, process daemonization,
//! and daemon control operations (start, stop, kill, status).

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::PathBuf;

use directories::ProjectDirs;

/// Get the project directories for desktop-cli
fn get_project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "desktop-cli", "desktop-cli")
}

/// Get the runtime directory for PID files
pub fn get_runtime_dir() -> PathBuf {
    // Try XDG_RUNTIME_DIR first, fall back to /tmp
    if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("desktop-cli")
    } else {
        PathBuf::from("/tmp/desktop-cli")
    }
}

/// Get the data directory for log files
pub fn get_data_dir() -> PathBuf {
    get_project_dirs()
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join(".local/share/desktop-cli")
        })
}

/// Get the PID file path
pub fn get_pid_file() -> PathBuf {
    get_runtime_dir().join("desktop-cli.pid")
}

/// Get the log file path
pub fn get_log_file() -> PathBuf {
    get_data_dir().join("desktop-cli.log")
}

/// Read the PID from the PID file
pub fn read_pid() -> Option<u32> {
    let pid_file = get_pid_file();
    if !pid_file.exists() {
        return None;
    }
    
    fs::read_to_string(&pid_file)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Write the current PID to the PID file
pub fn write_pid(pid: u32) -> std::io::Result<()> {
    let pid_file = get_pid_file();
    if let Some(parent) = pid_file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&pid_file, format!("{}\n", pid))
}

/// Remove the PID file
pub fn remove_pid_file() -> std::io::Result<()> {
    let pid_file = get_pid_file();
    if pid_file.exists() {
        fs::remove_file(&pid_file)?;
    }
    Ok(())
}

/// Check if a process with the given PID is running
#[cfg(unix)]
pub fn is_process_running(pid: u32) -> bool {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    
    // Signal 0 checks if process exists without sending a signal
    kill(Pid::from_raw(pid as i32), None).is_ok()
}

#[cfg(not(unix))]
pub fn is_process_running(_pid: u32) -> bool {
    false
}

/// Check if the daemon is running
pub fn is_daemon_running() -> Option<u32> {
    read_pid().filter(|&pid| is_process_running(pid))
}

/// Stop the daemon gracefully (SIGTERM)
#[cfg(unix)]
pub fn stop_daemon() -> Result<(), String> {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    
    let pid = is_daemon_running().ok_or("Daemon is not running")?;
    
    kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
        .map_err(|e| format!("Failed to send SIGTERM: {}", e))?;
    
    // Wait for process to exit (with timeout)
    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if !is_process_running(pid) {
            remove_pid_file().ok();
            return Ok(());
        }
    }
    
    Err("Daemon did not stop within 5 seconds. Try 'kill' command.".to_string())
}

#[cfg(not(unix))]
pub fn stop_daemon() -> Result<(), String> {
    Err("Daemon management is only supported on Unix".to_string())
}

/// Force kill the daemon (SIGKILL)
#[cfg(unix)]
pub fn kill_daemon() -> Result<u32, String> {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    
    let pid = is_daemon_running().ok_or("Daemon is not running")?;
    
    kill(Pid::from_raw(pid as i32), Signal::SIGKILL)
        .map_err(|e| format!("Failed to send SIGKILL: {}", e))?;
    
    remove_pid_file().ok();
    Ok(pid)
}

#[cfg(not(unix))]
pub fn kill_daemon() -> Result<u32, String> {
    Err("Daemon management is only supported on Unix".to_string())
}

/// View the last N lines of the log file
pub fn view_logs(lines: usize) -> Result<Vec<String>, String> {
    let log_file = get_log_file();
    
    if !log_file.exists() {
        return Err("No log file found".to_string());
    }
    
    let file = File::open(&log_file)
        .map_err(|e| format!("Failed to open log file: {}", e))?;
    
    let reader = BufReader::new(file);
    let all_lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
    
    let start = all_lines.len().saturating_sub(lines);
    Ok(all_lines[start..].to_vec())
}

/// Follow the log file (tail -f style)
pub fn follow_logs() -> Result<(), String> {
    let log_file = get_log_file();
    
    if !log_file.exists() {
        return Err("No log file found".to_string());
    }
    
    let mut file = File::open(&log_file)
        .map_err(|e| format!("Failed to open log file: {}", e))?;
    
    // Seek to end of file
    file.seek(SeekFrom::End(0))
        .map_err(|e| format!("Failed to seek: {}", e))?;
    
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    
    loop {
        match reader.read_line(&mut line) {
            Ok(0) => {
                // No new data, sleep briefly
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Ok(_) => {
                print!("{}", line);
                std::io::stdout().flush().ok();
                line.clear();
            }
            Err(e) => {
                return Err(format!("Error reading log: {}", e));
            }
        }
    }
}

/// Daemon status information
pub struct DaemonStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub port: Option<u16>,
    pub log_file: PathBuf,
    pub pid_file: PathBuf,
}

impl DaemonStatus {
    pub fn check() -> Self {
        let pid = is_daemon_running();
        DaemonStatus {
            running: pid.is_some(),
            pid,
            port: None, // Could be read from config or detected
            log_file: get_log_file(),
            pid_file: get_pid_file(),
        }
    }
}

/// Setup log file for daemon mode, returns the file for writing
pub fn setup_log_file() -> std::io::Result<File> {
    let log_file = get_log_file();
    if let Some(parent) = log_file.parent() {
        fs::create_dir_all(parent)?;
    }
    
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
}

/// Daemonize the current process
#[cfg(unix)]
pub fn daemonize(port: u16) -> Result<(), String> {
    use daemonize::Daemonize;
    
    let pid_file = get_pid_file();
    let log_file = get_log_file();
    
    // Ensure directories exist
    if let Some(parent) = pid_file.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create runtime dir: {}", e))?;
    }
    if let Some(parent) = log_file.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create data dir: {}", e))?;
    }
    
    // Open log file for stdout/stderr redirection
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| format!("Failed to open log file for stdout: {}", e))?;
    
    let stderr = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| format!("Failed to open log file for stderr: {}", e))?;
    
    let daemonize = Daemonize::new()
        .pid_file(&pid_file)
        .chown_pid_file(true)
        .working_directory("/")
        .stdout(stdout)
        .stderr(stderr);
    
    daemonize.start().map_err(|e| format!("Failed to daemonize: {}", e))?;
    
    Ok(())
}

#[cfg(not(unix))]
pub fn daemonize(_port: u16) -> Result<(), String> {
    Err("Daemonization is only supported on Unix. Use --foreground mode on other platforms.".to_string())
}
