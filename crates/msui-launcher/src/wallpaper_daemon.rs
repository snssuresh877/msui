use iced::widget::{container, image, space};
use iced::{ContentFit, Element, Length, Task as Command, Theme};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::{LayerShellSettings, StartMode};
use msui_core::WallpaperManager;
use std::path::PathBuf;
use std::time::Duration;

pub fn run_wallpaper_daemon() -> Result<(), iced_exwlshell::Error> {
    let start_mode = if let Some(target) = msui_core::DisplayManager::get_primary_monitor_name() {
        StartMode::TargetScreen(target)
    } else {
        StartMode::Active
    };

    application(
        WallpaperDaemon::new,
        WallpaperDaemon::namespace,
        WallpaperDaemon::update,
        WallpaperDaemon::view,
    )
    .theme(Theme::CatppuccinMocha)
    .layer_settings(LayerShellSettings {
        size: LayerSize::FILL,
        anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
        layer: Layer::Background,
        keyboard_interactivity: KeyboardInteractivity::None,
        exclusive_zone: -1,
        start_mode,
        ..Default::default()
    })
    .subscription(WallpaperDaemon::subscription)
    .run()
}

pub struct WallpaperDaemon {
    current_path: Option<PathBuf>,
    image_handle: Option<image::Handle>,
}

#[derive(Debug, Clone)]
pub enum Message {
    CheckFile,
}

impl TryInto<ExwlShellCustomActionWithId> for Message {
    type Error = Self;
    fn try_into(self) -> Result<ExwlShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}

impl WallpaperDaemon {
    fn new() -> (Self, Command<Message>) {
        let current_path = WallpaperManager::get_current();
        let image_handle = current_path
            .as_ref()
            .map(|p| image::Handle::from_path(p));

        (
            Self {
                current_path,
                image_handle,
            },
            Command::none(),
        )
    }

    fn namespace() -> String {
        String::from("msui-wallpaper-daemon")
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(Duration::from_millis(500)).map(|_| Message::CheckFile)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::CheckFile => {
                let latest = WallpaperManager::get_current();
                if latest != self.current_path {
                    self.current_path = latest.clone();
                    self.image_handle = latest.as_ref().map(|p| image::Handle::from_path(p));
                }
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        if let Some(handle) = &self.image_handle {
            image(handle.clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(ContentFit::Cover)
                .into()
        } else {
            container(space())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(container::transparent)
                .into()
        }
    }
}
