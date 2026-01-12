mod automation;
mod daemon;
mod error;
mod executor;
mod gemini;
mod rpc;

use clap::{Parser, Subcommand};
use std::net::Ipv4Addr;
use tokio::net::TcpStream;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use remoc::prelude::*;
use rpc::service::DesktopService;
use rpc::{DesktopServiceClient, ListWindowsRequest, ScreenshotRequest, ExecuteRequest, DetectRequest};

/// Desktop Daemon - Control desktop applications through visual grounding
#[derive(Parser, Debug)]
#[command(name = "desktop", author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the daemon as a background process
    Start {
        /// Comma-separated list of allowed executables (e.g., "notepad.exe,chrome.exe")
        #[arg(long)]
        allowed_executables: Option<String>,

        /// Server port
        #[arg(long, default_value = "9870")]
        port: u16,

        /// Gemini model to use
        #[arg(long, default_value = "gemini-3-flash-preview")]
        gemini_model: String,

        /// Gemini API key (can also be set via GEMINI_API_KEY environment variable)
        #[arg(long)]
        gemini_api_key: Option<String>,

        /// Run in foreground (don't daemonize)
        #[arg(long, short = 'f')]
        foreground: bool,
    },

    /// Stop the running daemon gracefully
    Stop,

    /// Force kill the daemon
    Kill,

    /// Show daemon status
    Status,

    /// View daemon logs
    Logs {
        /// Follow log output
        #[arg(short, long)]
        follow: bool,

        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "50")]
        lines: usize,
    },

    /// Call a service method on the running daemon
    Call {
        #[command(subcommand)]
        method: CallMethod,

        /// Daemon port
        #[arg(long, default_value = "9870", global = true)]
        port: u16,
    },
}

#[derive(Subcommand, Debug)]
enum CallMethod {
    /// List visible windows
    ListWindows {
        /// Filter by executable path
        #[arg(long)]
        exe_filter: Option<String>,

        /// Regex pattern to match window titles
        #[arg(long)]
        title_pattern: Option<String>,
    },

    /// Take a screenshot of a window
    Screenshot {
        /// Window handle (HWND)
        hwnd: String,

        /// Screenshot method (bitblt or printwindow)
        #[arg(long)]
        method: Option<String>,
    },

    /// Execute natural language instructions on a window
    Execute {
        /// Window handle (HWND)
        hwnd: String,

        /// Natural language instructions
        #[arg(required = true)]
        instructions: Vec<String>,

        /// Retry strategy (none, basic, advanced)
        #[arg(long)]
        retry_strategy: Option<String>,
    },

    /// Detect UI elements using visual grounding
    Detect {
        /// Window handle (HWND)
        hwnd: String,

        /// Natural language query
        query: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start {
            allowed_executables,
            port,
            gemini_model,
            gemini_api_key,
            foreground,
        } => {
            cmd_start(allowed_executables, port, gemini_model, gemini_api_key, foreground).await?;
        }
        Commands::Stop => {
            cmd_stop()?;
        }
        Commands::Kill => {
            cmd_kill()?;
        }
        Commands::Status => {
            cmd_status();
        }
        Commands::Logs { follow, lines } => {
            cmd_logs(follow, lines)?;
        }
        Commands::Call { method, port } => {
            cmd_call(method, port).await?;
        }
    }

    Ok(())
}

async fn cmd_start(
    allowed_executables: Option<String>,
    port: u16,
    gemini_model: String,
    gemini_api_key: Option<String>,
    foreground: bool,
) -> anyhow::Result<()> {
    // Check if daemon is already running
    if let Some(pid) = daemon::is_daemon_running() {
        eprintln!("Daemon is already running (PID: {})", pid);
        std::process::exit(1);
    }

    // Get Gemini API key from args or environment
    let gemini_api_key = gemini_api_key
        .or_else(|| std::env::var("GEMINI_API_KEY").ok());

    // Validate Gemini API key
    if gemini_api_key.is_none() {
        eprintln!("Error: Gemini API key is required. Set GEMINI_API_KEY environment variable or use --gemini-api-key");
        std::process::exit(1);
    }

    // Parse allowed executables
    let allowed_executables = allowed_executables
        .as_ref()
        .map(|s| {
            s.split(',')
                .map(|e| e.trim().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if foreground {
        // Run in foreground with console logging
        setup_console_logging();
        run_server(port, gemini_model, gemini_api_key.unwrap(), allowed_executables).await?;
    } else {
        // Daemonize the process
        println!("Starting Desktop daemon on port {}...", port);
        
        daemon::daemonize(port).map_err(|e| anyhow::anyhow!("{}", e))?;
        
        // After daemonization, we're in the child process
        // Setup file-based logging
        setup_file_logging();
        
        // Write our PID (daemonize creates the file but let's ensure it's correct)
        let pid = std::process::id();
        daemon::write_pid(pid)?;
        
        tracing::info!("Desktop daemon started (PID: {})", pid);
        
        run_server(port, gemini_model, gemini_api_key.unwrap(), allowed_executables).await?;
    }

    Ok(())
}

fn cmd_stop() -> anyhow::Result<()> {
    match daemon::stop_daemon() {
        Ok(()) => {
            println!("Daemon stopped gracefully");
            Ok(())
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_kill() -> anyhow::Result<()> {
    match daemon::kill_daemon() {
        Ok(pid) => {
            println!("Daemon killed (PID: {})", pid);
            Ok(())
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_status() {
    let status = daemon::DaemonStatus::check();
    
    if status.running {
        println!("Desktop daemon is running (PID: {})", status.pid.unwrap());
    } else {
        println!("Desktop daemon is not running");
    }
    
    println!();
    println!("PID file: {}", status.pid_file.display());
    println!("Log file: {}", status.log_file.display());
}

fn cmd_logs(follow: bool, lines: usize) -> anyhow::Result<()> {
    if follow {
        daemon::follow_logs().map_err(|e| anyhow::anyhow!("{}", e))?;
    } else {
        match daemon::view_logs(lines) {
            Ok(log_lines) => {
                for line in log_lines {
                    println!("{}", line);
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

async fn cmd_call(method: CallMethod, port: u16) -> anyhow::Result<()> {
    // Connect to daemon
    let socket = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).await
        .map_err(|e| anyhow::anyhow!("Failed to connect to daemon on port {}: {}", port, e))?;
    
    let (socket_rx, socket_tx) = socket.into_split();
    
    // Establish remoc connection
    let (conn, _tx, mut rx): (_, rch::base::Sender<()>, rch::base::Receiver<DesktopServiceClient>) =
        remoc::Connect::io(remoc::Cfg::default(), socket_rx, socket_tx).await?;
    
    tokio::spawn(conn);
    
    // Receive the service client
    let mut client = rx.recv().await?
        .ok_or_else(|| anyhow::anyhow!("Failed to receive service client"))?;
    
    // Call the requested method
    match method {
        CallMethod::ListWindows { exe_filter, title_pattern } => {
            let result = client.list_windows(ListWindowsRequest {
                executable_filter: exe_filter,
                title_pattern,
            }).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        CallMethod::Screenshot { hwnd, method } => {
            let result = client.take_screenshot(ScreenshotRequest {
                hwnd,
                method,
            }).await?;
            // Print metadata, not the full base64
            println!("{{\"width\": {}, \"height\": {}, \"format\": \"{}\", \"base64_length\": {}}}", 
                result.width, result.height, result.format, result.base64_image.len());
        }
        CallMethod::Execute { hwnd, instructions, retry_strategy } => {
            let result = client.execute_instructions(ExecuteRequest {
                hwnd,
                instructions,
                retry_strategy,
            }).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        CallMethod::Detect { hwnd, query } => {
            let result = client.detect_elements(DetectRequest {
                hwnd,
                query,
            }).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }
    
    Ok(())
}

fn setup_console_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "desktop_cli=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

fn setup_file_logging() {
    // For daemon mode, we write to the log file
    // The daemonize crate redirects stdout/stderr to the log file,
    // so we can just use the default fmt layer
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "desktop_cli=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_ansi(false))
        .init();
}

async fn run_server(
    port: u16,
    gemini_model: String,
    gemini_api_key: String,
    allowed_executables: Vec<String>,
) -> anyhow::Result<()> {
    tracing::info!(
        "Starting Desktop Daemon on port {} with Gemini model {}",
        port,
        gemini_model
    );

    if !allowed_executables.is_empty() {
        tracing::info!("Allowed executables: {:?}", allowed_executables);
    } else {
        tracing::warn!("No executable whitelist set - all windows are accessible");
    }

    // Initialize Gemini client
    let gemini_client = gemini::GeminiClient::new(gemini_api_key, gemini_model)?;

    tracing::info!("Gemini client initialized with model: {}", gemini_client.model());

    // Create RPC server configuration
    let server_config = rpc::server::RpcServerConfig {
        port,
        gemini_client,
        allowed_executables,
    };

    // Start the RPC server
    rpc::start_server(server_config).await?;

    Ok(())
}
