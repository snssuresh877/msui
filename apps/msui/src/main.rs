use msui_core::{MsuiConfig, ThemeManager, WallpaperManager};
use std::env;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("bar");

    match command {
        "bar" => {
            tracing::info!("Starting msUI Top Bar & Background Services...");
            // Start background Tokio runtime for clipboard watcher and D-Bus notifications
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create Tokio runtime");

                rt.block_on(async {
                    tokio::spawn(msui_core::start_clipboard_watcher());
                    let _ = msui_core::run_notification_service().await;
                });
            });

            let config = MsuiConfig::load_or_default();
            if let Err(e) = msui_bar::run_bar(config) {
                tracing::error!("msUI Bar exited with error: {:?}", e);
            }
        }
        "clipboard" => {
            tracing::info!("Starting msUI Clipboard History Manager...");
            if let Err(e) = msui_launcher::run_clipboard_menu() {
                tracing::error!("msUI Clipboard Menu exited with error: {:?}", e);
            }
        }
        "toast" => {
            tracing::info!("Starting msUI Toast Notification Window...");
            if let Err(e) = msui_launcher::run_toast_window() {
                tracing::error!("msUI Toast Window exited with error: {:?}", e);
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
        "notifications" | "notifs" => {
            tracing::info!("Starting msUI Notification Center...");
            if let Err(e) = msui_launcher::run_notification_center() {
                tracing::error!("msUI Notification Center exited with error: {:?}", e);
            }
        }
        "bluetooth" | "bt" => {
            tracing::info!("Starting msUI Bluetooth Manager...");
            if let Err(e) = msui_launcher::run_bluetooth_menu() {
                tracing::error!("msUI Bluetooth Menu exited with error: {:?}", e);
            }
        }
        "wallpaper" | "wall" => {
            let sub = args.get(2).map(|s| s.as_str());
            match sub {
                Some("cycle" | "next") => {
                    if let Some(next) = WallpaperManager::cycle() {
                        println!("Wallpaper switched to: {}", next.display());
                    }
                }
                Some("random") => {
                    if let Some(rand) = WallpaperManager::random() {
                        println!("Random wallpaper set: {}", rand.display());
                    }
                }
                Some("set") => {
                    if let Some(path_str) = args.get(3) {
                        let path = Path::new(path_str);
                        if WallpaperManager::set(path) {
                            println!("Wallpaper set to: {}", path.display());
                        } else {
                            eprintln!("File does not exist: {}", path.display());
                            std::process::exit(1);
                        }
                    } else {
                        eprintln!("Usage: msui wallpaper set <path>");
                        std::process::exit(1);
                    }
                }
                Some("current") => {
                    if let Some(cur) = WallpaperManager::get_current() {
                        println!("{}", cur.display());
                    } else {
                        println!("No wallpaper currently set.");
                    }
                }
                _ => {
                    tracing::info!("Starting msUI Wallpaper Selector...");
                    if let Err(e) = msui_launcher::run_wallpaper_menu() {
                        tracing::error!("msUI Wallpaper Menu exited with error: {:?}", e);
                    }
                }
            }
        }
        "bg" | "wallpaper-daemon" => {
            tracing::info!("Starting msUI Native Wallpaper Background Daemon...");
            if let Err(e) = msui_launcher::run_wallpaper_daemon() {
                tracing::error!("msUI Wallpaper Daemon exited with error: {:?}", e);
            }
        }
        "theme" | "themes" => {
            if let Some(name) = args.get(2) {
                if ThemeManager::set_theme(name) {
                    println!("Theme switched to: {}", name);
                } else {
                    eprintln!("Failed to set theme: {}", name);
                    std::process::exit(1);
                }
            } else {
                tracing::info!("Starting msUI Theme Selector...");
                if let Err(e) = msui_launcher::run_theme_menu() {
                    tracing::error!("msUI Theme Menu exited with error: {:?}", e);
                }
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
          bar           Run the Wayland Layer Shell status bar & background daemons (default)\n  \
          launcher      Open the Spotlight application launcher overlay\n  \
          clipboard     Open the Spotlight clipboard history manager\n  \
          notifications Open the Notification Center history overlay\n  \
          bluetooth     Open the Bluetooth settings manager overlay\n  \
          wallpaper     Open the Wallpaper changer overlay (or 'cycle', 'random', 'set <path>')\n  \
          bg            Run the native Wayland background wallpaper daemon\n  \
          theme         Open the Theme switcher overlay (or pass theme name)\n  \
          power         Open the Power & Session menu overlay\n  \
          toast         Run the Toast notifications overlay\n  \
          help          Display this help message\n\n\
        OPTIONS:\n  \
          -h, --help     Print help information\n  \
          -v, --version  Print version information"
    );
}
