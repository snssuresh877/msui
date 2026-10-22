use crate::file_entry::FileEntry;
use crate::toolbar::ViewMode;
use iced::widget::{button, column, container, row, scrollable, space, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Name,
    Size,
    Modified,
    Type,
}

pub struct FileList;

impl FileList {
    pub fn view<'a, Message: Clone + 'static>(
        all_entries: &'a [FileEntry],
        filtered_indices: &'a [usize],
        selected_idx: Option<usize>,
        view_mode: ViewMode,
        sort_by: SortField,
        sort_ascending: bool,
        search_query: &str,
        on_select: impl Fn(usize) -> Message + Copy + 'static,
        on_open: impl Fn(usize) -> Message + Copy + 'static,
        on_sort: impl Fn(SortField) -> Message + Copy + 'static,
    ) -> Element<'a, Message> {
        if filtered_indices.is_empty() {
            let msg = if search_query.is_empty() {
                "This folder is empty".to_string()
            } else {
                format!("No items match \"{search_query}\"")
            };

            return container(
                column![
                    text("󰉋").size(48).style(|_theme: &Theme| text::Style {
                        color: Some(Color::from_rgb8(108, 112, 134)),
                    }),
                    text(msg).size(14).style(|_theme: &Theme| text::Style {
                        color: Some(Color::from_rgb8(166, 173, 200)),
                    }),
                ]
                .spacing(12)
                .align_x(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
        }

        match view_mode {
            ViewMode::Details => Self::view_details(
                all_entries,
                filtered_indices,
                selected_idx,
                sort_by,
                sort_ascending,
                on_select,
                on_open,
                on_sort,
            ),
            ViewMode::Grid => Self::view_grid(
                all_entries,
                filtered_indices,
                selected_idx,
                on_select,
                on_open,
            ),
        }
    }

    fn view_details<'a, Message: Clone + 'static>(
        all_entries: &'a [FileEntry],
        filtered_indices: &'a [usize],
        selected_idx: Option<usize>,
        sort_by: SortField,
        sort_ascending: bool,
        _on_select: impl Fn(usize) -> Message + Copy + 'static,
        on_open: impl Fn(usize) -> Message + Copy + 'static,
        on_sort: impl Fn(SortField) -> Message + Copy + 'static,
    ) -> Element<'a, Message> {
        let sort_indicator = |field: SortField| -> &'static str {
            if sort_by == field {
                if sort_ascending { " ▲" } else { " ▼" }
            } else {
                ""
            }
        };

        // Header Row
        let col_name = button(
            row![
                text(format!("NAME{}", sort_indicator(SortField::Name))).size(11),
            ]
            .align_y(Alignment::Center),
        )
        .on_press(on_sort(SortField::Name))
        .style(header_button_style)
        .width(Length::FillPortion(5));

        let col_size = button(
            row![
                text(format!("SIZE{}", sort_indicator(SortField::Size))).size(11),
            ]
            .align_y(Alignment::Center),
        )
        .on_press(on_sort(SortField::Size))
        .style(header_button_style)
        .width(Length::FillPortion(2));

        let col_modified = button(
            row![
                text(format!("DATE MODIFIED{}", sort_indicator(SortField::Modified))).size(11),
            ]
            .align_y(Alignment::Center),
        )
        .on_press(on_sort(SortField::Modified))
        .style(header_button_style)
        .width(Length::FillPortion(3));

        let col_perms = button(
            row![
                text(format!("PERMS{}", sort_indicator(SortField::Type))).size(11),
            ]
            .align_y(Alignment::Center),
        )
        .on_press(on_sort(SortField::Type))
        .style(header_button_style)
        .width(Length::FillPortion(2));

        let header = container(
            row![col_name, col_size, col_modified, col_perms]
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill),
        )
        .padding([6, 12])
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(24, 24, 37))), // Mantle #181825
            border: Border {
                color: Color::from_rgb8(49, 50, 68),
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        });

        let mut rows = column![].spacing(2).width(Length::Fill);

        for (filtered_idx, &entry_idx) in filtered_indices.iter().enumerate() {
            if let Some(entry) = all_entries.get(entry_idx) {
                let is_selected = selected_idx == Some(filtered_idx);
                let accent_color = entry.accent_color;

                let name_cell = row![
                    text(entry.icon_glyph)
                        .size(16)
                        .style(move |_theme: &Theme| text::Style {
                            color: Some(accent_color),
                        }),
                    text(&entry.name).size(13),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::FillPortion(5));

                let size_cell = container(
                    text(&entry.human_size)
                        .size(12)
                        .style(|_theme: &Theme| text::Style {
                            color: Some(Color::from_rgb8(166, 173, 200)),
                        }),
                )
                .width(Length::FillPortion(2));

                let date_cell = container(
                    text(&entry.human_date)
                        .size(12)
                        .style(|_theme: &Theme| text::Style {
                            color: Some(Color::from_rgb8(147, 154, 183)),
                        }),
                )
                .width(Length::FillPortion(3));

                let perms_cell = container(
                    text(&entry.human_permissions)
                        .size(11)
                        .style(|_theme: &Theme| text::Style {
                            color: Some(Color::from_rgb8(108, 112, 134)),
                        }),
                )
                .width(Length::FillPortion(2));

                let row_content = row![name_cell, size_cell, date_cell, perms_cell]
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .width(Length::Fill);

                let row_btn = button(row_content)
                    .width(Length::Fill)
                    .padding([6, 12])
                    .on_press(on_open(filtered_idx))
                    .style(move |theme: &Theme, status| {
                        list_row_style(theme, status, is_selected)
                    });

                rows = rows.push(row_btn);
            }
        }

        let scroll = scrollable(rows)
            .width(Length::Fill)
            .height(Length::Fill);

        column![header, scroll]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_grid<'a, Message: Clone + 'static>(
        all_entries: &'a [FileEntry],
        filtered_indices: &'a [usize],
        selected_idx: Option<usize>,
        _on_select: impl Fn(usize) -> Message + Copy + 'static,
        on_open: impl Fn(usize) -> Message + Copy + 'static,
    ) -> Element<'a, Message> {
        let mut cards: Vec<Element<'a, Message>> = Vec::new();

        for (filtered_idx, &entry_idx) in filtered_indices.iter().enumerate() {
            if let Some(entry) = all_entries.get(entry_idx) {
                let is_selected = selected_idx == Some(filtered_idx);
                let accent_color = entry.accent_color;

                let card_content = column![
                    text(entry.icon_glyph)
                        .size(32)
                        .style(move |_theme: &Theme| text::Style {
                            color: Some(accent_color),
                        }),
                    space().height(Length::Fixed(4.0)),
                    text(&entry.name)
                        .size(12)
                        .align_x(Alignment::Center),
                    text(&entry.human_size)
                        .size(10)
                        .style(|_theme: &Theme| text::Style {
                            color: Some(Color::from_rgb8(147, 154, 183)),
                        }),
                ]
                .spacing(2)
                .align_x(Alignment::Center);

                let card_btn = button(card_content)
                    .width(Length::Fixed(110.0))
                    .padding(10)
                    .on_press(on_open(filtered_idx))
                    .style(move |theme: &Theme, status| {
                        grid_card_style(theme, status, is_selected)
                    });

                cards.push(card_btn.into());
            }
        }

        let grid_layout = row(cards).spacing(10).wrap();

        container(
            scrollable(grid_layout)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .padding(14)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

fn header_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let text_color = match status {
        button::Status::Hovered => Color::from_rgb8(203, 166, 247), // Mauve
        _ => Color::from_rgb8(147, 154, 183),                      // Subtext0
    };

    button::Style {
        background: None,
        text_color,
        border: Border::default(),
        ..Default::default()
    }
}

fn list_row_style(_theme: &Theme, status: button::Status, is_selected: bool) -> button::Style {
    let bg_selected = Color::from_rgb8(49, 50, 68); // Surface0
    let bg_hover = Color::from_rgb8(36, 39, 58);   // Surface0 muted
    let text_normal = Color::from_rgb8(205, 214, 244);

    if is_selected {
        button::Style {
            background: Some(Background::Color(bg_selected)),
            text_color: Color::from_rgb8(245, 224, 220),
            border: Border {
                color: Color::from_rgb8(203, 166, 247),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        }
    } else {
        match status {
            button::Status::Hovered => button::Style {
                background: Some(Background::Color(bg_hover)),
                text_color: text_normal,
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            _ => button::Style {
                background: None,
                text_color: text_normal,
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }
}

fn grid_card_style(_theme: &Theme, status: button::Status, is_selected: bool) -> button::Style {
    let bg_card = Color::from_rgb8(24, 24, 37);      // Mantle
    let bg_selected = Color::from_rgb8(49, 50, 68);  // Surface0
    let bg_hover = Color::from_rgb8(36, 39, 58);

    if is_selected {
        button::Style {
            background: Some(Background::Color(bg_selected)),
            text_color: Color::from_rgb8(245, 224, 220),
            border: Border {
                color: Color::from_rgb8(203, 166, 247),
                width: 1.5,
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    } else {
        match status {
            button::Status::Hovered => button::Style {
                background: Some(Background::Color(bg_hover)),
                text_color: Color::from_rgb8(205, 214, 244),
                border: Border {
                    color: Color::from_rgb8(69, 71, 90),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            },
            _ => button::Style {
                background: Some(Background::Color(bg_card)),
                text_color: Color::from_rgb8(205, 214, 244),
                border: Border {
                    color: Color::from_rgb8(49, 50, 68),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            },
        }
    }
}
