use iced::widget::{button, container, row, space, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Details,
    Grid,
}

pub struct Toolbar;

impl Toolbar {
    pub fn view<'a, Message: Clone + 'static>(
        current_path: &Path,
        can_go_back: bool,
        can_go_forward: bool,
        can_go_up: bool,
        is_editing_address: bool,
        address_text: &str,
        search_query: &str,
        view_mode: ViewMode,
        show_hidden: bool,
        on_back: Message,
        on_forward: Message,
        on_up: Message,
        on_refresh: Message,
        on_toggle_address_mode: Message,
        on_address_input: impl Fn(String) -> Message + Copy + 'static,
        on_address_submit: Message,
        on_search_input: impl Fn(String) -> Message + Copy + 'static,
        on_clear_search: Message,
        on_toggle_view_mode: Message,
        on_toggle_hidden: Message,
        on_open_terminal: Message,
        on_new_folder: Message,
        on_navigate_to: impl Fn(PathBuf) -> Message + Copy + 'static,
    ) -> Element<'a, Message> {
        // Nav buttons
        let btn_back = button(text("󰁍").size(16))
            .padding([6, 10])
            .style(move |theme: &Theme, status| nav_button_style(theme, status, can_go_back));
        let btn_back = if can_go_back {
            btn_back.on_press(on_back)
        } else {
            btn_back
        };

        let btn_forward = button(text("󰁔").size(16))
            .padding([6, 10])
            .style(move |theme: &Theme, status| nav_button_style(theme, status, can_go_forward));
        let btn_forward = if can_go_forward {
            btn_forward.on_press(on_forward)
        } else {
            btn_forward
        };

        let btn_up = button(text("󰁝").size(16))
            .padding([6, 10])
            .style(move |theme: &Theme, status| nav_button_style(theme, status, can_go_up));
        let btn_up = if can_go_up {
            btn_up.on_press(on_up)
        } else {
            btn_up
        };

        let btn_refresh = button(text("󰑐").size(16))
            .padding([6, 10])
            .on_press(on_refresh)
            .style(|theme: &Theme, status| nav_button_style(theme, status, true));

        let nav_cluster = row![btn_back, btn_forward, btn_up, btn_refresh]
            .spacing(4)
            .align_y(Alignment::Center);

        // Center: Address Bar or Breadcrumbs
        let center_area: Element<'a, Message> = if is_editing_address {
            let input = text_input("Enter directory path...", address_text)
                .on_input(on_address_input)
                .on_submit(on_address_submit.clone())
                .padding([6, 10])
                .size(13);

            let submit_btn = button(text("󰄬 Go").size(13))
                .on_press(on_address_submit)
                .padding([6, 10])
                .style(action_pill_style);

            let cancel_btn = button(text("󰅖").size(13))
                .on_press(on_toggle_address_mode.clone())
                .padding([6, 10])
                .style(action_pill_style);

            row![input, submit_btn, cancel_btn]
                .spacing(6)
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .into()
        } else {
            let breadcrumbs = build_breadcrumbs(current_path, on_navigate_to);
            let edit_icon = button(text("󰏫").size(13))
                .padding([4, 8])
                .on_press(on_toggle_address_mode)
                .style(action_pill_style);

            row![breadcrumbs, edit_icon]
                .spacing(6)
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .into()
        };

        // Right side: Search filter, View Mode, Hidden files, Terminal, New Folder
        let search_box = if search_query.is_empty() {
            row![
                text_input("󰍉 Filter...", search_query)
                    .on_input(on_search_input)
                    .padding([5, 8])
                    .size(12)
                    .width(Length::Fixed(140.0)),
            ]
        } else {
            row![
                text_input("Filter...", search_query)
                    .on_input(on_search_input)
                    .padding([5, 8])
                    .size(12)
                    .width(Length::Fixed(110.0)),
                button(text("󰅖").size(11))
                    .padding([5, 8])
                    .on_press(on_clear_search)
                    .style(action_pill_style),
            ]
            .spacing(4)
        }
        .align_y(Alignment::Center);

        let view_icon = match view_mode {
            ViewMode::Details => "󰘰 Grid",
            ViewMode::Grid => "󰉋 List",
        };
        let btn_view = button(text(view_icon).size(12))
            .padding([6, 10])
            .on_press(on_toggle_view_mode)
            .style(action_pill_style);

        let hidden_icon = if show_hidden { "󰈈 Dotfiles" } else { "󰈉 Dotfiles" };
        let btn_hidden = button(text(hidden_icon).size(12))
            .padding([6, 10])
            .on_press(on_toggle_hidden)
            .style(action_pill_style);

        let btn_terminal = button(text("󰞷 Term").size(12))
            .padding([6, 10])
            .on_press(on_open_terminal)
            .style(action_pill_style);

        let btn_new_folder = button(text("󰉍 New").size(12))
            .padding([6, 10])
            .on_press(on_new_folder)
            .style(action_pill_style);

        let actions_cluster = row![
            search_box,
            btn_view,
            btn_hidden,
            btn_terminal,
            btn_new_folder,
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let toolbar_content = row![
            nav_cluster,
            space().width(Length::Fixed(6.0)),
            center_area,
            space().width(Length::Fixed(6.0)),
            actions_cluster,
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

        container(toolbar_content)
            .width(Length::Fill)
            .padding([10, 14])
            .style(|_theme: &Theme| {
                container::Style {
                    background: Some(Background::Color(Color::from_rgb8(30, 30, 46))), // Base #1e1e2e
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

fn build_breadcrumbs<'a, Message: Clone + 'static>(
    current_path: &Path,
    on_navigate_to: impl Fn(PathBuf) -> Message + Copy + 'static,
) -> Element<'a, Message> {
    let home = std::env::var("HOME").unwrap_or_default();
    let is_in_home = !home.is_empty() && current_path.starts_with(&home);

    let mut segments = Vec::new();

    if is_in_home {
        let home_path = PathBuf::from(&home);
        segments.push(("󰋜 ~".to_string(), home_path.clone()));

        if let Ok(rel) = current_path.strip_prefix(&home_path) {
            let mut accum = home_path;
            for comp in rel.iter() {
                accum = accum.join(comp);
                segments.push((comp.to_string_lossy().to_string(), accum.clone()));
            }
        }
    } else {
        segments.push(("󰋊 /".to_string(), PathBuf::from("/")));
        let mut accum = PathBuf::from("/");
        for comp in current_path.iter() {
            let s = comp.to_string_lossy().to_string();
            if s != "/" {
                accum = accum.join(comp);
                segments.push((s, accum.clone()));
            }
        }
    }

    let mut crumbs_row = row![].spacing(4).align_y(Alignment::Center);

    for (idx, (label, path)) in segments.into_iter().enumerate() {
        if idx > 0 {
            crumbs_row = crumbs_row.push(
                text("›")
                    .size(13)
                    .style(|_theme: &Theme| text::Style {
                        color: Some(Color::from_rgb8(108, 112, 134)), // Overlay0
                    }),
            );
        }

        let is_last = path == current_path;
        let btn = button(text(label).size(12))
            .padding([4, 8])
            .on_press(on_navigate_to(path))
            .style(move |theme: &Theme, status| breadcrumb_button_style(theme, status, is_last));

        crumbs_row = crumbs_row.push(btn);
    }

    container(crumbs_row)
        .padding([2, 6])
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(24, 24, 37))), // Mantle #181825
            border: Border {
                color: Color::from_rgb8(49, 50, 68),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn nav_button_style(_theme: &Theme, status: button::Status, enabled: bool) -> button::Style {
    if !enabled {
        return button::Style {
            background: None,
            text_color: Color::from_rgb8(88, 91, 112), // Surface2
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        };
    }

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb8(49, 50, 68))),
            text_color: Color::from_rgb8(203, 166, 247), // Mauve
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color::from_rgb8(69, 71, 90))),
            text_color: Color::from_rgb8(203, 166, 247),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        },
        _ => button::Style {
            background: Some(Background::Color(Color::from_rgb8(24, 24, 37))),
            text_color: Color::from_rgb8(205, 214, 244),
            border: Border {
                color: Color::from_rgb8(49, 50, 68),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        },
    }
}

fn breadcrumb_button_style(_theme: &Theme, status: button::Status, is_last: bool) -> button::Style {
    let text_color = if is_last {
        Color::from_rgb8(203, 166, 247) // Mauve
    } else {
        Color::from_rgb8(186, 194, 222) // Subtext1
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb8(49, 50, 68))),
            text_color: Color::from_rgb8(245, 224, 220),
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        },
        _ => button::Style {
            background: if is_last {
                Some(Background::Color(Color::from_rgb8(49, 50, 68)))
            } else {
                None
            },
            text_color,
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        },
    }
}

fn action_pill_style(_theme: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb8(49, 50, 68))),
            text_color: Color::from_rgb8(203, 166, 247),
            border: Border {
                color: Color::from_rgb8(203, 166, 247),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color::from_rgb8(69, 71, 90))),
            text_color: Color::from_rgb8(203, 166, 247),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        },
        _ => button::Style {
            background: Some(Background::Color(Color::from_rgb8(24, 24, 37))),
            text_color: Color::from_rgb8(205, 214, 244),
            border: Border {
                color: Color::from_rgb8(49, 50, 68),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        },
    }
}
