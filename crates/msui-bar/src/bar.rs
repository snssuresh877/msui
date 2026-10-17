use chrono::Local;
use iced::widget::{button, container, row, space, text};
use iced::{Alignment, Element, Length, Task as Command};
use iced_exwlshell::get_wlconnection;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, Layer, LayerSize};
use iced_exwlshell::settings::{ExWlSettings, LayerShellSettings, StartMode};
use iced_exwlshell::to_layer_message;
use iced_wayland_subscriber::OutputId;
use iced_wayland_subscriber::shell::{ShellEvent, ShellReceiver};
use iced_wayland_subscriber::workspace::{
    State, Workspace, WorkspaceCapabilities, WorkspaceEvent, WorkspaceGroup,
    WorkspaceId, WorkspaceSnapshot,
};

use msui_core::{BatteryInfo, BatteryMonitor, MsuiConfig, SystemMetrics, SystemMonitor};
use std::sync::Arc;
use std::time::Duration;

pub fn run_bar(config: MsuiConfig) -> Result<(), iced_exwlshell::Error> {
    let (shell_broadcast, shell_events) = iced_wayland_subscriber::shell::channel();

    let height = config.bar.height;
    let layer = match config.bar.layer.to_lowercase().as_str() {
        "overlay" => Layer::Overlay,
        "bottom" => Layer::Bottom,
        "background" => Layer::Background,
        _ => Layer::Top,
    };

    let anchor = match config.bar.anchor.to_lowercase().as_str() {
        "bottom" => Anchor::Bottom | Anchor::Left | Anchor::Right,
        _ => Anchor::Top | Anchor::Left | Anchor::Right,
    };

    let cfg_clone = config.clone();

    application(
        move || Bar::new(cfg_clone.clone(), shell_events.clone()),
        Bar::namespace,
        Bar::update,
        Bar::view,
    )
    .subscription(Bar::subscription)
    .wl_settings(ExWlSettings {
        layer_settings: LayerShellSettings {
            size: LayerSize::fill_width(height),
            exclusive_zone: height as i32,
            anchor,
            layer,
            start_mode: StartMode::Active,
            ..Default::default()
        },
        shell_broadcast,
        ..Default::default()
    })
    .run()
}

pub struct Bar {
    config: MsuiConfig,
    shell_events: ShellReceiver,
    workspaces: Option<Arc<WorkspaceSnapshot>>,
    output: Option<OutputId>,
    battery_monitor: BatteryMonitor,
    system_monitor: SystemMonitor,
    current_time: String,
    battery_info: Option<BatteryInfo>,
    system_metrics: SystemMetrics,
    hyprland_workspace: i32,
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    ClockTick,
    MetricsTick,
    WorkspacesUpdated(Arc<WorkspaceSnapshot>),
    ActivateWorkspace(WorkspaceId),
    OutputChanged(Option<OutputId>),
    OpenLauncher,
    OpenTerminal,
    Ignored,
}

impl Bar {
    fn new(config: MsuiConfig, shell_events: ShellReceiver) -> Self {
        let battery_monitor = BatteryMonitor::new();
        let mut system_monitor = SystemMonitor::new();
        let battery_info = battery_monitor.read();
        let system_metrics = system_monitor.sample();
        let current_time = Local::now().format(&config.bar.time_format).to_string();

        Self {
            config,
            shell_events,
            workspaces: None,
            output: None,
            battery_monitor,
            system_monitor,
            current_time,
            battery_info,
            system_metrics,
            hyprland_workspace: 1,
        }
    }

    fn namespace() -> String {
        String::from("msui-bar")
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        let clock = iced::time::every(Duration::from_secs(1)).map(|_| Message::ClockTick);
        let metrics = iced::time::every(Duration::from_secs(self.config.bar.sysinfo_refresh_seconds))
            .map(|_| Message::MetricsTick);

        let wayland_workspaces = iced_wayland_subscriber::workspace::listen(get_wlconnection()).map(
            |event| match event {
                WorkspaceEvent::Updated(snapshot) => Message::WorkspacesUpdated(snapshot),
                _ => Message::Ignored,
            },
        );

        let shell_listener = self.shell_events.listen().filter_map(|event| match event {
            ShellEvent::WindowOutputChanged { output, .. } => {
                Some(Message::OutputChanged(output.as_ref().map(OutputId::from)))
            }
            _ => None,
        });

        iced::Subscription::batch([clock, metrics, wayland_workspaces, shell_listener])
    }

    fn group(&self) -> Option<&WorkspaceGroup> {
        self.workspaces.as_ref()?.group_for_output(self.output?)
    }

    fn occupant(&self, slot: u32) -> Option<&Workspace> {
        let snapshot = self.workspaces.as_ref()?;
        let group = self.group()?;
        let slot = slot.to_string();
        snapshot
            .workspaces_in(&group.id)
            .find(|workspace| workspace.name == slot)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ClockTick => {
                self.current_time = Local::now().format(&self.config.bar.time_format).to_string();
            }
            Message::MetricsTick => {
                self.system_metrics = self.system_monitor.sample();
                if self.config.bar.show_battery {
                    self.battery_info = self.battery_monitor.read();
                }
            }
            Message::WorkspacesUpdated(snapshot) => {
                self.workspaces = Some(snapshot);
            }
            Message::OutputChanged(output) => {
                self.output = output;
            }
            Message::ActivateWorkspace(id) => {
                if let Some(snapshot) = &self.workspaces {
                    let _ = snapshot.activate(&id);
                }
            }
            Message::OpenLauncher => {
                let _ = std::process::Command::new("msui")
                    .arg("launcher")
                    .spawn();
            }
            Message::OpenTerminal => {
                let term = &self.config.launcher.terminal;
                let _ = std::process::Command::new(term).spawn();
            }
            Message::Ignored => {}
            _ => {}
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        // --- Left Section: Brand / Launcher Button + Workspaces ---
        let brand_btn = button(text("  msUI  "))
            .style(button::primary)
            .on_press(Message::OpenLauncher);

        let mut left_row = row![brand_btn].spacing(8).align_y(Alignment::Center);

        if self.config.bar.show_workspaces {
            let mut ws_row = row![].spacing(4).align_y(Alignment::Center);

            if self.workspaces.is_some() {
                // Wayland ext-workspace-v1
                for slot in 1..=8 {
                    let occupant = self.occupant(slot);
                    let is_active = occupant.map_or(false, |w| w.state.contains(State::Active));
                    let label = if is_active {
                        format!(" [{slot}] ")
                    } else {
                        format!("  {slot}  ")
                    };

                    let btn_style = if is_active {
                        button::primary
                    } else {
                        button::secondary
                    };

                    let press = occupant
                        .filter(|w| w.capabilities.contains(WorkspaceCapabilities::Activate))
                        .map(|w| Message::ActivateWorkspace(w.id.clone()));

                    ws_row = ws_row.push(
                        button(text(label))
                            .style(btn_style)
                            .on_press_maybe(press),
                    );
                }
            } else {
                // Fallback basic workspace buttons
                for slot in 1..=5 {
                    let is_active = slot == self.hyprland_workspace;
                    let label = format!("  {slot}  ");
                    let btn_style = if is_active {
                        button::primary
                    } else {
                        button::secondary
                    };
                    ws_row = ws_row.push(button(text(label)).style(btn_style));
                }
            }
            left_row = left_row.push(ws_row);
        }

        // --- Center Section: Clock ---
        let clock_widget = text(&self.current_time);

        // --- Right Section: Hardware & Status ---
        let mut right_row = row![].spacing(12).align_y(Alignment::Center);

        if self.config.bar.show_cpu {
            let cpu_str = format!("CPU {:.0}%", self.system_metrics.cpu_usage_percent);
            right_row = right_row.push(text(cpu_str));
        }

        if self.config.bar.show_memory {
            let ram_str = format!(
                "RAM {:.1}/{:.1}G",
                self.system_metrics.ram_used_mb as f32 / 1024.0,
                self.system_metrics.ram_total_mb as f32 / 1024.0
            );
            right_row = right_row.push(text(ram_str));
        }

        if self.config.bar.show_battery {
            if let Some(bat) = &self.battery_info {
                let charging_symbol = if bat.is_charging { "⚡" } else { "🔋" };
                let bat_str = format!("{} {}%", charging_symbol, bat.percentage);
                right_row = right_row.push(text(bat_str));
            }
        }

        let term_btn = button(text(" Term "))
            .style(button::secondary)
            .on_press(Message::OpenTerminal);
        right_row = right_row.push(term_btn);

        // Assemble full bar layout
        let bar_content = row![
            left_row,
            space().width(Length::Fill),
            clock_widget,
            space().width(Length::Fill),
            right_row,
        ]
        .width(Length::Fill)
        .align_y(Alignment::Center)
        .padding([2, 12]);

        container(bar_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
