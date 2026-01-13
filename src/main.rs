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
use rpc::{DesktopServiceClient, ScreenshotRequest, ExecuteRequest, DetectRequest};
use rpc::{DumpTreeRequest, FindElementRequest, InvokePatternRequest};
use rpc::{ListWindowsRequest, SetDefaultWindowRequest, GetDefaultWindowRequest};
use rpc::{SummaryRequest, QueryRequest};

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

        /// Filter windows by executable path (substring match, persists for session)
        #[arg(long)]
        exe_filter: Option<String>,

        /// Regex pattern to filter window titles (persists for session)
        #[arg(long)]
        title_pattern: Option<String>,
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

    /// Window management commands
    Window {
        #[command(subcommand)]
        cmd: WindowCommand,

        /// Daemon port
        #[arg(long, default_value = "9870", global = true)]
        port: u16,
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
enum WindowCommand {
    /// List visible windows (with session filters applied)
    List,

    /// Set the default target window for subsequent commands
    SetDefault {
        /// Window index (1-based from list) or HWND
        window: String,
    },

    /// Show the current default window
    GetDefault,
}

#[derive(Subcommand, Debug)]
enum CallMethod {
    /// Take a screenshot of a window
    Screenshot {
        /// Window handle (HWND) - uses default if not specified
        #[arg(long)]
        hwnd: Option<String>,

        /// Screenshot method (bitblt or printwindow)
        #[arg(long)]
        method: Option<String>,
    },

    /// Execute natural language instructions on a window
    Execute {
        /// Window handle (HWND) - uses default if not specified
        #[arg(long)]
        hwnd: Option<String>,

        /// Natural language instructions
        #[arg(required = true)]
        instructions: Vec<String>,

        /// Retry strategy (none, basic, advanced)
        #[arg(long)]
        retry_strategy: Option<String>,
    },

    /// Detect UI elements using visual grounding
    Detect {
        /// Window handle (HWND) - uses default if not specified
        #[arg(long)]
        hwnd: Option<String>,

        /// Natural language query
        query: String,
    },

    // =========================================================================
    // UIA (UI Automation) Commands
    // =========================================================================

    /// Dump the UIA element tree for a window
    DumpTree {
        /// Window handle (HWND) - uses default if not specified
        #[arg(long)]
        hwnd: Option<String>,

        /// Maximum depth
        #[arg(long, default_value = "5")]
        depth: u32,

        /// Prune offscreen elements
        #[arg(long)]
        prune_offscreen: bool,

        /// Prune empty elements (no name and no patterns)
        #[arg(long)]
        prune_empty: bool,

        /// Max list items per container (0 = unlimited)
        #[arg(long, default_value = "20")]
        max_list_items: u32,
    },

    /// Find UI elements by CSS-style selector
    FindElement {
        /// Window handle (HWND) - uses default if not specified
        #[arg(long)]
        hwnd: Option<String>,

        /// CSS-style selector (e.g., "Button#save", "[name~='*OK*']")
        selector: String,

        /// Find all matches (default: first only)
        #[arg(long)]
        all: bool,

        /// Timeout in milliseconds
        #[arg(long, default_value = "3000")]
        timeout: u64,
    },

    /// Invoke a UIA pattern operation on an element
    Invoke {
        /// Window handle (HWND) - uses default if not specified
        #[arg(long)]
        hwnd: Option<String>,

        /// CSS-style selector to find target element
        selector: String,

        /// Pattern operation (invoke, get-value, set-value, toggle, select, expand, collapse)
        #[arg(long)]
        pattern: String,

        /// Value for set operations
        #[arg(long)]
        value: Option<String>,
    },

    // =========================================================================
    // LLM-Optimized Commands (compact output for AI agents)
    // =========================================================================

    /// Get a compact UI summary optimized for LLM consumption
    ///
    /// Returns categorized elements (actions, navigation, content) with
    /// minimal noise. Use this after every action to understand UI state.
    Summary {
        /// Window handle (HWND) - uses default if not specified
        #[arg(long)]
        hwnd: Option<String>,

        /// Output format: json (default), text
        #[arg(long, default_value = "json")]
        format: String,

        /// Include bounding boxes in output
        #[arg(long)]
        bounds: bool,

        /// Include full hierarchy paths
        #[arg(long)]
        paths: bool,

        /// Focus on region: x,y,w,h (e.g., "100,200,300,400")
        #[arg(long)]
        region: Option<String>,

        /// Maximum depth (default: 10)
        #[arg(long, default_value = "10")]
        depth: u32,

        /// Filter by roles (comma-separated: button,input,menu)
        #[arg(long)]
        roles: Option<String>,
    },

    /// Query elements using enhanced LLM-friendly syntax
    ///
    /// Examples:
    ///   @button "Save"        - Button with name "Save"
    ///   @input:enabled        - All enabled input fields
    ///   #btnSave              - Element with automation ID
    ///   @menu "File" > "Open" - Menu path navigation
    ///   @tab:nth(2)           - Second tab
    ///   ~below("Label") @input - Input below a label
    Query {
        /// Window handle (HWND) - uses default if not specified
        #[arg(long)]
        hwnd: Option<String>,

        /// Query string (see examples in help)
        query: String,

        /// Return all matches (default: first only)
        #[arg(long)]
        all: bool,

        /// Output format: full, compact, refs
        #[arg(long, default_value = "compact")]
        format: String,

        /// Timeout in milliseconds
        #[arg(long, default_value = "3000")]
        timeout: u64,
    },

    /// Perform an action on an element and return UI summary
    ///
    /// Combines query + invoke + summary in one call for efficiency.
    /// This is the recommended way for LLMs to interact with UI elements.
    Do {
        /// Window handle (HWND) - uses default if not specified
        #[arg(long)]
        hwnd: Option<String>,

        /// Action: click, type, toggle, expand, collapse, select
        action: String,

        /// Target element query (e.g., @button "Save", #inputField)
        target: String,

        /// Value for type/set operations
        #[arg(long)]
        value: Option<String>,
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
            exe_filter,
            title_pattern,
        } => {
            cmd_start(allowed_executables, port, gemini_model, gemini_api_key, foreground, exe_filter, title_pattern).await?;
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
        Commands::Window { cmd, port } => {
            cmd_window(cmd, port).await?;
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
    exe_filter: Option<String>,
    title_pattern: Option<String>,
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
        run_server(port, gemini_model, gemini_api_key.unwrap(), allowed_executables, exe_filter, title_pattern).await?;
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
        
        run_server(port, gemini_model, gemini_api_key.unwrap(), allowed_executables, exe_filter, title_pattern).await?;
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

async fn cmd_window(cmd: WindowCommand, port: u16) -> anyhow::Result<()> {
    let mut client = connect_to_daemon(port).await?;

    match cmd {
        WindowCommand::List => {
            let result = client.list_windows(ListWindowsRequest::default()).await?;
            // Print with 1-based index for user-friendly selection
            for (i, window) in result.iter().enumerate() {
                println!("[{}] {} - {} ({})", i + 1, window.hwnd, window.title, window.executable);
            }
        }
        WindowCommand::SetDefault { window } => {
            client.set_default_window(SetDefaultWindowRequest { window }).await?;
            println!("Default window set");
        }
        WindowCommand::GetDefault => {
            let result = client.get_default_window(GetDefaultWindowRequest::default()).await?;
            if let Some(hwnd) = result.hwnd {
                println!("Default window: {} ({})", hwnd, result.title.unwrap_or_default());
            } else {
                println!("No default window set");
            }
        }
    }

    Ok(())
}

async fn connect_to_daemon(port: u16) -> anyhow::Result<DesktopServiceClient> {
    let socket = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).await
        .map_err(|e| anyhow::anyhow!("Failed to connect to daemon on port {}: {}", port, e))?;
    
    let (socket_rx, socket_tx) = socket.into_split();
    
    let (conn, _tx, mut rx): (_, rch::base::Sender<()>, rch::base::Receiver<DesktopServiceClient>) =
        remoc::Connect::io(remoc::Cfg::default(), socket_rx, socket_tx).await?;
    
    tokio::spawn(conn);
    
    rx.recv().await?
        .ok_or_else(|| anyhow::anyhow!("Failed to receive service client"))
}

/// Resolve hwnd from optional CLI arg or get default from daemon
async fn resolve_hwnd(client: &mut DesktopServiceClient, hwnd: Option<String>) -> anyhow::Result<String> {
    if let Some(h) = hwnd {
        Ok(h)
    } else {
        let result = client.get_default_window(GetDefaultWindowRequest::default()).await?;
        result.hwnd.ok_or_else(|| anyhow::anyhow!("No window specified and no default window set. Use 'desktop window set-default <window>' first."))
    }
}

async fn cmd_call(method: CallMethod, port: u16) -> anyhow::Result<()> {
    let mut client = connect_to_daemon(port).await?;
    
    // Call the requested method
    match method {
        CallMethod::Screenshot { hwnd, method } => {
            let hwnd = resolve_hwnd(&mut client, hwnd).await?;
            let result = client.take_screenshot(ScreenshotRequest {
                hwnd,
                method,
            }).await?;
            // Print metadata, not the full base64
            println!("{{\"width\": {}, \"height\": {}, \"format\": \"{}\", \"base64_length\": {}}}", 
                result.width, result.height, result.format, result.base64_image.len());
        }
        CallMethod::Execute { hwnd, instructions, retry_strategy } => {
            let hwnd = resolve_hwnd(&mut client, hwnd).await?;
            let result = client.execute_instructions(ExecuteRequest {
                hwnd,
                instructions,
                retry_strategy,
            }).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        CallMethod::Detect { hwnd, query } => {
            let hwnd = resolve_hwnd(&mut client, hwnd).await?;
            let result = client.detect_elements(DetectRequest {
                hwnd,
                query,
            }).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        
        // UIA Commands
        CallMethod::DumpTree { hwnd, depth, prune_offscreen, prune_empty, max_list_items } => {
            let hwnd = resolve_hwnd(&mut client, hwnd).await?;
            let result = client.dump_tree(DumpTreeRequest {
                hwnd,
                max_depth: Some(depth),
                prune_offscreen: Some(prune_offscreen),
                prune_empty: Some(prune_empty),
                max_list_items: Some(max_list_items),
            }).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        CallMethod::FindElement { hwnd, selector, all, timeout } => {
            let hwnd = resolve_hwnd(&mut client, hwnd).await?;
            let result = client.find_elements(FindElementRequest {
                hwnd,
                selector,
                find_all: Some(all),
                timeout_ms: Some(timeout),
            }).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        CallMethod::Invoke { hwnd, selector, pattern, value } => {
            let hwnd = resolve_hwnd(&mut client, hwnd).await?;
            let result = client.invoke_pattern(InvokePatternRequest {
                hwnd,
                selector,
                pattern,
                value,
            }).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }

        // LLM-Optimized Commands
        CallMethod::Summary { hwnd, format, bounds, paths, region, depth, roles } => {
            let hwnd = resolve_hwnd(&mut client, hwnd).await?;

            // Parse region if provided
            let focus_region = region.as_ref().map(|r| {
                let parts: Vec<i32> = r.split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                if parts.len() == 4 {
                    Some([parts[0], parts[1], parts[2], parts[3]])
                } else {
                    None
                }
            }).flatten();

            // Parse roles if provided
            let roles_vec = roles.map(|r| {
                r.split(',').map(|s| s.trim().to_string()).collect()
            });

            let result = client.get_summary(SummaryRequest {
                hwnd,
                format: Some(format),
                include_bounds: Some(bounds),
                include_paths: Some(paths),
                focus_region,
                max_depth: Some(depth),
                roles: roles_vec,
            }).await?;

            // Output is already formatted by the server based on format option
            println!("{}", result);
        }

        CallMethod::Query { hwnd, query, all, format, timeout } => {
            let hwnd = resolve_hwnd(&mut client, hwnd).await?;
            let result = client.query_elements(QueryRequest {
                hwnd,
                query,
                all: Some(all),
                timeout_ms: Some(timeout),
                format: Some(format),
            }).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }

        CallMethod::Do { hwnd, action, target, value } => {
            let hwnd = resolve_hwnd(&mut client, hwnd).await?;

            // Map action to pattern
            let pattern = match action.to_lowercase().as_str() {
                "click" => "invoke",
                "type" | "input" | "set" => "set-value",
                "toggle" | "check" | "uncheck" => "toggle",
                "expand" | "open" => "expand",
                "collapse" | "close" => "collapse",
                "select" | "choose" => "select",
                "get" | "read" => "get-value",
                _ => &action,
            };

            // First invoke the pattern
            let result = client.invoke_pattern(InvokePatternRequest {
                hwnd: hwnd.clone(),
                selector: target,
                pattern: pattern.to_string(),
                value,
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
    exe_filter: Option<String>,
    title_pattern: Option<String>,
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

    if let Some(ref filter) = exe_filter {
        tracing::info!("Executable filter: {}", filter);
    }
    if let Some(ref pattern) = title_pattern {
        tracing::info!("Title pattern: {}", pattern);
    }

    // Initialize Gemini client
    let gemini_client = gemini::GeminiClient::new(gemini_api_key, gemini_model)?;

    tracing::info!("Gemini client initialized with model: {}", gemini_client.model());

    // Create RPC server configuration
    let server_config = rpc::server::RpcServerConfig {
        port,
        gemini_client,
        allowed_executables,
        exe_filter,
        title_pattern,
    };

    // Start the RPC server
    rpc::start_server(server_config).await?;

    Ok(())
}
