use msui_core::MsuiConfig;
use std::env;

fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("bar");

    match command {
        "bar" => {
            tracing::info!("Starting msUI Top Bar...");
            let config = MsuiConfig::load_or_default();
            if let Err(e) = msui_bar::run_bar(config) {
                tracing::error!("msUI Bar exited with error: {:?}", e);
            }
        }
        "launcher" => {
            tracing::info!("Starting msUI Application Launcher...");
            let config = MsuiConfig::load_or_default();
            if let Err(e) = msui_launcher::run_launcher(config) {
                tracing::error!("msUI Launcher exited with error: {:?}", e);
            }
        }
        "power" => {
            tracing::info!("Starting msUI Power & Session Menu...");
            if let Err(e) = msui_launcher::run_power_menu() {
                tracing::error!("msUI Power Menu exited with error: {:?}", e);
            }
        }
        "--help" | "-h" | "help" => {
            print_help();
        }
        "--version" | "-v" => {
            println!("msUI v{}", env!("CARGO_PKG_VERSION"));
        }
        other => {
            eprintln!("Unknown command: '{}'\n", other);
            print_help();
            std::process::exit(1);
        }
    }

    Ok(())
}

fn print_help() {
    println!(
        "msUI - High-performance Wayland Desktop Shell in Rust + Iced\n\n\
        USAGE:\n  \
          msui [COMMAND]\n\n\
        COMMANDS:\n  \
          bar         Run the Wayland Layer Shell status bar (default)\n  \
          launcher    Open the Spotlight application launcher overlay\n  \
          power       Open the Power & Session menu overlay\n  \
          help        Display this help message\n\n\
        OPTIONS:\n  \
          -h, --help     Print help information\n  \
          -v, --version  Print version information"
    );
}
