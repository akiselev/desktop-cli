mod agent;
mod automation;
mod error;
mod executor;
mod gemini;
mod ops;
mod rpc;
mod targeting;

use clap::{Parser, Subcommand};
use targeting::{
    format_suggestions, format_window_list, format_window_list_json, resolve_window,
    resolve_with_element, WindowQuery,
};

/// Desktop CLI - Control desktop applications through UI Automation
///
/// A Windows desktop automation tool optimized for LLM agents.
#[derive(Parser, Debug)]
#[command(name = "desktop", author, version, about, long_about = None)]
struct Cli {
    /// Override window target (exe name, title:X, :index, hwnd:X, etc.)
    #[arg(global = true, short = 't', long = "target")]
    target: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List visible windows with query hints
    ///
    /// Shows all visible windows with useful information for targeting.
    /// Use --json for machine-readable output, --suggest for query hints.
    Windows {
        /// Filter by executable name (substring match)
        #[arg(long)]
        exe: Option<String>,

        /// Filter by window title (substring match)
        #[arg(long)]
        title: Option<String>,

        /// Output as JSON (for agents)
        #[arg(long)]
        json: bool,

        /// Show query suggestions for this HWND
        #[arg(long)]
        suggest: Option<String>,
    },

    /// Get a compact UI summary optimized for LLM consumption
    ///
    /// Returns categorized elements (actions, navigation, content) with
    /// minimal noise. Use this after every action to understand UI state.
    Summary {
        /// Window query (e.g., "notepad", ":1", "title:PCB")
        window: Option<String>,

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
    ///   @tab:nth(2)           - Second tab
    Query {
        /// Window query
        window: String,

        /// Element selector
        selector: String,

        /// Return all matches (default: first only)
        #[arg(long)]
        all: bool,

        /// Output format: full, compact, refs
        #[arg(long, default_value = "compact")]
        format: String,
    },

    /// Click an element
    ///
    /// Examples:
    ///   click notepad "@button 'Save'"
    ///   click altium "Button[name='Compile']"
    ///   click :1 --coords 100,200
    Click {
        /// Window query
        window: String,

        /// Element selector (or use --coords)
        selector: Option<String>,

        /// Click type: left (default), right, double
        #[arg(long, short = 'k', default_value = "left")]
        kind: String,

        /// Click at coordinates: x,y (instead of selector)
        #[arg(long, short = 'c')]
        coords: Option<String>,
    },

    /// Type text into an element
    ///
    /// Examples:
    ///   type notepad "#editor" --value "Hello World"
    ///   type altium "@input 'Name'" --value "Component1"
    Type {
        /// Window query
        window: String,

        /// Element selector (to focus before typing)
        selector: String,

        /// Text to type
        #[arg(long)]
        value: String,
    },

    /// Send key combination
    ///
    /// Examples:
    ///   keys notepad "ctrl+s"
    ///   keys :1 "alt+f4"
    Keys {
        /// Window query
        window: String,

        /// Key combination (e.g., "ctrl+c", "alt+f4", "enter")
        keys: String,
    },

    /// Scroll up or down
    ///
    /// Examples:
    ///   scroll notepad up
    ///   scroll :1 down --amount 5
    Scroll {
        /// Window query
        window: String,

        /// Direction: up or down
        direction: String,

        /// Number of scroll notches (default: 3)
        #[arg(long, short = 'n', default_value = "3")]
        amount: i32,
    },

    /// Take a screenshot of a window
    Screenshot {
        /// Window query
        window: String,

        /// Screenshot method (bitblt or printwindow)
        #[arg(long)]
        method: Option<String>,
    },

    /// Dump the UIA element tree for a window
    DumpTree {
        /// Window query
        window: String,

        /// Maximum depth
        #[arg(long, default_value = "5")]
        depth: u32,
    },

    /// Find UI elements by CSS-style selector
    FindElement {
        /// Window query
        window: String,

        /// CSS-style selector (e.g., "Button#save", "[name~='*OK*']")
        selector: String,

        /// Find all matches (default: first only)
        #[arg(long)]
        all: bool,
    },

    /// Invoke a UIA pattern operation on an element
    Invoke {
        /// Window query
        window: String,

        /// CSS-style selector to find target element
        selector: String,

        /// Pattern operation (invoke, get-value, set-value, toggle, select, expand, collapse)
        #[arg(long)]
        pattern: String,

        /// Value for set operations
        #[arg(long)]
        value: Option<String>,
    },

    /// Perform an action on an element (combined query + invoke)
    ///
    /// Combines finding and acting on an element in one call.
    Do {
        /// Window query
        window: String,

        /// Action: click, type, toggle, expand, collapse, select
        action: String,

        /// Target element query (e.g., @button "Save", #inputField)
        target: String,

        /// Value for type/set operations
        #[arg(long)]
        value: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Windows {
            exe,
            title,
            json,
            suggest,
        } => {
            cmd_windows(exe, title, json, suggest)?;
        }

        Commands::Summary {
            window,
            format,
            bounds,
            paths,
            region,
            depth,
            roles,
        } => {
            let hwnd = resolve_target(window.as_deref(), None, cli.target.as_deref())?;
            let focus_region = parse_region(&region);
            let roles_vec = roles.map(|r| r.split(',').map(|s| s.trim().to_string()).collect());

            let result =
                ops::get_summary(&hwnd, &format, bounds, paths, focus_region, depth, roles_vec)?;
            println!("{}", result);
        }

        Commands::Query {
            window,
            selector,
            all,
            format: _,
        } => {
            let hwnd =
                resolve_target_with_element(&window, &selector, cli.target.as_deref())?;
            let result = ops::query_elements(&hwnd, &selector, all)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }

        Commands::Click {
            window,
            selector,
            kind,
            coords,
        } => {
            let coords_parsed = parse_coords(&coords);

            // If coordinates specified, just use window resolution
            // If selector specified, use element-aware resolution
            let hwnd = if coords_parsed.is_some() || selector.is_none() {
                resolve_target(Some(&window), None, cli.target.as_deref())?
            } else {
                resolve_target_with_element(
                    &window,
                    selector.as_deref().unwrap_or(""),
                    cli.target.as_deref(),
                )?
            };

            ops::click(&hwnd, &kind, coords_parsed, selector.as_deref())?;
            println!("Click successful");
        }

        Commands::Type {
            window,
            selector,
            value,
        } => {
            let hwnd =
                resolve_target_with_element(&window, &selector, cli.target.as_deref())?;
            ops::type_text(&hwnd, &value, Some(&selector))?;
            println!("Text typed successfully");
        }

        Commands::Keys { window, keys } => {
            let _hwnd = resolve_target(Some(&window), None, cli.target.as_deref())?;
            ops::send_keys(&keys)?;
            println!("Keys sent successfully");
        }

        Commands::Scroll {
            window,
            direction,
            amount,
        } => {
            let _hwnd = resolve_target(Some(&window), None, cli.target.as_deref())?;
            ops::scroll(&direction, amount)?;
            println!("Scroll successful");
        }

        Commands::Screenshot { window, method } => {
            let hwnd = resolve_target(Some(&window), None, cli.target.as_deref())?;
            let result = ops::take_screenshot(&hwnd, method.as_deref())?;
            println!(
                "{{\"width\": {}, \"height\": {}, \"format\": \"{}\", \"base64_length\": {}}}",
                result.width,
                result.height,
                result.format,
                result.base64_image.len()
            );
        }

        Commands::DumpTree { window, depth } => {
            let hwnd = resolve_target(Some(&window), None, cli.target.as_deref())?;
            let result = ops::dump_tree(&hwnd, depth)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }

        Commands::FindElement {
            window,
            selector,
            all,
        } => {
            let hwnd =
                resolve_target_with_element(&window, &selector, cli.target.as_deref())?;
            let result = ops::find_elements(&hwnd, &selector, all)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }

        Commands::Invoke {
            window,
            selector,
            pattern,
            value,
        } => {
            let hwnd =
                resolve_target_with_element(&window, &selector, cli.target.as_deref())?;
            let result = ops::invoke_pattern(&hwnd, &selector, &pattern, value.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }

        Commands::Do {
            window,
            action,
            target,
            value,
        } => {
            let hwnd =
                resolve_target_with_element(&window, &target, cli.target.as_deref())?;

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

            let result = ops::invoke_pattern(&hwnd, &target, pattern, value.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}

// ============================================================================
// Command Implementations
// ============================================================================

fn cmd_windows(
    exe: Option<String>,
    title: Option<String>,
    json: bool,
    suggest: Option<String>,
) -> anyhow::Result<()> {
    let windows = ops::list_windows(exe.as_deref(), title.as_deref())?;

    if let Some(hwnd) = suggest {
        // Show suggestions for specific window
        match format_suggestions(&hwnd, &windows) {
            Some(output) => println!("{}", output),
            None => eprintln!("Window with HWND {} not found", hwnd),
        }
    } else if json {
        // JSON output for agents
        let output = format_window_list_json(&windows);
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        // Human-readable list
        println!("{}", format_window_list(&windows));
    }

    Ok(())
}

// ============================================================================
// Target Resolution Helpers
// ============================================================================

/// Resolve window target, optionally with element disambiguation
fn resolve_target(
    window_arg: Option<&str>,
    _element_selector: Option<&str>,
    flag_target: Option<&str>,
) -> anyhow::Result<String> {
    // Priority: window_arg > flag > env var
    let query_str = window_arg
        .or(flag_target)
        .or_else(|| std::env::var("DESKTOP_WINDOW").ok().as_deref().map(|_| {
            // This closure doesn't work well, handle separately
            ""
        }))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No window specified. Use a window query or set DESKTOP_WINDOW env var.\n\
                 Run 'desktop windows' to see available windows."
            )
        })?;

    // Check env var if nothing else set
    let query_str = if query_str.is_empty() {
        std::env::var("DESKTOP_WINDOW")
            .map_err(|_| anyhow::anyhow!("No window specified"))?
    } else {
        query_str.to_string()
    };

    let query = WindowQuery::parse(&query_str)
        .map_err(|e| anyhow::anyhow!("Invalid window query: {}", e))?;

    let windows = ops::list_windows(None, None)?;

    match resolve_window(&query, &windows) {
        Ok(window) => Ok(window.hwnd.clone()),
        Err(targeting::ResolutionError::AmbiguousWindow { query, windows }) => {
            Err(format_ambiguous_error(&query, &windows))
        }
        Err(e) => Err(anyhow::anyhow!("{}", e)),
    }
}

/// Resolve with element-aware disambiguation
fn resolve_target_with_element(
    window_arg: &str,
    element_selector: &str,
    flag_target: Option<&str>,
) -> anyhow::Result<String> {
    let query_str = flag_target.unwrap_or(window_arg);

    let query = WindowQuery::parse(query_str)
        .map_err(|e| anyhow::anyhow!("Invalid window query: {}", e))?;

    let windows = ops::list_windows(None, None)?;

    // Use element-aware resolution
    match resolve_with_element(&query, element_selector, &windows, |hwnd, selector| {
        ops::element_exists(hwnd, selector).map_err(|e| e.to_string())
    }) {
        Ok(window) => Ok(window.hwnd.clone()),
        Err(targeting::ResolutionError::AmbiguousWindow { query, windows }) => {
            Err(format_ambiguous_error(&query, &windows))
        }
        Err(targeting::ResolutionError::AmbiguousElement {
            selector,
            windows,
        }) => {
            let mut msg = format!(
                "Found '{}' in {} windows:\n",
                selector,
                windows.len()
            );
            for (i, w) in windows.iter().enumerate() {
                msg.push_str(&format!(
                    "  [{}] {} - {} (hwnd:{})\n",
                    i + 1,
                    extract_exe_name(&w.executable),
                    w.title,
                    w.hwnd
                ));
            }
            msg.push_str("Tip: Use ':1' or refine with 'title:...'");
            Err(anyhow::anyhow!("{}", msg))
        }
        Err(e) => Err(anyhow::anyhow!("{}", e)),
    }
}

fn format_ambiguous_error(
    query: &str,
    windows: &[automation::types::WindowInfo],
) -> anyhow::Error {
    let mut msg = format!("Found {} windows matching '{}':\n", windows.len(), query);
    for (i, w) in windows.iter().enumerate() {
        msg.push_str(&format!(
            "  [:{}] {} - {} (hwnd:{}, pid:{})\n",
            i + 1,
            extract_exe_name(&w.executable),
            w.title,
            w.hwnd,
            w.pid
        ));
    }
    msg.push_str("Tip: Use ':1', ':2', etc. or refine with 'title:...'");
    anyhow::anyhow!("{}", msg)
}

fn extract_exe_name(exe_path: &str) -> String {
    exe_path
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(exe_path)
        .trim_end_matches(".exe")
        .trim_end_matches(".EXE")
        .to_lowercase()
}

// ============================================================================
// Parsing Helpers
// ============================================================================

fn parse_region(region: &Option<String>) -> Option<[i32; 4]> {
    region.as_ref().and_then(|r| {
        let parts: Vec<i32> = r
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        if parts.len() == 4 {
            Some([parts[0], parts[1], parts[2], parts[3]])
        } else {
            None
        }
    })
}

fn parse_coords(coords: &Option<String>) -> Option<(i32, i32)> {
    coords.as_ref().and_then(|c| {
        let parts: Vec<i32> = c
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        if parts.len() == 2 {
            Some((parts[0], parts[1]))
        } else {
            None
        }
    })
}
