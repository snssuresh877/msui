pub mod app;
pub mod file_entry;
pub mod file_list;
pub mod operations;
pub mod sidebar;
pub mod status_bar;
pub mod toolbar;

pub use app::FileManagerApp;
pub use file_entry::FileEntry;
pub use toolbar::ViewMode;

use iced::{Font, Settings, Size, window};
use msui_core::ThemeManager;
use std::path::PathBuf;

fn app_theme(_state: &FileManagerApp) -> iced::Theme {
    ThemeManager::current_theme()
}

pub fn run_fm(initial_path: Option<PathBuf>) -> iced::Result {
    let path_clone = initial_path.clone();
    iced::application(
        move || FileManagerApp::new(path_clone.clone()),
        FileManagerApp::update,
        FileManagerApp::view,
    )
    .title(|state: &FileManagerApp| format!("msUI Files - {}", state.current_path.display()))
    .theme(app_theme)
    .settings(Settings {
        id: Some("msui-fm".to_string()),
        default_font: Font {
            family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
            ..Font::DEFAULT
        },
        ..Default::default()
    })
    .window(window::Settings {
        size: Size::new(1100.0, 720.0),
        min_size: Some(Size::new(650.0, 420.0)),
        resizable: true,
        decorations: true,
        ..Default::default()
    })
    .subscription(FileManagerApp::subscription)
    .run()
}
