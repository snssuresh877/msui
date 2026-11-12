mod daemon;

use msui_core::ipc::{IpcClient, IpcCommand};
use msui_core::{
    AiConfig, LlamaRuntime, MemorySentinel, ModelStore, MsuiConfig, ReminderManager, ThemeManager,
    WallpaperManager,
};
use std::env;
use std::path::Path;

pub(crate) struct ModalManager {
    current: Option<(String, tokio::process::Child)>,
}

impl ModalManager {
    pub(crate) fn new() -> Self {
        Self { current: None }
    }

    pub(crate) async fn handle_command(&mut self, cmd: IpcCommand) {
        let is_embedded = |target: &str| -> bool {
            matches!(
                target,
                "bar" | "wallpaper" | "wall" | "dnd" | "launcher" | "power"
                    | "control" | "quicksettings" | "cc" | "topbar" | "bar-settings"
                    | "clipboard" | "clip"
                    | "notifications" | "notifs"
                    | "drawer" | "tasks" | "apps" | "appdrawer"
                    | "wifi" | "network"
                    | "bluetooth" | "bt"
                    | "audio" | "mixer"
                    | "theme" | "themes"
                    | "fshare" | "share" | "fileshare"
                    | "battery" | "bat"
                    | "display" | "disp" | "monitors"
                    | "media" | "player"
                    | "storage" | "disks"
                    | "reminder" | "reminders"
                    | "settings" | "systemcontrol" | "control-panel"
            )
        };

        match cmd {
            IpcCommand::Toggle(target) => {
                if is_embedded(&target) {
                    return;
                }

                if let Some((current_target, mut child)) = self.current.take() {
                    match child.try_wait() {
                        Ok(None) => {
                            let _ = child.kill().await;
                            let _ = child.wait().await;
                            if current_target == target {
                                return;
                            }
                        }
                        _ => {}
                    }
                }

                let bin = env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                let mut cmd = tokio::process::Command::new(bin);
                cmd.arg(&target);
                cmd.arg("--direct");
                match cmd.spawn() {
                    Ok(child) => {
                        self.current = Some((target, child));
                    }
                    Err(e) => {
                        tracing::error!("Failed to spawn modal {}: {:?}", target, e);
                    }
                }
            }
            IpcCommand::Open(target) => {
                if is_embedded(&target) {
                    return;
                }

                if let Some((current_target, mut child)) = self.current.take() {
                    match child.try_wait() {
                        Ok(None) => {
                            if current_target == target {
                                self.current = Some((current_target, child));
                                return;
                            }
                            let _ = child.kill().await;
                            let _ = child.wait().await;
                        }
                        _ => {}
                    }
                }

                let bin = env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                let mut cmd = tokio::process::Command::new(bin);
                cmd.arg(&target);
                cmd.arg("--direct");
                if let Ok(child) = cmd.spawn() {
                    self.current = Some((target, child));
                }
            }
            IpcCommand::Close => {
                if let Some((_, mut child)) = self.current.take() {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                }
            }
            IpcCommand::DndToggle => {
                msui_core::DndManager::toggle_dnd();
            }
            IpcCommand::DndSet(on) => {
                msui_core::DndManager::set_dnd(on);
            }
            IpcCommand::WallpaperCycle => {
                WallpaperManager::cycle();
            }
            IpcCommand::NotificationAction { id, action } => {
                tokio::spawn(async move {
                    let _ = msui_core::emit_action_invoked(id, &action).await;
                    let _ = msui_core::emit_notification_closed(id, 2).await;
                });
            }
            IpcCommand::NotificationClose { id, reason } => {
                tokio::spawn(async move {
                    let _ = msui_core::emit_notification_closed(id, reason).await;
                });
            }
            IpcCommand::Ping | IpcCommand::Osd { .. } | IpcCommand::ReloadConfig | IpcCommand::ClipboardAdd(_) | IpcCommand::Unknown(_) => {}
        }
    }
}

fn handle_clipboard_ingest() {
    use std::io::Read;
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_ok() {
        if input.trim().is_empty() {
            return;
        }
        // Check if password manager hint is active; if so, skip storing
        if msui_core::is_password_hint_active() {
            return;
        }
        // Attempt sending via IPC to running msUI daemon
        if msui_core::IpcClient::send_blocking(&msui_core::IpcCommand::ClipboardAdd(input.clone())).is_err() {
            // Fallback: update local clipboard store directly
            let mut store = msui_core::GLOBAL_CLIPBOARD.lock().unwrap();
            store.add(input);
        }
    }
}

fn try_ipc_toggle(target: &str) -> bool {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--direct") {
        return false;
    }
    match IpcClient::send_blocking(&IpcCommand::Toggle(target.to_string())) {
        Ok(resp) => {
            tracing::debug!("IPC toggle {} success: {}", target, resp);
            true
        }
        Err(_) => false,
    }
}

fn main() -> anyhow::Result<()> {
    // Initialize tracing
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("daemon");

    match command {
        "daemon" | "run" => {
            let config = MsuiConfig::load_or_default();
            daemon::run_daemon(config)?;
        }
        "bar" => {
            let sub = args.get(2).map(|s| s.as_str());
            if args.iter().any(|a| a == "--direct" || a == "--standalone") {
                let config = MsuiConfig::load_or_default();
                return msui_bar::run_bar(config).map_err(|e| anyhow::anyhow!("{:?}", e));
            }
            match sub {
                Some("toggle") | None => {
                    match IpcClient::send_blocking(&IpcCommand::Toggle("bar".to_string())) {
                        Ok(res) => {
                            println!("{}", res);
                            return Ok(());
                        }
                        Err(_) => {
                            tracing::info!("msUI daemon not running; launching unified daemon for bar...");
                            let config = MsuiConfig::load_or_default();
                            daemon::run_daemon(config)?;
                        }
                    }
                }
                Some("open") => {
                    match IpcClient::send_blocking(&IpcCommand::Open("bar".to_string())) {
                        Ok(res) => {
                            println!("{}", res);
                            return Ok(());
                        }
                        Err(_) => {
                            let config = MsuiConfig::load_or_default();
                            daemon::run_daemon(config)?;
                        }
                    }
                }
                Some("close") => {
                    match IpcClient::send_blocking(&IpcCommand::Close) {
                        Ok(res) => {
                            println!("{}", res);
                            return Ok(());
                        }
                        Err(e) => {
                            eprintln!("msUI daemon not running: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Some("glass" | "transparency") => {
                    let mode = args.get(3).map(|s| s.as_str()).unwrap_or("frosted");
                    let mut config = MsuiConfig::load_or_default();
                    config.bar.transparency = mode.to_string();
                    config.save_to_default_path()?;
                    let _ = IpcClient::send_blocking(&IpcCommand::ReloadConfig);
                    println!("Top bar transparency set to '{}'", mode);
                    return Ok(());
                }
                Some("style") => {
                    let style = args.get(3).map(|s| s.as_str()).unwrap_or("modern");
                    let mut config = MsuiConfig::load_or_default();
                    config.bar.style = style.to_string();
                    config.save_to_default_path()?;
                    let _ = IpcClient::send_blocking(&IpcCommand::ReloadConfig);
                    println!("Top bar appearance style set to '{}'", style);
                    return Ok(());
                }
                Some(other) => {
                    eprintln!("Unknown bar subcommand: '{}'. Usage: msui bar [toggle|open|close|glass <frosted|transparent|solid>|style <modern|classic>]", other);
                    std::process::exit(1);
                }
            }
        }
        "clipboard-ingest" => {
            handle_clipboard_ingest();
            return Ok(());
        }
        "clipboard" => {
            if args.get(2).map(|s| s.as_str()) == Some("ingest") {
                handle_clipboard_ingest();
                return Ok(());
            }
            if try_ipc_toggle("clipboard") {
                return Ok(());
            }
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
            if try_ipc_toggle("launcher") {
                return Ok(());
            }
            tracing::info!("Starting msUI Application Launcher...");
            let config = MsuiConfig::load_or_default();
            if let Err(e) = msui_launcher::run_launcher(config) {
                tracing::error!("msUI Launcher exited with error: {:?}", e);
            }
        }
        "fm" | "files" | "filemanager" => {
            let target = args.get(2).map(std::path::PathBuf::from);
            tracing::info!("Starting GUI File Manager (Thunar) (target: {:?})...", target);
            let mut cmd = std::process::Command::new("thunar");
            if let Some(t) = target {
                cmd.arg(t);
            }
            if let Err(e) = cmd.spawn() {
                tracing::error!("Failed to launch Thunar: {:?}", e);
            }
        }
        "power" => {
            if try_ipc_toggle("power") {
                return Ok(());
            }
            tracing::info!("Starting msUI Power & Session Menu...");
            if let Err(e) = msui_launcher::run_power_menu() {
                tracing::error!("msUI Power Menu exited with error: {:?}", e);
            }
        }
        "notifications" | "notifs" => {
            if try_ipc_toggle("notifications") {
                return Ok(());
            }
            tracing::info!("Starting msUI Notification Center...");
            if let Err(e) = msui_launcher::run_notification_center() {
                tracing::error!("msUI Notification Center exited with error: {:?}", e);
            }
        }
        "bluetooth" | "bt" => {
            let sub = args.get(2).map(|s| s.as_str());
            if sub.is_none() && try_ipc_toggle("bluetooth") {
                return Ok(());
            }
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
                        let mut password = args.get(4).map(|s| s.to_string());
                        if password.is_none() {
                            use std::io::{IsTerminal, Read};
                            if !std::io::stdin().is_terminal() {
                                let mut stdin_pass = String::new();
                                if std::io::stdin().read_to_string(&mut stdin_pass).is_ok() {
                                    let trimmed = stdin_pass.trim_end_matches(&['\r', '\n'][..]);
                                    if !trimmed.is_empty() {
                                        password = Some(trimmed.to_string());
                                    }
                                }
                            }
                        }
                        match msui_core::NetworkMonitor::connect_wifi(ssid, password.as_deref()) {
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
                    if try_ipc_toggle("wifi") {
                        return Ok(());
                    }
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
                    if try_ipc_toggle("wallpaper") {
                        return Ok(());
                    }
                    tracing::info!("Starting msUI Wallpaper Selector...");
                    if let Err(e) = msui_launcher::run_wallpaper_menu() {
                        tracing::error!("msUI Wallpaper Menu exited with error: {:?}", e);
                    }
                }
            }
        }
        "-bg" | "--bg" => {
            if let Some(bg) = args.get(2) {
                let mut config = MsuiConfig::load_or_default();
                config.gui.background = bg.to_string();
                config.save_to_default_path()?;
                let _ = IpcClient::send_blocking(&IpcCommand::ReloadConfig);
                println!("✓ MSUI Universal Background set to: {}", bg);
            } else {
                let config = MsuiConfig::load_or_default();
                println!("MSUI Universal Background: {}", config.gui.background);
            }
            return Ok(());
        }
        "bg" | "wallpaper-daemon" => {
            if let Some(arg2) = args.get(2) {
                if arg2.starts_with('#') || (arg2.len() == 6 && arg2.chars().all(|c| c.is_ascii_hexdigit())) {
                    let mut config = MsuiConfig::load_or_default();
                    config.gui.background = arg2.to_string();
                    config.save_to_default_path()?;
                    let _ = IpcClient::send_blocking(&IpcCommand::ReloadConfig);
                    println!("✓ MSUI Universal Background set to: {}", arg2);
                    return Ok(());
                }
            }
            if IpcClient::send_blocking(&IpcCommand::Ping).is_ok() {
                println!("msUI unified daemon is already running (native wallpaper engine active).");
                return Ok(());
            }
            tracing::info!("Starting standalone msUI Native Wallpaper Background Daemon...");
            if let Err(e) = msui_launcher::run_wallpaper_daemon() {
                tracing::error!("msUI Wallpaper Daemon exited with error: {:?}", e);
            }
        }
        "theme" | "themes" => {
            if let Some(name) = args.get(2) {
                if msui_core::ColorPreset::find(name).is_some() {
                    let _ = msui_core::ColorEngine::apply_preset(name, msui_core::ColorSchemeMode::Dark);
                    println!("Applied '{}' theme preset via Adaptive Color Engine", name);
                } else if ThemeManager::set_theme(name) {
                    println!("Theme switched to: {}", name);
                } else {
                    eprintln!("Failed to set theme: {}", name);
                    std::process::exit(1);
                }
            } else {
                if try_ipc_toggle("settings") {
                    return Ok(());
                }
                let config = MsuiConfig::load_or_default();
                if let Err(e) = msui_launcher::run_settings_panel(config) {
                    tracing::error!("msUI Settings Panel exited with error: {:?}", e);
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
                    if try_ipc_toggle("audio") {
                        return Ok(());
                    }
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
                        if let Some(mode) = bat.conservation_mode {
                            let mode_str = if mode { "Desk Mode (Active: ~55–60% charge limit)" } else { "Full Charge Mode (100%)" };
                            println!("{:<22}: {}", "Charging Mode", mode_str);
                        }
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
                Some("desk") => {
                    match msui_core::BatteryMonitor::set_conservation_mode(true) {
                        Ok(()) => println!("Enabled Desk Mode: battery charging capped at 55–60% for battery longevity."),
                        Err(e) => {
                            eprintln!("Failed to enable desk mode: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Some("full") => {
                    match msui_core::BatteryMonitor::set_conservation_mode(false) {
                        Ok(()) => println!("Enabled Full Charge Mode: battery will charge to 100% capacity."),
                        Err(e) => {
                            eprintln!("Failed to switch to full charge mode: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Some("conservation") => {
                    let sub2 = args.get(3).map(|s| s.as_str());
                    match sub2 {
                        Some("on" | "enable" | "1") => {
                            match msui_core::BatteryMonitor::set_conservation_mode(true) {
                                Ok(()) => println!("Lenovo Conservation Mode enabled (55–60% limit)."),
                                Err(e) => {
                                    eprintln!("Failed to enable conservation mode: {}", e);
                                    std::process::exit(1);
                                }
                            }
                        }
                        Some("off" | "disable" | "0") => {
                            match msui_core::BatteryMonitor::set_conservation_mode(false) {
                                Ok(()) => println!("Lenovo Conservation Mode disabled (charging to 100%)."),
                                Err(e) => {
                                    eprintln!("Failed to disable conservation mode: {}", e);
                                    std::process::exit(1);
                                }
                            }
                        }
                        _ => {
                            let current = msui_core::BatteryMonitor::read_conservation_mode();
                            println!("Conservation Mode: {:?}", current);
                        }
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
                    if try_ipc_toggle("battery") {
                        return Ok(());
                    }
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
                    if try_ipc_toggle("display") {
                        return Ok(());
                    }
                    tracing::info!("Starting msUI Display & Monitor Manager...");
                    if let Err(e) = msui_launcher::run_display_menu() {
                        tracing::error!("msUI Display Menu exited with error: {:?}", e);
                    }
                }
            }
        }
        "drawer" | "tasks" | "appdrawer" | "apps" => {
            let sub = args.get(2).map(|s| s.as_str());
            match sub {
                Some("list") => {
                    let clients = msui_core::HyprlandIpc::get_clients();
                    if clients.is_empty() {
                        println!("No active windows or tasks found.");
                    } else {
                        println!("{:<16} {:<6} {:<8} {:<24} {:<32}", "ADDRESS", "WS", "PID", "CLASS", "TITLE");
                        println!("{}", "-".repeat(88));
                        for c in clients {
                            let title = if c.title.len() > 30 {
                                format!("{}…", &c.title[..30])
                            } else {
                                c.title
                            };
                            let active_mark = if c.focus_history_id == 0 { "*" } else { " " };
                            println!("{:<16} {:<1}{:<5} {:<8} {:<24} {:<32}", c.address, active_mark, c.workspace.id, c.pid, c.class, title);
                        }
                    }
                }
                Some("focus") => {
                    if let Some(addr) = args.get(3) {
                        if msui_core::HyprlandIpc::focus_window(addr) {
                            println!("Focused window: {}", addr);
                        } else {
                            eprintln!("Failed to focus window: {}", addr);
                            std::process::exit(1);
                        }
                    } else {
                        eprintln!("Usage: msui tasks focus <address>");
                        std::process::exit(1);
                    }
                }
                Some("close") => {
                    if let Some(addr) = args.get(3) {
                        if msui_core::HyprlandIpc::close_window(addr) {
                            println!("Closed window: {}", addr);
                        } else {
                            eprintln!("Failed to close window: {}", addr);
                            std::process::exit(1);
                        }
                    } else {
                        eprintln!("Usage: msui tasks close <address>");
                        std::process::exit(1);
                    }
                }
                _ => {
                    if try_ipc_toggle("drawer") {
                        return Ok(());
                    }
                    tracing::info!("Starting msUI App Drawer & Task Switcher...");
                    let config = MsuiConfig::load_or_default();
                    if let Err(e) = msui_launcher::run_app_drawer(config) {
                        tracing::error!("msUI App Drawer exited with error: {:?}", e);
                    }
                }
            }
        }
        "control" | "quicksettings" | "cc" => {
            let sub = args.get(2).map(|s| s.as_str());
            match sub {
                Some("dnd") => {
                    let action = args.get(3).map(|s| s.as_str()).unwrap_or("status");
                    match action {
                        "on" | "enable" => {
                            msui_core::DndManager::set_dnd(true);
                            println!("Do Not Disturb: ENABLED (toasts muted)");
                        }
                        "off" | "disable" => {
                            msui_core::DndManager::set_dnd(false);
                            println!("Do Not Disturb: DISABLED (toasts active)");
                        }
                        "toggle" => {
                            let state = msui_core::DndManager::toggle_dnd();
                            println!("Do Not Disturb: {}", if state { "ENABLED (toasts muted)" } else { "DISABLED (toasts active)" });
                        }
                        _ => {
                            let state = msui_core::DndManager::is_dnd_enabled();
                            println!("Do Not Disturb Status: {}", if state { "ENABLED" } else { "DISABLED" });
                            println!("Usage: msui control dnd <on|off|toggle|status>");
                        }
                    }
                }
                Some("topbar" | "bar" | "settings") => {
                    if try_ipc_toggle("topbar") {
                        return Ok(());
                    }
                    tracing::info!("Starting msUI Settings Panel (Bar Settings)...");
                    let config = MsuiConfig::load_or_default();
                    if let Err(e) = msui_launcher::run_settings_panel(config) {
                        tracing::error!("msUI Settings Panel exited with error: {:?}", e);
                    }
                }
                _ => {
                    if try_ipc_toggle("control") {
                        return Ok(());
                    }
                    tracing::info!("Starting msUI Quick Settings Control Center...");
                    let config = MsuiConfig::load_or_default();
                    if let Err(e) = msui_launcher::run_control_center(config) {
                        tracing::error!("msUI Control Center exited with error: {:?}", e);
                    }
                }
            }
        }
        "topbar" => {
            if try_ipc_toggle("topbar") {
                return Ok(());
            }
            tracing::info!("Starting msUI Settings Panel (Bar Settings)...");
            let config = MsuiConfig::load_or_default();
            if let Err(e) = msui_launcher::run_settings_panel(config) {
                tracing::error!("msUI Settings Panel exited with error: {:?}", e);
            }
        }
        "media" | "mpris" | "player" => {
            let sub = args.get(2).map(|s| s.as_str());
            match sub {
                Some("status") => {
                    if let Some(info) = msui_core::MediaMonitor::read() {
                        let m = info.position_secs / 60;
                        let s = info.position_secs % 60;
                        let tm = info.length_secs / 60;
                        let ts = info.length_secs % 60;
                        println!("Player : {}", info.player_name);
                        println!("Status : {}", if info.is_playing { "Playing" } else { "Paused" });
                        println!("Title  : {}", info.title);
                        println!("Artist : {}", info.artist);
                        if !info.album.is_empty() {
                            println!("Album  : {}", info.album);
                        }
                        println!("Time   : {:02}:{:02} / {:02}:{:02}", m, s, tm, ts);
                    } else {
                        println!("No active media player found.");
                    }
                }
                Some("play-pause") | Some("toggle") => {
                    msui_core::MediaMonitor::play_pause();
                    println!("Toggled media playback.");
                }
                Some("play") => {
                    msui_core::MediaMonitor::play();
                    println!("Started media playback.");
                }
                Some("pause") => {
                    msui_core::MediaMonitor::pause();
                    println!("Paused media playback.");
                }
                Some("next") => {
                    msui_core::MediaMonitor::next();
                    println!("Skipped to next track.");
                }
                Some("prev") | Some("previous") => {
                    msui_core::MediaMonitor::previous();
                    println!("Skipped to previous track.");
                }
                Some("seek") => {
                    if let Some(val_str) = args.get(3) {
                        if let Ok(secs) = val_str.parse::<u64>() {
                            msui_core::MediaMonitor::seek(secs);
                            println!("Seeked to {}s.", secs);
                        } else {
                            eprintln!("Invalid seek position in seconds.");
                            std::process::exit(1);
                        }
                    } else {
                        eprintln!("Usage: msui media seek <seconds>");
                        std::process::exit(1);
                    }
                }
                _ => {
                    if try_ipc_toggle("media") {
                        return Ok(());
                    }
                    tracing::info!("Starting msUI Media Player Overlay...");
                    let config = MsuiConfig::load_or_default();
                    if let Err(e) = msui_launcher::run_media_menu(config) {
                        tracing::error!("msUI Media Menu exited with error: {:?}", e);
                    }
                }
            }
        }
        "osd" => {
            let target = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let action = args.get(3).map(|s| s.as_str()).unwrap_or("");

            // Try sending over IPC to the running unified daemon first
            let ipc_cmd = IpcCommand::Osd {
                kind: target.to_string(),
                action: action.to_string(),
            };
            if IpcClient::send_blocking(&ipc_cmd).is_ok() {
                if target == "volume" || target == "vol" {
                    let audio = msui_core::AudioMonitor::read();
                    println!("Volume: {}% (Muted: {})", audio.volume_percent, audio.is_muted);
                } else if target == "brightness" || target == "bright" {
                    let bright_mon = msui_core::BrightnessMonitor::new();
                    let b = bright_mon.read().map(|b| b.percent).unwrap_or(0);
                    println!("Brightness: {}%", b);
                }
                return Ok(());
            }

            // Fallback to standalone if daemon is not running
            match target {
                "volume" | "vol" => {
                    msui_core::OsdManager::trigger_volume(action);
                    let audio = msui_core::AudioMonitor::read();
                    println!("Volume: {}% (Muted: {})", audio.volume_percent, audio.is_muted);
                }
                "brightness" | "bright" => {
                    msui_core::OsdManager::trigger_brightness(action);
                    let bright_mon = msui_core::BrightnessMonitor::new();
                    let b = bright_mon.read().map(|b| b.percent).unwrap_or(0);
                    println!("Brightness: {}%", b);
                }
                _ => {
                    println!("Usage: msui osd <volume|brightness> <up|down|mute|<percent>>");
                }
            }
        }
        "osd-window" => {
            if let Err(e) = msui_launcher::run_osd_window() {
                tracing::error!("msUI OSD Window exited with error: {:?}", e);
            }
        }
        "storage" | "disks" | "drives" => {
            let sub = args.get(2).map(|s| s.as_str());
            match sub {
                Some("list") => {
                    let partitions = msui_core::StorageManager::list_partitions();
                    if partitions.is_empty() {
                        println!("No storage devices found.");
                    } else {
                        println!("{:<14} {:<20} {:<8} {:<10} {:<10} {:<6} {:<24}", "DEVICE", "MOUNTPOINT", "FSTYPE", "USED", "TOTAL", "USE%", "LABEL/MODEL");
                        println!("{}", "-".repeat(95));
                        for p in partitions {
                            let mp = p.mount_point.as_deref().unwrap_or("-");
                            let fs = p.fs_type.as_deref().unwrap_or("-");
                            let used = p.used_formatted();
                            let total = p.total_formatted();
                            let pct = format!("{}%", p.use_percent);
                            let title = if p.is_removable {
                                format!("[USB] {}", p.drive_model.as_deref().unwrap_or("Removable"))
                            } else {
                                p.label.as_deref().or(p.drive_model.as_deref()).unwrap_or("-").to_string()
                            };
                            println!("{:<14} {:<20} {:<8} {:<10} {:<10} {:<6} {:<24}", p.device_path, mp, fs, used, total, pct, title);
                        }
                    }
                }
                Some("mount") => {
                    if let Some(dev) = args.get(3) {
                        match msui_core::StorageManager::mount_device(dev) {
                            Ok(mp) => println!("Mounted {} at {}", dev, mp),
                            Err(e) => {
                                eprintln!("Error mounting {}: {}", dev, e);
                                std::process::exit(1);
                            }
                        }
                    } else {
                        eprintln!("Usage: msui storage mount <device>");
                        std::process::exit(1);
                    }
                }
                Some("unmount") => {
                    if let Some(dev) = args.get(3) {
                        match msui_core::StorageManager::unmount_device(dev) {
                            Ok(()) => println!("Unmounted {}", dev),
                            Err(e) => {
                                eprintln!("Error unmounting {}: {}", dev, e);
                                std::process::exit(1);
                            }
                        }
                    } else {
                        eprintln!("Usage: msui storage unmount <device>");
                        std::process::exit(1);
                    }
                }
                Some("eject") => {
                    if let Some(dev) = args.get(3) {
                        match msui_core::StorageManager::eject_drive(dev) {
                            Ok(()) => println!("Safely powered off / ejected {}", dev),
                            Err(e) => {
                                eprintln!("Error ejecting {}: {}", dev, e);
                                std::process::exit(1);
                            }
                        }
                    } else {
                        eprintln!("Usage: msui storage eject <device>");
                        std::process::exit(1);
                    }
                }
                _ => {
                    if try_ipc_toggle("storage") {
                        return Ok(());
                    }
                    tracing::info!("Starting msUI Disk Storage & Removable Drives Manager...");
                    let config = MsuiConfig::load_or_default();
                    if let Err(e) = msui_launcher::run_storage_menu(config) {
                        tracing::error!("msUI Storage Menu exited with error: {:?}", e);
                    }
                }
            }
        }
        "reminder" | "reminders" | "timer" | "timers" => {
            let sub = args.get(2).map(|s| s.as_str());
            match sub {
                Some("list") => {
                    let reminders = ReminderManager::list_active();
                    if reminders.is_empty() {
                        println!("No active reminders or timers.");
                    } else {
                        println!("{:<38} {:<24} {:<12} {:<12} {:<10}", "ID", "TITLE", "REMAINING", "DURATION", "STATUS");
                        println!("{}", "-".repeat(98));
                        for r in reminders {
                            let total_mins = r.total_duration_secs / 60;
                            let dur_str = format!("{}m", total_mins);
                            let status = if r.is_expired() { "EXPIRED" } else { "RUNNING" };
                            println!("{:<38} {:<24} {:<12} {:<12} {:<10}", r.id, r.title, r.formatted_remaining(), dur_str, status);
                        }
                    }
                }
                Some("add") => {
                    let arg3 = args.get(3).map(|s| s.as_str());
                    let arg4 = args.get(4).map(|s| s.as_str());

                    let (title, mins) = match (arg3, arg4) {
                        (Some(a), Some(b)) => {
                            if let Ok(m) = b.parse::<u64>() {
                                (a.to_string(), m)
                            } else if let Ok(m) = a.parse::<u64>() {
                                (b.to_string(), m)
                            } else {
                                (format!("{} {}", a, b), 10)
                            }
                        }
                        (Some(a), None) => {
                            if let Ok(m) = a.parse::<u64>() {
                                ("Timer".to_string(), m)
                            } else {
                                (a.to_string(), 10)
                            }
                        }
                        _ => {
                            eprintln!("Usage: msui reminder add <title> [minutes]");
                            std::process::exit(1);
                        }
                    };

                    let item = ReminderManager::add_reminder(&title, mins);
                    println!("Created timer: \"{}\" for {} min(s) [ID: {}]", item.title, mins, item.id);
                }
                Some("delete" | "remove" | "rm") => {
                    if let Some(id_str) = args.get(3) {
                        if let Ok(id) = id_str.parse::<u64>() {
                            if ReminderManager::remove_reminder(id) {
                                println!("Deleted reminder: {}", id);
                            } else {
                                eprintln!("Reminder ID not found: {}", id);
                                std::process::exit(1);
                            }
                        } else {
                            eprintln!("Invalid reminder ID: {}", id_str);
                            std::process::exit(1);
                        }
                    } else {
                        eprintln!("Usage: msui reminder delete <id>");
                        std::process::exit(1);
                    }
                }
                Some("clear") => {
                    ReminderManager::clear_all();
                    println!("Cleared all reminders and timers.");
                }
                _ => {
                    if try_ipc_toggle("reminder") {
                        return Ok(());
                    }
                    tracing::info!("Starting msUI Reminders & Countdown Timer Manager...");
                    let config = MsuiConfig::load_or_default();
                    if let Err(e) = msui_launcher::run_reminder_menu(config) {
                        tracing::error!("msUI Reminder Menu exited with error: {:?}", e);
                    }
                }
            }
        }
        "fshare" | "share" | "fileshare" => {
            let sub = args.get(2).map(|s| s.as_str());
            match sub {
                Some("history") => {
                    let history = msui_core::fshare::load_transfer_history();
                    if history.is_empty() {
                        println!("No FShare transfers found in history.");
                    } else {
                        println!("{:<6} {:<32} {:<12} {}", "DIR", "FILENAME", "SIZE", "TIMESTAMP");
                        println!("{}", "-".repeat(70));
                        for h in history {
                            let size_mb = h.size_bytes as f64 / (1024.0 * 1024.0);
                            println!("{:<6} {:<32} {:<12} {}", h.direction, h.filename, format!("{:.2} MB", size_mb), h.timestamp);
                        }
                    }
                }
                Some("send") => {
                    let mut file_paths: Vec<std::path::PathBuf> = Vec::new();
                    for target_file_str in args.iter().skip(3) {
                        let path = if let Some(stripped) = target_file_str.strip_prefix("~/") {
                            if let Some(home) = std::env::var_os("HOME") {
                                std::path::PathBuf::from(home).join(stripped)
                            } else {
                                std::path::PathBuf::from(target_file_str)
                            }
                        } else {
                            std::path::PathBuf::from(target_file_str)
                        };

                        if !path.exists() {
                            eprintln!("Error: File '{}' does not exist.", target_file_str);
                            std::process::exit(1);
                        }
                        file_paths.push(path);
                    }

                    if file_paths.is_empty() {
                        eprintln!("Usage: msui fshare send <file-path> [file-path-2 ...]");
                        std::process::exit(1);
                    }

                    let local_ip = match msui_core::fshare::get_local_ip() {
                        Some(ip) => ip,
                        None => {
                            eprintln!("Error: Could not detect active Wi-Fi LAN IP.");
                            std::process::exit(1);
                        }
                    };

                    let token = msui_core::fshare::generate_token();
                    let pin = msui_core::fshare::generate_pin();
                    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

                    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
                    rt.block_on(async move {
                        let handle = msui_core::fshare::FShareServer::start_send(
                            file_paths.clone(),
                            local_ip.clone(),
                            token.clone(),
                            pin.clone(),
                            tx,
                        ).await?;

                        let url = format!("http://{}:{}/d/{}", local_ip, handle.port, token);
                        println!("========================================");
                        println!("  msUI FShare - File Sharing Server");
                        println!("========================================");
                        if file_paths.len() == 1 {
                            println!("Sharing File: {}", file_paths[0].display());
                        } else {
                            println!("Sharing {} Files:", file_paths.len());
                            for f in &file_paths {
                                println!("  - {}", f.display());
                            }
                        }
                        println!("Direct URL:   {}", url);
                        println!("PIN Code:     {}", pin);
                        println!();
                        if let Ok(qr) = msui_core::fshare::generate_qr_terminal(&url) {
                            println!("{}", qr);
                        }
                        println!("Scan QR code with phone camera or open the URL.");
                        println!("Press Ctrl+C to terminate.");
                        println!("========================================");

                        while let Some(ev) = rx.recv().await {
                            match ev {
                                msui_core::fshare::FShareEvent::ClientConnected { peer_ip } => {
                                    println!("📱 Phone connected from {}", peer_ip);
                                }
                                msui_core::fshare::FShareEvent::Progress { transferred_bytes, total_bytes, speed_mbps } => {
                                    let pct = if total_bytes > 0 { (transferred_bytes as f32 / total_bytes as f32) * 100.0 } else { 0.0 };
                                    print!("\rTransferring: {:.1}% ({:.1} MB/s)...", pct, speed_mbps);
                                    let _ = std::io::Write::flush(&mut std::io::stdout());
                                }
                                msui_core::fshare::FShareEvent::FileSent { filename, size_bytes } => {
                                    println!("\n✅ Successfully sent '{}' ({:.2} MB)!", filename, size_bytes as f64 / (1024.0 * 1024.0));
                                    msui_core::fshare::log_transfer("Sent", &filename, size_bytes);
                                    if file_paths.len() <= 1 {
                                        break;
                                    }
                                }
                                msui_core::fshare::FShareEvent::Error(e) => {
                                    eprintln!("\n❌ Transfer error: {}", e);
                                    break;
                                }
                                _ => {}
                            }
                        }
                        handle.stop();
                        Ok::<(), anyhow::Error>(())
                    })?;
                }
                Some("receive") | Some("recv") => {
                    let save_dir = msui_core::fshare::default_save_dir();
                    let local_ip = match msui_core::fshare::get_local_ip() {
                        Some(ip) => ip,
                        None => {
                            eprintln!("Error: Could not detect active Wi-Fi LAN IP.");
                            std::process::exit(1);
                        }
                    };

                    let token = msui_core::fshare::generate_token();
                    let pin = msui_core::fshare::generate_pin();
                    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

                    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
                    rt.block_on(async move {
                        let handle = msui_core::fshare::FShareServer::start_recv(
                            save_dir.clone(),
                            local_ip.clone(),
                            token.clone(),
                            pin.clone(),
                            tx,
                        ).await?;

                        let url = format!("http://{}:{}/r/{}", local_ip, handle.port, token);
                        println!("========================================");
                        println!("  msUI FShare - File Dropzone Receiver");
                        println!("========================================");
                        println!("Destination: {}", save_dir.display());
                        println!("Direct URL:  {}", url);
                        println!("PIN Code:    {}", pin);
                        println!();
                        if let Ok(qr) = msui_core::fshare::generate_qr_terminal(&url) {
                            println!("{}", qr);
                        }
                        println!("Scan QR code with phone camera to send files to PC.");
                        println!("Press Ctrl+C to terminate.");
                        println!("========================================");

                        while let Some(ev) = rx.recv().await {
                            match ev {
                                msui_core::fshare::FShareEvent::ClientConnected { peer_ip } => {
                                    println!("📱 Phone connected from {}", peer_ip);
                                }
                                msui_core::fshare::FShareEvent::Progress { transferred_bytes, total_bytes, speed_mbps } => {
                                    let pct = if total_bytes > 0 { (transferred_bytes as f32 / total_bytes as f32) * 100.0 } else { 0.0 };
                                    print!("\rReceiving: {:.1}% ({:.1} MB/s)...", pct, speed_mbps);
                                    let _ = std::io::Write::flush(&mut std::io::stdout());
                                }
                                msui_core::fshare::FShareEvent::FileReceived { filename, size_bytes, path } => {
                                    println!("\n✅ Received '{}' -> {}", filename, path.display());
                                    msui_core::fshare::log_transfer("Received", &filename, size_bytes);
                                }
                                msui_core::fshare::FShareEvent::Error(e) => {
                                    eprintln!("\n❌ Error: {}", e);
                                }
                                _ => {}
                            }
                        }
                        handle.stop();
                        Ok::<(), anyhow::Error>(())
                    })?;
                }
                _ => {
                    let mut initial_files = Vec::new();
                    for arg in args.iter().skip(2) {
                        let p = if let Some(stripped) = arg.strip_prefix("~/") {
                            if let Some(home) = std::env::var_os("HOME") {
                                std::path::PathBuf::from(home).join(stripped)
                            } else {
                                std::path::PathBuf::from(arg)
                            }
                        } else {
                            std::path::PathBuf::from(arg)
                        };
                        if p.exists() {
                            initial_files.push(p);
                        }
                    }

                    if initial_files.is_empty() {
                        if try_ipc_toggle("fshare") {
                            return Ok(());
                        }
                    }

                    tracing::info!("Starting msUI FShare Hub with {} initial file(s)...", initial_files.len());
                    if let Err(e) = msui_launcher::run_fshare_modal(initial_files) {
                        tracing::error!("msUI FShare exited with error: {:?}", e);
                    }
                }
            }
        }
        "toggle" => {
            if let Some(target) = args.get(2) {
                match IpcClient::send_blocking(&IpcCommand::Toggle(target.to_string())) {
                    Ok(res) => println!("{}", res),
                    Err(e) => {
                        eprintln!("Failed to send toggle over IPC: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("Usage: msui toggle <target>");
                std::process::exit(1);
            }
        }
        "open" => {
            if let Some(target) = args.get(2) {
                match IpcClient::send_blocking(&IpcCommand::Open(target.to_string())) {
                    Ok(res) => println!("{}", res),
                    Err(e) => {
                        eprintln!("Failed to send open over IPC: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("Usage: msui open <target>");
                std::process::exit(1);
            }
        }
        "close" => {
            match IpcClient::send_blocking(&IpcCommand::Close) {
                Ok(res) => println!("{}", res),
                Err(e) => {
                    eprintln!("Failed to send close over IPC: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "ping" => {
            match IpcClient::send_blocking(&IpcCommand::Ping) {
                Ok(res) => println!("Daemon response: {}", res),
                Err(e) => {
                    eprintln!("Daemon not responding: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "reload" => {
            match IpcClient::send_blocking(&IpcCommand::ReloadConfig) {
                Ok(res) => println!("Daemon response: {}", res),
                Err(e) => {
                    eprintln!("Failed to reload config via IPC: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "polkit" => {
            match IpcClient::send_blocking(&IpcCommand::Ping) {
                Ok(_) => {
                    println!("msUI unified daemon is already running (PolicyKit 1 authentication agent active).");
                    return Ok(());
                }
                Err(e) => {
                    tracing::debug!("Daemon ping check failed: {:?}", e);
                }
            }
            tracing::info!("Starting standalone msUI Native Polkit 1 Authentication Agent...");
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            rt.block_on(async {
                if let Err(e) = msui_core::run_polkit_agent().await {
                    tracing::error!("msUI Polkit Agent error: {:?}", e);
                    std::process::exit(1);
                }
            });
        }
        "polkit-dialog" => {
            let mut action_id = String::new();
            let mut message = String::new();
            let mut icon_name = String::new();
            let mut user = String::new();
            let mut cookie = String::new();
            let mut command_str = String::new();

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--action-id" => {
                        if let Some(v) = args.get(i + 1) { action_id = v.clone(); i += 1; }
                    }
                    "--message" => {
                        if let Some(v) = args.get(i + 1) { message = v.clone(); i += 1; }
                    }
                    "--icon" => {
                        if let Some(v) = args.get(i + 1) { icon_name = v.clone(); i += 1; }
                    }
                    "--user" => {
                        if let Some(v) = args.get(i + 1) { user = v.clone(); i += 1; }
                    }
                    "--cookie" => {
                        if let Some(v) = args.get(i + 1) { cookie = v.clone(); i += 1; }
                    }
                    "--command" => {
                        if let Some(v) = args.get(i + 1) { command_str = v.clone(); i += 1; }
                    }
                    _ => {}
                }
                i += 1;
            }

            if cookie.is_empty() {
                use std::io::{IsTerminal, Read};
                if !std::io::stdin().is_terminal() {
                    let mut stdin_cookie = String::new();
                    if std::io::stdin().read_to_string(&mut stdin_cookie).is_ok() {
                        cookie = stdin_cookie.trim_end_matches(&['\r', '\n'][..]).to_string();
                    }
                }
            }

            let dialog_args = msui_launcher::PolkitDialogArgs {
                action_id,
                message,
                icon_name,
                user,
                cookie,
                command: command_str,
            };

            if let Err(e) = msui_launcher::run_polkit_dialog(dialog_args) {
                tracing::error!("msUI Polkit Dialog error: {:?}", e);
                std::process::exit(1);
            }
        }
        "settings" | "systemcontrol" | "control-panel" => {
            let sub = args.get(2).map(|s| s.as_str());
            if sub.is_none() && try_ipc_toggle("settings") {
                return Ok(());
            }
            tracing::info!("Starting msUI System Control Panel...");
            let config = MsuiConfig::load_or_default();
            if let Err(e) = msui_launcher::run_settings_panel(config) {
                tracing::error!("msUI Settings Panel exited with error: {:?}", e);
            }
        }
        "-border" | "--border" | "border" => {
            let sub = args.get(2).map(|s| s.as_str());
            let mut config = MsuiConfig::load_or_default();
            match sub {
                Some("off" | "disable" | "hide" | "0") => {
                    config.gui.border_enabled = false;
                    config.save_to_default_path()?;
                    let _ = IpcClient::send_blocking(&IpcCommand::ReloadConfig);
                    println!("✓ MSUI Universal Border: DISABLED (Border Off)");
                }
                Some("on" | "enable" | "show" | "1") => {
                    config.gui.border_enabled = true;
                    if config.gui.border_width == 0.0 {
                        config.gui.border_width = 1.5;
                    }
                    config.save_to_default_path()?;
                    let _ = IpcClient::send_blocking(&IpcCommand::ReloadConfig);
                    println!("✓ MSUI Universal Border: ENABLED ({}px, {})", config.gui.border_width, config.gui.border_color);
                }
                Some("toggle") => {
                    config.gui.border_enabled = !config.gui.border_enabled;
                    config.save_to_default_path()?;
                    let _ = IpcClient::send_blocking(&IpcCommand::ReloadConfig);
                    let state = if config.gui.border_enabled { "ENABLED" } else { "DISABLED (Border Off)" };
                    println!("✓ MSUI Universal Border: {}", state);
                }
                Some("color") => {
                    if let Some(col) = args.get(3) {
                        config.gui.border_color = col.to_string();
                        config.gui.border_enabled = true;
                        config.save_to_default_path()?;
                        let _ = IpcClient::send_blocking(&IpcCommand::ReloadConfig);
                        println!("✓ MSUI Universal Border Color set to: {}", col);
                    } else {
                        eprintln!("Usage: msui border color <#hex> (e.g. msui border color \"#9333ea\")");
                        std::process::exit(1);
                    }
                }
                Some("width") => {
                    if let Some(w_str) = args.get(3) {
                        if let Ok(w) = w_str.parse::<f32>() {
                            config.gui.border_width = w;
                            if w <= 0.0 {
                                config.gui.border_enabled = false;
                            } else {
                                config.gui.border_enabled = true;
                            }
                            config.save_to_default_path()?;
                            let _ = IpcClient::send_blocking(&IpcCommand::ReloadConfig);
                            println!("✓ MSUI Universal Border Width set to: {}px", w);
                        } else {
                            eprintln!("Invalid border width: {}", w_str);
                            std::process::exit(1);
                        }
                    } else {
                        eprintln!("Usage: msui border width <pixels> (e.g. msui border width 1.5)");
                        std::process::exit(1);
                    }
                }
                Some("opacity") => {
                    if let Some(o_str) = args.get(3) {
                        if let Ok(o) = o_str.parse::<f32>() {
                            config.gui.border_opacity = o.clamp(0.0, 1.0);
                            config.save_to_default_path()?;
                            let _ = IpcClient::send_blocking(&IpcCommand::ReloadConfig);
                            println!("✓ MSUI Universal Border Opacity set to: {}", config.gui.border_opacity);
                        } else {
                            eprintln!("Invalid border opacity (must be 0.0 to 1.0): {}", o_str);
                            std::process::exit(1);
                        }
                    } else {
                        eprintln!("Usage: msui border opacity <0.0-1.0>");
                        std::process::exit(1);
                    }
                }
                Some("status") | None => {
                    let status = if config.gui.border_enabled && config.gui.border_width > 0.0 {
                        format!("ENABLED (Width: {}px, Color: {}, Opacity: {})", config.gui.border_width, config.gui.border_color, config.gui.border_opacity)
                    } else {
                        "DISABLED (Border Off)".to_string()
                    };
                    println!("MSUI Universal Border: {}", status);
                }
                Some(other) => {
                    eprintln!("Unknown border command: '{}'. Usage: msui border [on|off|toggle|color <hex>|width <px>|opacity <0.0-1.0>|status]", other);
                    std::process::exit(1);
                }
            }
        }
        "gui" => {
            let sub = args.get(2).map(|s| s.as_str());
            let mut config = MsuiConfig::load_or_default();
            match sub {
                Some("bg" | "background") => {
                    if let Some(bg) = args.get(3) {
                        config.gui.background = bg.to_string();
                        config.save_to_default_path()?;
                        let _ = IpcClient::send_blocking(&IpcCommand::ReloadConfig);
                        println!("✓ MSUI Universal Background set to: {}", bg);
                    } else {
                        eprintln!("Usage: msui gui bg <#hex> (e.g. msui gui bg \"#121218\")");
                        std::process::exit(1);
                    }
                }
                Some("bg-opacity" | "opacity") => {
                    if let Some(op_str) = args.get(3) {
                        if let Ok(op) = op_str.parse::<f32>() {
                            config.gui.bg_opacity = op.clamp(0.0, 1.0);
                            config.save_to_default_path()?;
                            let _ = IpcClient::send_blocking(&IpcCommand::ReloadConfig);
                            println!("✓ MSUI Universal Background Opacity set to: {}", config.gui.bg_opacity);
                        } else {
                            eprintln!("Invalid opacity: {}", op_str);
                            std::process::exit(1);
                        }
                    } else {
                        eprintln!("Usage: msui gui bg-opacity <0.0-1.0>");
                        std::process::exit(1);
                    }
                }
                Some("radius" | "corner-radius") => {
                    if let Some(r_str) = args.get(3) {
                        if let Ok(r) = r_str.parse::<f32>() {
                            config.gui.corner_radius = r;
                            config.save_to_default_path()?;
                            let _ = IpcClient::send_blocking(&IpcCommand::ReloadConfig);
                            println!("✓ MSUI Universal Corner Radius set to: {}px", r);
                        } else {
                            eprintln!("Invalid corner radius: {}", r_str);
                            std::process::exit(1);
                        }
                    } else {
                        eprintln!("Usage: msui gui radius <pixels>");
                        std::process::exit(1);
                    }
                }
                Some("reset") => {
                    config.gui = msui_core::GuiConfig::default();
                    config.save_to_default_path()?;
                    let _ = IpcClient::send_blocking(&IpcCommand::ReloadConfig);
                    println!("✓ MSUI Universal GUI styling reset to default obsidian & purple accent theme");
                }
                Some("status") | None => {
                    println!("MSUI Universal GUI Configuration:");
                    println!("  Background:      {} (opacity: {})", config.gui.background, config.gui.bg_opacity);
                    println!("  Border Enabled:  {}", config.gui.border_enabled);
                    println!("  Border Color:    {} (opacity: {})", config.gui.border_color, config.gui.border_opacity);
                    println!("  Border Width:    {}px", config.gui.border_width);
                    println!("  Corner Radius:   {}px", config.gui.corner_radius);
                }
                Some(other) => {
                    eprintln!("Unknown gui command: '{}'. Usage: msui gui [status|bg <hex>|bg-opacity <0.0-1.0>|radius <px>|reset]", other);
                    std::process::exit(1);
                }
            }
        }
        "color" | "theme-engine" => {
            let sub = args.get(2).map(|s| s.as_str());
            let mode = if args.iter().any(|a| a == "--light" || a == "light") {
                msui_core::ColorSchemeMode::Light
            } else {
                msui_core::ColorSchemeMode::Dark
            };

            match sub {
                Some("auto" | "wallpaper" | "extract") => {
                    let wall_path = if let Some(p) = args.get(3) {
                        std::path::PathBuf::from(p)
                    } else if let Some(cur) = msui_core::WallpaperManager::get_current() {
                        cur
                    } else {
                        eprintln!("No wallpaper found to extract colors from.");
                        std::process::exit(1);
                    };

                    println!("Extracting adaptive Material 3 palette from: {}", wall_path.display());
                    match msui_core::ColorEngine::apply_wallpaper(&wall_path, mode) {
                        Ok(tokens) => {
                            println!("✓ Successfully extracted and applied adaptive desktop theme ({:?}):", mode);
                            println!("  Primary:      {}", tokens.primary);
                            println!("  Secondary:    {}", tokens.secondary);
                            println!("  Surface:      {}", tokens.surface);
                            println!("  Background:   {}", tokens.background);
                            println!("  Outline:      {}", tokens.outline);
                        }
                        Err(e) => {
                            eprintln!("Color extraction error: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Some("preset") => {
                    if let Some(name) = args.get(3) {
                        match msui_core::ColorEngine::apply_preset(name, mode) {
                            Ok(tokens) => {
                                println!("✓ Applied preset '{}' ({:?}):", name, mode);
                                println!("  Primary:    {}", tokens.primary);
                                println!("  Background: {}", tokens.background);
                            }
                            Err(e) => {
                                eprintln!("Error applying preset: {}", e);
                                std::process::exit(1);
                            }
                        }
                    } else {
                        println!("Available presets:");
                        for p in msui_core::PRESETS {
                            println!("  • {:<12} - {} (Accent: {}, Bg: {})", p.id, p.name, p.accent_hex, p.bg_hex);
                        }
                        println!("\nUsage: msui color preset <name> [--light]");
                    }
                }
                Some("set" | "accent") => {
                    if let Some(hex) = args.get(3) {
                        match msui_core::ColorEngine::apply_accent(hex, mode) {
                            Ok(tokens) => {
                                println!("✓ Generated and applied Material 3 scheme from accent '{}' ({:?}):", hex, mode);
                                println!("  Primary:    {}", tokens.primary);
                                println!("  Surface:    {}", tokens.surface);
                                println!("  Background: {}", tokens.background);
                            }
                            Err(e) => {
                                eprintln!("Error applying accent: {}", e);
                                std::process::exit(1);
                            }
                        }
                    } else {
                        eprintln!("Usage: msui color set <#hex> [--light]");
                        std::process::exit(1);
                    }
                }
                Some("mode") => {
                    if let Some(m_str) = args.get(3) {
                        let m = match m_str.to_lowercase().as_str() {
                            "light" => msui_core::ColorSchemeMode::Light,
                            _ => msui_core::ColorSchemeMode::Dark,
                        };
                        let mut tokens = msui_core::ColorEngine::current_tokens();
                        let pal = msui_core::CorePalette::from_hex(&tokens.primary)
                            .unwrap_or_else(|| msui_core::CorePalette::from_hex("#9333ea").unwrap());
                        tokens = msui_core::SemanticTokens::from_palette(&pal, m);
                        let _ = msui_core::ColorEngine::save_and_apply(&tokens);
                        println!("✓ Switched theme mode to: {:?}", m);
                    } else {
                        eprintln!("Usage: msui color mode [dark|light]");
                        std::process::exit(1);
                    }
                }
                Some("status" | "current") | None => {
                    let tokens = msui_core::ColorEngine::current_tokens();
                    println!("MSUI Adaptive Color Engine Status ({:?}):", tokens.mode);
                    println!("  Primary:      {}", tokens.primary);
                    println!("  On-Primary:   {}", tokens.on_primary);
                    println!("  Secondary:    {}", tokens.secondary);
                    println!("  Tertiary:     {}", tokens.tertiary);
                    println!("  Surface:      {}", tokens.surface);
                    println!("  Background:   {}", tokens.background);
                    println!("  Outline:      {}", tokens.outline);
                    println!("  Error:        {}", tokens.error);
                }
                Some(other) => {
                    eprintln!("Unknown color command: '{}'.\nUsage: msui color [auto|preset <name>|set <hex>|mode <dark|light>|status]", other);
                    std::process::exit(1);
                }
            }
        }
        "system" => {
            let sub_args: Vec<&str> = args.iter().skip(2).map(|s| s.as_str()).collect();
            let mut cmd = std::process::Command::new("systemcontrol");
            cmd.args(sub_args);
            let status = cmd.status()?;
            std::process::exit(status.code().unwrap_or(0));
        }
        "ai" => {
            handle_ai_command(&args)?;
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

fn handle_ai_command(args: &[String]) -> anyhow::Result<()> {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("status");
    match sub {
        "status" => {
            let status = LlamaRuntime::get_status();
            let mem = MemorySentinel::read_memory_status().ok();
            let models = ModelStore::list_models();
            let cfg = AiConfig::load();

            println!("\x1b[1;36m==================================================\x1b[0m");
            println!("\x1b[1;36m              msUI Local AI Manager               \x1b[0m");
            println!("\x1b[1;36m==================================================\x1b[0m");
            if status.is_running {
                println!("  Status:       \x1b[1;32m● RUNNING\x1b[0m");
                println!("  PID:          {}", status.pid.unwrap_or(0));
                println!("  Active Model: \x1b[1m{}\x1b[0m", status.model_name.as_deref().unwrap_or("unknown"));
                println!("  API Endpoint: \x1b[4m{}\x1b[0m", status.api_url);
                if let Some(ram) = status.ram_usage_mb {
                    println!("  Process RAM:  {:.1} MB", ram);
                }
                if let Some(ref st) = status.started_at {
                    println!("  Started At:   {}", st);
                }
            } else {
                println!("  Status:       \x1b[1;30m○ STOPPED (0 MB RAM)\x1b[0m");
                println!("  Configured:   Port {}, Context {} tokens, {} CPU threads", cfg.port, cfg.context_size, cfg.cpu_threads);
            }

            println!("\x1b[1mSystem Memory:\x1b[0m");
            if let Some(m) = mem {
                println!("  Available:    {:.2} GB / {:.2} GB total", m.available_mb as f64 / 1024.0, m.total_mb as f64 / 1024.0);
                println!("  Swap (ZRAM):  {:.2} GB free / {:.2} GB total", m.swap_free_mb as f64 / 1024.0, m.swap_total_mb as f64 / 1024.0);
            }

            println!("\x1b[1mInstalled Models ({}):\x1b[0m", models.len());
            if models.is_empty() {
                println!("  No .gguf models found in ~/.local/share/msui/ai/models/");
            } else {
                for m in &models {
                    println!("  • \x1b[1m{}\x1b[0m ({}, {})", m.name, m.size_human, m.quantization);
                }
            }
            println!("\x1b[1;36m==================================================\x1b[0m");
        }
        "start" => {
            let model_name = args.get(3).map(|s| s.as_str());
            println!("Starting msUI Local AI runtime (llama-server)...");
            match LlamaRuntime::start(model_name) {
                Ok(status) => {
                    println!("\x1b[1;32m[✔] Local AI started successfully!\x1b[0m");
                    println!("  Model:    {}", status.model_name.as_deref().unwrap_or(""));
                    println!("  Endpoint: {}/v1", status.api_url);
                    println!("  PID:      {}", status.pid.unwrap_or(0));
                }
                Err(e) => {
                    eprintln!("\x1b[1;31m[✘] Failed to start Local AI:\x1b[0m {}", e);
                    std::process::exit(1);
                }
            }
        }
        "stop" => {
            println!("Stopping msUI Local AI runtime...");
            if let Err(e) = LlamaRuntime::stop() {
                eprintln!("Error stopping: {}", e);
            } else {
                println!("\x1b[1;32m[✔] Local AI stopped. 100% RAM reclaimed (0 MB footprint).\x1b[0m");
            }
        }
        "restart" => {
            let model_name = args.get(3).map(|s| s.as_str());
            let _ = LlamaRuntime::stop();
            std::thread::sleep(std::time::Duration::from_millis(300));
            match LlamaRuntime::start(model_name) {
                Ok(status) => {
                    println!("\x1b[1;32m[✔] Local AI restarted on {}\x1b[0m", status.api_url);
                }
                Err(e) => {
                    eprintln!("Failed to restart: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "models" | "list" => {
            let models = ModelStore::list_models();
            if models.is_empty() {
                println!("No models installed in ~/.local/share/msui/ai/models/");
                println!("To add a model, copy any .gguf file into ~/.local/share/msui/ai/models/ or run: msui ai pull <preset>");
            } else {
                println!("{:<25} {:<10} {:<10} {:<20}", "NAME", "SIZE", "QUANT", "MODIFIED");
                println!("{:-<70}", "");
                for m in models {
                    println!("{:<25} {:<10} {:<10} {:<20}", m.name, m.size_human, m.quantization, m.modified);
                }
            }
        }
        "delete" | "rm" => {
            if let Some(name) = args.get(3) {
                match ModelStore::delete_model(name) {
                    Ok(true) => println!("\x1b[1;32m[✔] Model '{}' deleted.\x1b[0m", name),
                    Ok(false) => eprintln!("\x1b[1;31m[✘] Model '{}' not found.\x1b[0m", name),
                    Err(e) => eprintln!("\x1b[1;31m[✘] Failed to delete: {}\x1b[0m", e),
                }
            } else {
                eprintln!("Usage: msui ai delete <model-name>");
            }
        }
        "chat" | "prompt" => {
            use std::io::{IsTerminal, Read, Write};
            let mut piped_input = String::new();
            if !std::io::stdin().is_terminal() {
                let _ = std::io::stdin().read_to_string(&mut piped_input);
            }
            let cli_prompt = if args.len() > 3 {
                args[3..].join(" ")
            } else {
                String::new()
            };

            let query = match (piped_input.trim(), cli_prompt.trim()) {
                ("", "") => {
                    eprintln!("Usage: msui ai chat \"Your question here\" or pipe text via stdin");
                    return Ok(());
                }
                (piped, "") => piped.to_string(),
                ("", prompt) => prompt.to_string(),
                (piped, prompt) => format!("{}\n\nContext / Input:\n```\n{}\n```", prompt, piped),
            };

            let status = LlamaRuntime::get_status();
            if !status.is_running {
                println!("Local AI runtime not running. Launching on-demand...");
                if let Err(e) = LlamaRuntime::start(None) {
                    eprintln!("Failed to launch AI engine: {}", e);
                    std::process::exit(1);
                }
            }
            let cfg = AiConfig::load();
            let wait_start = std::time::Instant::now();
            while !LlamaRuntime::check_health(&cfg.host, cfg.port) {
                if wait_start.elapsed() > std::time::Duration::from_secs(10) {
                    eprintln!("AI server is not ready to serve queries. Check logs at ~/.local/share/msui/ai/logs/llama-server.log");
                    std::process::exit(1);
                }
                std::thread::sleep(std::time::Duration::from_millis(250));
            }

            let url = format!("http://{}:{}/v1/chat/completions", cfg.host, cfg.port);
            let payload = serde_json::json!({
                "messages": [
                    {"role": "system", "content": "You are a helpful, concise Linux and programming assistant."},
                    {"role": "user", "content": query}
                ],
                "temperature": 0.2,
                "max_tokens": 1024
            });
            let payload_bytes = serde_json::to_vec(&payload)?;

            // CWE-214 remediation: stream payload via stdin pipe rather than -d CLI argument
            let mut child = std::process::Command::new("curl")
                .arg("-s")
                .arg("-X")
                .arg("POST")
                .arg(&url)
                .arg("-H")
                .arg("Content-Type: application/json")
                .arg("--data-binary")
                .arg("@-")
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?;

            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(&payload_bytes)?;
            }

            let output = child.wait_with_output()?;
            if output.status.success() {
                LlamaRuntime::record_activity();

                if let Ok(res_json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                    if let Some(err_msg) = res_json["error"]["message"].as_str() {
                        eprintln!("AI Error: {}", err_msg);
                        return Ok(());
                    }
                    if let Some(content) = res_json["choices"][0]["message"]["content"].as_str() {
                        println!("\n{}", content);
                        return Ok(());
                    }
                }
                println!("{}", String::from_utf8_lossy(&output.stdout));
            } else {
                eprintln!("Query failed: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        "pull" | "download" => {
            let target = match args.get(3).map(|s| s.as_str()) {
                Some(t) => t,
                None => {
                    println!("Usage: msui ai pull <url-or-preset>");
                    println!("Presets:");
                    println!("  qwen-0.5b   Qwen2.5-Coder-0.5B-Instruct-Q4_K_M (380 MB, ultra-light)");
                    println!("  qwen-1.5b   Qwen2.5-Coder-1.5B-Instruct-Q4_K_M (980 MB, recommended for 8GB RAM)");
                    println!("  qwen-3b     Qwen2.5-Coder-3B-Instruct-Q4_K_M (1.9 GB)");
                    println!("Or pass any direct Hugging Face .gguf download link.");
                    return Ok(());
                }
            };

            let (url, filename) = match target {
                "qwen-0.5b" => (
                    "https://huggingface.co/Qwen/Qwen2.5-Coder-0.5B-Instruct-GGUF/resolve/main/qwen2.5-coder-0.5b-instruct-q4_k_m.gguf",
                    "qwen2.5-coder-0.5b-instruct-q4_k_m.gguf",
                ),
                "qwen-1.5b" => (
                    "https://huggingface.co/Qwen/Qwen2.5-Coder-1.5B-Instruct-GGUF/resolve/main/qwen2.5-coder-1.5b-instruct-q4_k_m.gguf",
                    "qwen2.5-coder-1.5b-instruct-q4_k_m.gguf",
                ),
                "qwen-3b" => (
                    "https://huggingface.co/Qwen/Qwen2.5-Coder-3B-Instruct-GGUF/resolve/main/qwen2.5-coder-3b-instruct-q4_k_m.gguf",
                    "qwen2.5-coder-3b-instruct-q4_k_m.gguf",
                ),
                direct_url => {
                    let fname = direct_url.rsplit('/').next().unwrap_or("model.gguf");
                    (direct_url, fname)
                }
            };

            let dest_dir = ModelStore::models_dir();
            let _ = std::fs::create_dir_all(&dest_dir);
            let dest_file = dest_dir.join(filename);
            println!("\x1b[1;36mDownloading model to: {}\x1b[0m", dest_file.display());

            let mut cmd = if std::process::Command::new("aria2c").arg("--version").output().is_ok() {
                let mut c = std::process::Command::new("aria2c");
                c.args(["--enable-rpc=false", "-s", "8", "-x", "8", "-c", "-d", dest_dir.to_str().unwrap(), "-o", filename, url]);
                c
            } else {
                let mut c = std::process::Command::new("curl");
                c.args(["-L", "--fail", "--progress-bar", "-C", "-", "-o", dest_file.to_str().unwrap(), url]);
                c
            };

            let status = cmd.status()?;
            if status.success() {
                println!("\x1b[1;32m[✔] Model downloaded successfully!\x1b[0m");
                let mut cfg = AiConfig::load();
                if cfg.default_model.is_none() {
                    cfg.default_model = Some(dest_file.file_stem().unwrap().to_str().unwrap().to_string());
                    let _ = cfg.save();
                }
            } else {
                eprintln!("\x1b[1;31m[✘] Download failed.\x1b[0m");
            }
        }
        other => {
            eprintln!("Unknown AI subcommand: '{}'.\nUsage: msui ai [status|start|stop|restart|models|pull|delete|chat <prompt>]", other);
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
          color         MSUI Adaptive Color Engine (auto|preset <name>|set <hex>|mode <dark|light>|status)\n  \
          drawer        Open the App Drawer & Window Task Switcher overlay (or 'list', 'focus <addr>', 'close <addr>')\n  \
          control       Open the Unified Quick Settings Control Center popup (or 'dnd <on|off|toggle>')\n  \
          media         Open the MPRIS Now Playing Controller overlay (or 'status', 'play-pause', 'next', 'prev', 'seek <secs>')\n  \
          reminder      Open the Reminders & Countdown Timer Manager overlay (or 'list', 'add <title> <mins>', 'delete <id>', 'clear')\n  \
          osd           Trigger volume or screen brightness OSD HUD (e.g. 'osd volume up', 'osd brightness down')\n  \
          launcher      Open the Spotlight application launcher overlay\n  \
          clipboard     Open the Spotlight clipboard history manager\n  \
          notifications Open the Notification Center history overlay\n  \
          storage       Open the Disk Storage & Removable Drives overlay (or 'list', 'mount <dev>', 'unmount <dev>', 'eject <dev>')\n  \
          audio         Open the Audio Mixer & Device Switcher overlay (or 'sinks', 'sources', 'set-sink', 'set-source', 'volume', 'mute')\n  \
          battery       Open the Battery Diagnostics & Power Profile Manager (or 'status', 'profile set')\n  \
          display       Open the Display, Multi-Monitor & Night Light manager overlay (or 'list', 'scale', 'rotate', 'nightlight')\n  \
          wifi          Open the Wi-Fi network manager overlay (or 'list', 'connect', 'disconnect')\n  \
          bluetooth     Open the Bluetooth settings manager overlay\n  \
          wallpaper     Open the Wallpaper changer overlay (or 'cycle', 'random', 'set <path>')\n  \
          bg            Run the native Wayland background wallpaper daemon\n  \
          theme         Open the Theme switcher overlay (or pass theme name)\n  \
          fshare        Open the msUI FShare local Wi-Fi & QR code file sharing hub (or 'history')\n  \
          ai            Local AI Manager powered by llama.cpp (status|start|stop|restart|models|delete|chat)\n  \
          power         Open the Power & Session menu overlay\n  \
          polkit        Run the native D-Bus PolicyKit 1 authentication agent daemon\n  \
          toggle        Toggle a modal overlay via instant IPC socket (e.g. 'msui toggle launcher')\n  \
          open          Open a modal overlay via instant IPC socket\n  \
          close         Close the currently active modal overlay via instant IPC socket\n  \
          reload        Reload msUI configuration from ~/.config/msui/msui.toml live\n  \
          border        Manage universal tool borders (e.g. 'border on', 'border off', 'border color <hex>', 'border width <px>')\n  \
          gui           Manage universal GUI styling (e.g. 'gui bg <hex>', 'gui status', 'gui radius <px>', 'gui reset')\n  \
          toast         Run the Toast notifications overlay\n  \
          help          Display this help message\n\n\
        OPTIONS:\n  \
          -h, --help     Print help information\n  \
          -v, --version  Print version information"
    );
}
