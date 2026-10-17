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
        let args = &self.exec_cmds[1..];

        std::process::Command::new(bin)
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        Ok(())
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
            })
        })
        .collect();

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}
