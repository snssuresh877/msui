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
        "wifi" | "network" => {
            let sub = args.get(2).map(|s| s.as_str());
            match sub {
                Some("list") => {
                    let networks = msui_core::NetworkMonitor::scan_wifi();
                    println!("{:<4} {:<32} {:<10} {:<10}", "USE", "SSID", "SIGNAL", "SECURITY");
                    println!("{}", "-".repeat(60));
                    for net in networks {
                        let active = if net.is_connected { "*" } else { " " };
                        let sec = if net.is_secure { &net.security } else { "Open" };
                        println!("{:<4} {:<32} {:<10} {:<10}", active, net.ssid, format!("{}% {}", net.signal, net.bars), sec);
                    }
                }
                Some("rescan") => {
                    msui_core::NetworkMonitor::rescan_wifi();
                    println!("Triggered Wi-Fi rescan.");
                }
                Some("disconnect") => {
                    if msui_core::NetworkMonitor::disconnect_wifi() {
                        println!("Disconnected from Wi-Fi.");
                    } else {
                        eprintln!("Failed to disconnect.");
                        std::process::exit(1);
                    }
                }
                Some("connect") => {
                    if let Some(ssid) = args.get(3) {
                        let password = args.get(4).map(|s| s.as_str());
                        match msui_core::NetworkMonitor::connect_wifi(ssid, password) {
                            Ok(()) => println!("Connected to \"{}\"", ssid),
                            Err(e) => {
                                eprintln!("Connection error: {}", e);
                                std::process::exit(1);
                            }
                        }
                    } else {
                        eprintln!("Usage: msui wifi connect <SSID> [password]");
                        std::process::exit(1);
                    }
                }
                _ => {
                    tracing::info!("Starting msUI Wi-Fi Network Manager...");
                    if let Err(e) = msui_launcher::run_wifi_menu() {
                        tracing::error!("msUI Wi-Fi Menu exited with error: {:?}", e);
                    }
                }
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
        "audio" | "mixer" => {
            let sub = args.get(2).map(|s| s.as_str());
            match sub {
                Some("sinks") => {
                    let sinks = msui_core::AudioMonitor::list_sinks();
                    println!("{:<4} {:<4} {:<6} {:<6} {:<32} {}", "USE", "IDX", "VOL", "MUTE", "NAME", "DESCRIPTION");
                    println!("{}", "-".repeat(75));
                    for s in sinks {
                        let active = if s.is_default { "*" } else { " " };
                        let mute = if s.is_muted { "yes" } else { "no" };
                        println!("{:<4} {:<4} {:<6} {:<6} {:<32} {}", active, s.index, format!("{}%", s.volume_percent), mute, s.name, s.description);
                    }
                }
                Some("sources") => {
                    let sources = msui_core::AudioMonitor::list_sources();
                    println!("{:<4} {:<4} {:<6} {:<6} {:<32} {}", "USE", "IDX", "VOL", "MUTE", "NAME", "DESCRIPTION");
                    println!("{}", "-".repeat(75));
                    for s in sources {
                        let active = if s.is_default { "*" } else { " " };
                        let mute = if s.is_muted { "yes" } else { "no" };
                        println!("{:<4} {:<4} {:<6} {:<6} {:<32} {}", active, s.index, format!("{}%", s.volume_percent), mute, s.name, s.description);
                    }
                }
                Some("set-sink") => {
                    if let Some(name) = args.get(3) {
                        msui_core::AudioMonitor::set_default_sink(name);
                        println!("Default audio output set to: {}", name);
                    } else {
                        eprintln!("Usage: msui audio set-sink <name-or-index>");
                        std::process::exit(1);
                    }
                }
                Some("set-source") => {
                    if let Some(name) = args.get(3) {
                        msui_core::AudioMonitor::set_default_source(name);
                        println!("Default audio input set to: {}", name);
                    } else {
                        eprintln!("Usage: msui audio set-source <name-or-index>");
                        std::process::exit(1);
                    }
                }
                Some("volume") => {
                    if let Some(val_str) = args.get(3) {
                        if let Ok(pct) = val_str.parse::<u8>() {
                            msui_core::AudioMonitor::set_volume(pct);
                            println!("Default audio volume set to: {}%", pct);
                        } else {
                            eprintln!("Invalid volume percentage: {}", val_str);
                            std::process::exit(1);
                        }
                    } else {
                        eprintln!("Usage: msui audio volume <0-100>");
                        std::process::exit(1);
                    }
                }
                Some("mute") => {
                    msui_core::AudioMonitor::toggle_mute();
                    println!("Toggled audio mute.");
                }
                _ => {
                    tracing::info!("Starting msUI Audio Mixer & Device Manager...");
                    if let Err(e) = msui_launcher::run_audio_menu() {
                        tracing::error!("msUI Audio Menu exited with error: {:?}", e);
                    }
                }
            }
        }
        "battery" | "power-profile" | "profile" => {
            let sub = args.get(2).map(|s| s.as_str());
            match sub {
                Some("status" | "info") => {
                    let monitor = msui_core::BatteryMonitor::new();
                    if let Some(bat) = monitor.read() {
                        let active_prof = msui_core::PowerProfileManager::get_profile();
                        println!("msUI Battery Telemetry & Diagnostics");
                        println!("{}", "=".repeat(45));
                        println!("{:<22}: {}% ({})", "Battery Charge", bat.percentage, bat.status.as_str());
                        println!("{:<22}: {}", "AC Adapter", if bat.is_ac_online { "Online (Plugged In)" } else { "Offline (On Battery)" });
                        println!("{:<22}: {:.2} W", "Power Draw", bat.power_watts);
                        println!("{:<22}: {:.2} V", "Voltage", bat.voltage_volts);
                        println!("{:<22}: {:.2} Wh / {:.2} Wh", "Current / Full Energy", bat.energy_now_wh, bat.energy_full_wh);
                        println!("{:<22}: {:.2} Wh", "Design Capacity", bat.energy_full_design_wh);
                        println!("{:<22}: {:.1}%", "Battery Health", bat.health_percent);
                        if let Some(cycles) = bat.cycle_count {
                            println!("{:<22}: {}", "Cycle Count", cycles);
                        }
                        println!("{:<22}: {}", "Technology", bat.technology);
                        if let Some(mins) = bat.time_remaining_mins {
                            let h = mins / 60;
                            let m = mins % 60;
                            println!("{:<22}: ~{}h {}m", "Estimated Time", h, m);
                        }
                        println!("{:<22}: {} ({})", "Active Power Profile", active_prof.display_name(), active_prof.as_str());
                    } else {
                        println!("No battery detected on this system (Desktop / AC Only).");
                    }
                }
                Some("profile") => {
                    let sub2 = args.get(3).map(|s| s.as_str());
                    match sub2 {
                        Some("set") => {
                            if let Some(p_str) = args.get(4) {
                                if let Some(prof) = msui_core::PowerProfile::from_str_loose(p_str) {
                                    if msui_core::PowerProfileManager::set_profile(prof) {
                                        println!("Power profile switched to: {} ({})", prof.display_name(), prof.as_str());
                                    } else {
                                        eprintln!("Failed to switch power profile to: {}", p_str);
                                        std::process::exit(1);
                                    }
                                } else {
                                    eprintln!("Invalid profile '{}'. Choose: power-saver, balanced, performance", p_str);
                                    std::process::exit(1);
                                }
                            } else {
                                eprintln!("Usage: msui battery profile set <power-saver|balanced|performance>");
                                std::process::exit(1);
                            }
                        }
                        _ => {
                            let current = msui_core::PowerProfileManager::get_profile();
                            println!("Active Power Profile: {} ({})", current.display_name(), current.as_str());
                            println!("Available: power-saver, balanced, performance");
                        }
                    }
                }
                Some("set") => {
                    if let Some(p_str) = args.get(3) {
                        if let Some(prof) = msui_core::PowerProfile::from_str_loose(p_str) {
                            if msui_core::PowerProfileManager::set_profile(prof) {
                                println!("Power profile switched to: {} ({})", prof.display_name(), prof.as_str());
                            } else {
                                eprintln!("Failed to switch power profile to: {}", p_str);
                                std::process::exit(1);
                            }
                        } else {
                            eprintln!("Invalid profile '{}'. Choose: power-saver, balanced, performance", p_str);
                            std::process::exit(1);
                        }
                    } else {
                        eprintln!("Usage: msui battery set <power-saver|balanced|performance>");
                        std::process::exit(1);
                    }
                }
                _ => {
                    tracing::info!("Starting msUI Battery & Power Profile Manager...");
                    if let Err(e) = msui_launcher::run_battery_menu() {
                        tracing::error!("msUI Battery Menu exited with error: {:?}", e);
                    }
                }
            }
        }
        "display" | "monitors" | "nightlight" => {
            let sub = args.get(2).map(|s| s.as_str());
            match sub {
                Some("list") => {
                    let monitors = msui_core::DisplayManager::list_monitors();
                    println!("{:<4} {:<8} {:<18} {:<8} {:<10} {}", "IDX", "NAME", "RESOLUTION", "SCALE", "ROTATION", "MODEL");
                    println!("{}", "-".repeat(70));
                    for m in monitors {
                        let res = format!("{}x{}@{:.0}Hz", m.width, m.height, m.refresh_rate);
                        let rot = match m.transform { 1 => "90°", 2 => "180°", 3 => "270°", _ => "0°" };
                        let scale = format!("{:.0}%", m.scale * 100.0);
                        println!("{:<4} {:<8} {:<18} {:<8} {:<10} {}", m.id, m.name, res, scale, rot, m.description);
                    }
                }
                Some("scale") => {
                    if let (Some(name), Some(sc_str)) = (args.get(3), args.get(4)) {
                        if let Ok(sc) = sc_str.parse::<f32>() {
                            if msui_core::DisplayManager::set_scale(name, sc) {
                                println!("Set scale on {} to {:.2}x", name, sc);
                            } else {
                                eprintln!("Failed to set scale on {}", name);
                                std::process::exit(1);
                            }
                        } else {
                            eprintln!("Invalid scale number: {}", sc_str);
                            std::process::exit(1);
                        }
                    } else {
                        eprintln!("Usage: msui display scale <output> <1.0|1.25|1.5|2.0>");
                        std::process::exit(1);
                    }
                }
                Some("rotate") => {
                    if let (Some(name), Some(deg_str)) = (args.get(3), args.get(4)) {
                        if let Ok(deg) = deg_str.parse::<u32>() {
                            if msui_core::DisplayManager::set_rotation(name, deg) {
                                println!("Set rotation on {} to {}°", name, deg);
                            } else {
                                eprintln!("Failed to set rotation on {}", name);
                                std::process::exit(1);
                            }
                        } else {
                            eprintln!("Invalid degrees: {}. Choose 0, 90, 180, 270", deg_str);
                            std::process::exit(1);
                        }
                    } else {
                        eprintln!("Usage: msui display rotate <output> <0|90|180|270>");
                        std::process::exit(1);
                    }
                }
                Some("nightlight") => {
                    let sub2 = args.get(3).map(|s| s.as_str());
                    match sub2 {
                        Some("on") => {
                            let temp = msui_core::NightLightManager::get_temperature();
                            msui_core::NightLightManager::enable(temp);
                            println!("Night light enabled ({}K)", temp);
                        }
                        Some("off") => {
                            msui_core::NightLightManager::disable();
                            println!("Night light disabled");
                        }
                        Some("toggle") => {
                            if msui_core::NightLightManager::toggle() {
                                println!("Night light enabled");
                            } else {
                                println!("Night light disabled");
                            }
                        }
                        Some(val) if val.parse::<u32>().is_ok() => {
                            let temp = val.parse::<u32>().unwrap();
                            msui_core::NightLightManager::enable(temp);
                            println!("Night light temperature set to: {}K", temp);
                        }
                        _ => {
                            let is_on = msui_core::NightLightManager::is_active();
                            let temp = msui_core::NightLightManager::get_temperature();
                            println!("Night Light Status: {}", if is_on { "Active (ON)" } else { "Inactive (OFF)" });
                            println!("Configured Warmth : {}K", temp);
                            println!("Usage: msui display nightlight <on|off|toggle|<kelvin>>");
                        }
                    }
                }
                _ => {
                    tracing::info!("Starting msUI Display & Monitor Manager...");
                    if let Err(e) = msui_launcher::run_display_menu() {
                        tracing::error!("msUI Display Menu exited with error: {:?}", e);
                    }
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
          audio         Open the Audio Mixer & Device Switcher overlay (or 'sinks', 'sources', 'set-sink', 'set-source', 'volume', 'mute')\n  \
          battery       Open the Battery Diagnostics & Power Profile Manager (or 'status', 'profile set')\n  \
          display       Open the Display, Multi-Monitor & Night Light manager overlay (or 'list', 'scale', 'rotate', 'nightlight')\n  \
          wifi          Open the Wi-Fi network manager overlay (or 'list', 'connect', 'disconnect')\n  \
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
