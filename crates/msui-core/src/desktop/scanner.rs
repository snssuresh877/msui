use freedesktop_desktop_entry::{self as fde, IconSource};
use std::path::PathBuf;
use std::sync::LazyLock;

static LOCALE: LazyLock<Vec<String>> = LazyLock::new(fde::get_languages_from_env);

static ICONS_SIZE: &[&str] = &["256x256", "128x128", "64x64", "48x48", "32x32", "16x16"];
static THEMES_LIST: &[&str] = &["breeze", "Adwaita", "hicolor"];

#[derive(Debug, Clone)]
pub struct DesktopApp {
    pub id: String,
    pub name: String,
    pub comment: String,
    pub exec_cmds: Vec<String>,
    pub icon: Option<PathBuf>,
    pub categories: Option<Vec<String>>,
    pub startup_wm_class: Option<String>,
}

impl DesktopApp {
    pub fn matches(&self, query: &str) -> bool {
        let q = query.to_lowercase();
        if self.name.to_lowercase().contains(&q) {
            return true;
        }
        if self.comment.to_lowercase().contains(&q) {
            return true;
        }
        if self.id.to_lowercase().contains(&q) {
            return true;
        }
        if let Some(cats) = &self.categories {
            for c in cats {
                if c.to_lowercase().contains(&q) {
                    return true;
                }
            }
        }
        false
    }

    pub fn launch(&self) -> std::io::Result<()> {
        if self.exec_cmds.is_empty() {
            return Ok(());
        }

        let bin = &self.exec_cmds[0];
        let args: Vec<&str> = self.exec_cmds[1..]
            .iter()
            .map(|s| s.as_str())
            .filter(|arg| {
                !matches!(
                    *arg,
                    "@@u" | "@@f" | "@@" | "%u" | "%U" | "%f" | "%F" | "%i" | "%c" | "%k"
                )
            })
            .collect();

        // If launching Antigravity and no client window exists, ensure any lingering
        // headless instance is cleaned up so singleInstanceLock doesn't block window creation.
        if self.id.eq_ignore_ascii_case("antigravity")
            || self.name.eq_ignore_ascii_case("antigravity")
            || bin.ends_with("antigravity")
        {
            let clients = crate::HyprlandIpc::get_clients();
            let has_window = clients.iter().any(|c| {
                c.class.eq_ignore_ascii_case("antigravity")
                    || c.initial_class.eq_ignore_ascii_case("antigravity")
            });
            if !has_window {
                let _ = std::process::Command::new("pkill")
                    .args(["-f", "/opt/Antigravity/antigravity|/usr/bin/antigravity"])
                    .output();
                std::thread::sleep(std::time::Duration::from_millis(150));
            }
        }

        use std::os::unix::process::CommandExt;
        let mut cmd = std::process::Command::new(bin);
        cmd.args(&args)
            .process_group(0)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        if let Ok(home) = std::env::var("HOME") {
            cmd.current_dir(home);
        }

        // Ensure Wayland environment hints for Electron & Wayland apps
        if std::env::var("OZONE_PLATFORM").is_err() {
            cmd.env("OZONE_PLATFORM", "wayland");
        }
        if std::env::var("ELECTRON_OZONE_PLATFORM_HINT").is_err() {
            cmd.env("ELECTRON_OZONE_PLATFORM_HINT", "wayland");
        }
        if std::env::var("MOZ_ENABLE_WAYLAND").is_err() {
            cmd.env("MOZ_ENABLE_WAYLAND", "1");
        }

        cmd.spawn()?;

        Ok(())
    }

    pub fn find_running_tray_item<'a>(
        &self,
        tray_items: &'a [crate::TrayItem],
    ) -> Option<&'a crate::TrayItem> {
        let app_id_lower = self.id.to_lowercase();
        let app_name_lower = self.name.to_lowercase();
        let exec_bin = self.exec_cmds.first().map(|b| {
            std::path::Path::new(b)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or(b)
                .to_lowercase()
        });

        tray_items.iter().find(|t| {
            let id = t.id.to_lowercase();
            let title = t.title.to_lowercase();
            let tooltip = t.tooltip.to_lowercase();
            let service = t.service.to_lowercase();

            id == app_id_lower
                || id == app_name_lower
                || app_id_lower.ends_with(&format!(".{}", id))
                || id.ends_with(&format!(".{}", app_id_lower))
                || (exec_bin.as_ref().is_some_and(|b| &id == b))
                || title == app_name_lower
                || title.starts_with(&app_name_lower)
                || tooltip.contains(&app_name_lower)
                || service.contains(&app_name_lower)
        })
    }

    pub fn find_running_client<'a>(
        &self,
        clients: &'a [crate::HyprlandClient],
    ) -> Option<&'a crate::HyprlandClient> {
        let app_id_lower = self.id.to_lowercase();
        let app_name_lower = self.name.to_lowercase();
        let wm_cls_lower = self.startup_wm_class.as_deref().map(|s| s.to_lowercase());
        let exec_bin = self.exec_cmds.first().map(|b| {
            std::path::Path::new(b)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or(b)
                .to_lowercase()
        });

        clients.iter().find(|c| {
            let cls = c.class.to_lowercase();
            let init_cls = c.initial_class.to_lowercase();
            let title = c.title.to_lowercase();

            cls == app_id_lower
                || init_cls == app_id_lower
                || wm_cls_lower.as_ref().is_some_and(|w| &cls == w || &init_cls == w)
                || (exec_bin.as_ref().is_some_and(|b| &cls == b || &init_cls == b))
                || cls == app_name_lower
                || init_cls == app_name_lower
                || app_id_lower.ends_with(&format!(".{}", cls))
                || cls.ends_with(&format!(".{}", app_id_lower))
                || title.starts_with(&app_name_lower)
        })
    }
}

fn get_icon_path_from_xdgicon(iconname: &str) -> Option<PathBuf> {
    let top = xdg::BaseDirectories::with_prefix("icons");
    if let Some(p) = top.find_data_file(format!("{iconname}.svg")) {
        return Some(p);
    }
    if let Some(p) = top.find_data_file(format!("{iconname}.png")) {
        return Some(p);
    }

    let scalable = xdg::BaseDirectories::with_prefix("icons/hicolor/scalable/apps");
    if let Some(p) = scalable.find_data_file(format!("{iconname}.svg")) {
        return Some(p);
    }

    for prefix in ICONS_SIZE {
        let sized = xdg::BaseDirectories::with_prefix(format!("icons/hicolor/{prefix}/apps"));
        if let Some(p) = sized.find_data_file(format!("{iconname}.png")) {
            return Some(p);
        }
        if let Some(p) = sized.find_data_file(format!("{iconname}.svg")) {
            return Some(p);
        }
    }

    let pix = xdg::BaseDirectories::with_prefix("pixmaps");
    if let Some(p) = pix.find_data_file(format!("{iconname}.svg")) {
        return Some(p);
    }
    if let Some(p) = pix.find_data_file(format!("{iconname}.png")) {
        return Some(p);
    }

    for theme in THEMES_LIST {
        for sz in &["48", "64", "scalable"] {
            let t = xdg::BaseDirectories::with_prefix(format!("icons/{theme}/apps/{sz}"));
            if let Some(p) = t.find_data_file(format!("{iconname}.svg")) {
                return Some(p);
            }
            if let Some(p) = t.find_data_file(format!("{iconname}.png")) {
                return Some(p);
            }
        }
    }

    None
}

fn resolve_icon(icon_source: IconSource) -> Option<PathBuf> {
    match icon_source {
        IconSource::Name(name) => get_icon_path_from_xdgicon(name.as_str()),
        IconSource::Path(path) => Some(path),
    }
}

pub fn find_icon_for_class(class: &str) -> Option<PathBuf> {
    let lower = class.to_lowercase();
    if let Some(p) = get_icon_path_from_xdgicon(&lower) {
        return Some(p);
    }
    // Check without dashes/underscores or after last period (e.g., org.mozilla.firefox -> firefox)
    if let Some(last_part) = lower.rsplit('.').next() {
        if let Some(p) = get_icon_path_from_xdgicon(last_part) {
            return Some(p);
        }
    }
    None
}

pub fn scan_desktop_apps() -> Vec<DesktopApp> {
    let desktop_entries = fde::desktop_entries(&LOCALE);

    let mut apps: Vec<DesktopApp> = desktop_entries
        .iter()
        .filter(|entry| !entry.no_display() && !entry.hidden())
        .flat_map(|entry| {
            let exec_cmds = entry.parse_exec().ok()?;
            let id = entry.id().to_string();
            let name = entry.name(&LOCALE).map(|n| n.to_string())?;
            let comment = entry
                .comment(&LOCALE)
                .map(|c| c.to_string())
                .unwrap_or_else(|| format!("Launch {name}"));
            let categories = entry
                .categories()
                .map(|c| c.iter().map(|i| i.to_string()).collect());
            let startup_wm_class = entry.startup_wm_class().map(|s| s.to_string());

            let icon = entry.icon().and_then(|icon_str| {
                resolve_icon(IconSource::from_unknown(icon_str))
            });

            Some(DesktopApp {
                id,
                name,
                comment,
                exec_cmds,
                icon,
                categories,
                startup_wm_class,
            })
        })
        .collect();

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_wechat() {
        let apps = scan_desktop_apps();
        let wechat = apps.iter().find(|a| a.name.to_lowercase().contains("wechat") || a.id.to_lowercase().contains("wechat"));
        println!("WeChat found in scan_desktop_apps: {:?}", wechat);
        assert!(wechat.is_some(), "WeChat desktop entry should be found by scanner");

        let app = wechat.unwrap();
        let mock_tray = vec![
            crate::TrayItem {
                service: "org.kde.StatusNotifierItem-2-2".into(),
                path: "/StatusNotifierItem".into(),
                id: "wechat".into(),
                title: "wechat".into(),
                icon_name: "".into(),
                tooltip: "WeChat".into(),
            }
        ];
        let found_tray = app.find_running_tray_item(&mock_tray);
        assert!(found_tray.is_some(), "Should find WeChat tray item");
        assert_eq!(found_tray.unwrap().id, "wechat");
    }

    #[test]
    fn test_scan_antigravity() {
        let apps = scan_desktop_apps();
        let antigravity = apps.iter().find(|a| a.name.to_lowercase().contains("antigravity") || a.id.to_lowercase().contains("antigravity"));
        println!("Antigravity found in scan_desktop_apps: {:?}", antigravity);
        assert!(antigravity.is_some(), "Antigravity desktop entry should be found by scanner");
    }

    #[test]
    fn test_scan_flux() {
        let apps = scan_desktop_apps();
        let flux = apps.iter().find(|a| a.name.to_lowercase() == "flux" || a.id.to_lowercase().contains("flux"));
        println!("Flux found in scan_desktop_apps: {:?}", flux);
        assert!(flux.is_some(), "Flux desktop entry should be found by scanner");
        let app = flux.unwrap();
        assert_eq!(app.startup_wm_class.as_deref(), Some("flux-fm"));
    }
}
