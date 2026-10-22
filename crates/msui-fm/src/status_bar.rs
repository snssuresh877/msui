use crate::file_entry::FileEntry;
use iced::widget::{container, row, space, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};
use msui_core::{StorageManager, StoragePartition};
use std::path::Path;

pub struct StatusBar;

impl StatusBar {
    pub fn view<'a, Message: Clone + 'static>(
        current_path: &Path,
        entries: &[FileEntry],
        filtered_indices: &[usize],
        selected_idx: Option<usize>,
        status_msg: Option<&'a (String, bool)>, // (message, is_error)
    ) -> Element<'a, Message> {
        let total_count = entries.len();
        let hidden_count = entries.iter().filter(|e| e.is_hidden).count();

        // Left info: item counts and selection details
        let count_str = if hidden_count > 0 {
            format!("{} items ({} hidden)", total_count, hidden_count)
        } else {
            format!("{} items", total_count)
        };

        let mut left_row = row![
            text(count_str).size(12).style(|_theme: &Theme| text::Style {
                color: Some(Color::from_rgb8(166, 173, 200)),
            }),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        if let Some(sel) = selected_idx {
            if let Some(&entry_idx) = filtered_indices.get(sel) {
                if let Some(entry) = entries.get(entry_idx) {
                    left_row = left_row.push(
                        text("•").size(12).style(|_theme: &Theme| text::Style {
                            color: Some(Color::from_rgb8(108, 112, 134)),
                        }),
                    );
                    left_row = left_row.push(
                        text(format!("Selected: {} ({})", entry.name, entry.human_size))
                            .size(12)
                            .style(|_theme: &Theme| text::Style {
                                color: Some(Color::from_rgb8(203, 166, 247)),
                            }),
                    );
                }
            }
        }

        // Center: status / toast message
        let center_el: Element<'a, Message> = if let Some((msg, is_err)) = status_msg {
            let color = if *is_err {
                Color::from_rgb8(243, 139, 168) // Red
            } else {
                Color::from_rgb8(166, 227, 161) // Green
            };
            text(msg.as_str()).size(12).style(move |_theme: &Theme| text::Style {
                color: Some(color),
            }).into()
        } else {
            space().width(Length::Shrink).into()
        };

        // Right info: Disk free space on current mount
        let disk_str = get_disk_free_summary(current_path);
        let right_el = row![
            text("󱛟").size(13).style(|_theme: &Theme| text::Style {
                color: Some(Color::from_rgb8(137, 180, 250)),
            }),
            text(disk_str).size(12).style(|_theme: &Theme| text::Style {
                color: Some(Color::from_rgb8(166, 173, 200)),
            }),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let content = row![
            left_row,
            space().width(Length::Fill),
            center_el,
            space().width(Length::Fill),
            right_el,
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Length::Fill);

        container(content)
            .width(Length::Fill)
            .padding([6, 14])
            .style(|_theme: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgb8(24, 24, 37))), // Mantle #181825
                border: Border {
                    color: Color::from_rgb8(49, 50, 68),
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }
}

fn get_disk_free_summary(path: &Path) -> String {
    let partitions = StorageManager::list_partitions();
    let mut best_match: Option<&StoragePartition> = None;
    let mut best_len = 0;

    for part in &partitions {
        if let Some(mp) = &part.mount_point {
            let mp_path = Path::new(mp);
            if path.starts_with(mp_path) {
                let len = mp.len();
                if len >= best_len {
                    best_len = len;
                    best_match = Some(part);
                }
            }
        }
    }

    if let Some(part) = best_match {
        let free = StoragePartition::format_bytes(part.available_bytes);
        let total = StoragePartition::format_bytes(part.total_bytes);
        format!("{free} free of {total}")
    } else {
        "Storage info unavailable".into()
    }
}
