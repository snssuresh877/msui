use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Theme};
use msui_core::{StorageManager, StoragePartition};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SidebarPlace {
    pub label: String,
    pub icon: &'static str,
    pub path: PathBuf,
}

pub struct Sidebar;

impl Sidebar {
    pub fn get_standard_places() -> Vec<SidebarPlace> {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home".into());
        let home_path = PathBuf::from(&home);

        let mut places = vec![
            SidebarPlace {
                label: "Home".into(),
                icon: "󰋜",
                path: home_path.clone(),
            },
            SidebarPlace {
                label: "Projects".into(),
                icon: "󰅩",
                path: home_path.join("Projects"),
            },
            SidebarPlace {
                label: "Downloads".into(),
                icon: "󰉍",
                path: home_path.join("Downloads"),
            },
            SidebarPlace {
                label: "Documents".into(),
                icon: "󰈙",
                path: home_path.join("Documents"),
            },
            SidebarPlace {
                label: "Pictures".into(),
                icon: "󰋩",
                path: home_path.join("Pictures"),
            },
            SidebarPlace {
                label: "Music".into(),
                icon: "󰝚",
                path: home_path.join("Music"),
            },
            SidebarPlace {
                label: "Videos".into(),
                icon: "󰿎",
                path: home_path.join("Videos"),
            },
            SidebarPlace {
                label: "Root File System".into(),
                icon: "󰋊",
                path: PathBuf::from("/"),
            },
        ];

        // Filter out paths that don't exist, except Home and Root
        places.retain(|p| p.path == Path::new("/") || p.path == home_path || p.path.exists());
        places
    }

    pub fn view<'a, Message: Clone + 'static>(
        current_path: &Path,
        on_navigate: impl Fn(PathBuf) -> Message + Copy + 'static,
    ) -> Element<'a, Message> {
        let places = Self::get_standard_places();

        let places_header = row![
            text("PLACES").size(11).style(text::secondary),
        ]
        .padding([4, 12]);

        let mut places_list = column![places_header].spacing(3).width(Length::Fill);

        for place in places {
            let is_active = current_path == place.path;
            let target_path = place.path.clone();

            let icon_text = text(place.icon).size(15);
            let label_text = text(place.label).size(13);

            let content = row![icon_text, label_text]
                .spacing(10)
                .align_y(Alignment::Center);

            let btn = button(content)
                .width(Length::Fill)
                .padding([7, 12])
                .on_press(on_navigate(target_path))
                .style(move |theme: &Theme, status| {
                    sidebar_button_style(theme, status, is_active)
                });

            places_list = places_list.push(btn);
        }

        // Storage / Disks section
        let partitions = StorageManager::list_partitions();
        let mounted_partitions: Vec<&StoragePartition> = partitions
            .iter()
            .filter(|p| p.is_mounted && p.mount_point.is_some())
            .collect();

        let mut storage_list = column![].spacing(3).width(Length::Fill);

        if !mounted_partitions.is_empty() {
            let storage_header = row![
                text("DRIVES & STORAGE").size(11).style(text::secondary),
            ]
            .padding(Padding {
                top: 10.0,
                right: 12.0,
                bottom: 4.0,
                left: 12.0,
            });

            storage_list = storage_list.push(storage_header);

            for part in mounted_partitions {
                if let Some(mp) = &part.mount_point {
                    let mp_path = PathBuf::from(mp);
                    let is_active = current_path == mp_path;
                    let target_path = mp_path.clone();

                    let icon_str = if part.is_removable { "󱊞" } else { "󱛟" };
                    let display_name = part.display_title();

                    let content = row![
                        text(icon_str).size(15),
                        text(display_name).size(13),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center);

                    let btn = button(content)
                        .width(Length::Fill)
                        .padding([7, 12])
                        .on_press(on_navigate(target_path))
                        .style(move |theme: &Theme, status| {
                            sidebar_button_style(theme, status, is_active)
                        });

                    storage_list = storage_list.push(btn);
                }
            }
        }

        let full_sidebar = column![places_list, storage_list]
            .spacing(12)
            .width(Length::Fill);

        container(
            scrollable(full_sidebar)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fixed(210.0))
        .height(Length::Fill)
        .padding([14, 8])
        .style(|_theme: &Theme| {
            container::Style {
                background: Some(Background::Color(Color::from_rgb8(24, 24, 37))), // Mantle #181825
                border: Border {
                    color: Color::from_rgb8(49, 50, 68), // Surface0 #313244
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
    }
}

fn sidebar_button_style(
    _theme: &Theme,
    status: button::Status,
    is_active: bool,
) -> button::Style {
    // Catppuccin Mocha colors
    let bg_active = Color::from_rgb8(49, 50, 68); // Surface0
    let bg_hover = Color::from_rgb8(30, 30, 46); // Base
    let text_active = Color::from_rgb8(203, 166, 247); // Mauve
    let text_normal = Color::from_rgb8(205, 214, 244); // Text

    if is_active {
        button::Style {
            background: Some(Background::Color(bg_active)),
            text_color: text_active,
            border: Border {
                color: Color::from_rgb8(203, 166, 247), // Mauve indicator
                width: 0.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        }
    } else {
        match status {
            button::Status::Hovered => button::Style {
                background: Some(Background::Color(bg_hover)),
                text_color: Color::from_rgb8(245, 224, 220), // Rosewater
                border: Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            button::Status::Pressed => button::Style {
                background: Some(Background::Color(bg_active)),
                text_color: text_active,
                border: Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            button::Status::Disabled => button::Style {
                text_color: Color::from_rgb8(108, 112, 134),
                ..Default::default()
            },
            button::Status::Active => button::Style {
                background: None,
                text_color: text_normal,
                border: Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }
}
