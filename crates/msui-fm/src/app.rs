use crate::file_entry::FileEntry;
use crate::file_list::{FileList, SortField};
use crate::operations::FileOperations;
use crate::sidebar::Sidebar;
use crate::status_bar::StatusBar;
use crate::toolbar::{Toolbar, ViewMode};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key, Modifiers};
use iced::widget::{button, column, container, row, text, text_input, stack};
use iced::{Alignment, Background, Border, Color, Element, Event, Length, Task, Theme, event};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum ModalKind {
    NewFolder,
    NewFile,
    Rename(PathBuf, String),
    Delete(PathBuf, String),
}

pub struct FileManagerApp {
    pub current_path: PathBuf,
    pub history_back: Vec<PathBuf>,
    pub history_forward: Vec<PathBuf>,
    pub entries: Vec<FileEntry>,
    pub filtered_indices: Vec<usize>,
    pub selected_idx: Option<usize>,
    pub search_query: String,
    pub is_editing_address: bool,
    pub address_input: String,
    pub view_mode: ViewMode,
    pub sort_by: SortField,
    pub sort_ascending: bool,
    pub show_hidden: bool,
    pub status_message: Option<(String, bool)>,
    pub active_modal: Option<ModalKind>,
    pub modal_input: String,
    pub fuzzy_matcher: SkimMatcherV2,
}

#[derive(Debug, Clone)]
pub enum Message {
    NavigateTo(PathBuf),
    HistoryBack,
    HistoryForward,
    GoParent,
    Refresh,
    ToggleHidden,
    SetViewMode(ViewMode),
    ToggleViewMode,
    SetSortField(SortField),
    ToggleAddressMode,
    AddressInputChanged(String),
    AddressSubmitted,
    SearchInputChanged(String),
    ClearSearch,
    SelectItem(usize),
    OpenItem(usize),
    OpenTerminal,
    OpenNewFolderModal,
    OpenNewFileModal,
    OpenRenameModal,
    OpenDeleteModal,
    ModalInputChanged(String),
    ModalConfirm,
    ModalCancel,
    DirectoryLoaded(PathBuf, Result<Vec<FileEntry>, String>),
    StatusTimeout,
    IcedEvent(Event),
}

impl FileManagerApp {
    pub fn new(initial_path: Option<PathBuf>) -> (Self, Task<Message>) {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home".into());
        let current_path = initial_path
            .and_then(|p| {
                if p.is_dir() {
                    Some(p)
                } else if let Some(parent) = p.parent() {
                    Some(parent.to_path_buf())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| PathBuf::from(home));

        let app = Self {
            address_input: current_path.to_string_lossy().to_string(),
            current_path: current_path.clone(),
            history_back: Vec::new(),
            history_forward: Vec::new(),
            entries: Vec::new(),
            filtered_indices: Vec::new(),
            selected_idx: None,
            search_query: String::new(),
            is_editing_address: false,
            view_mode: ViewMode::Details,
            sort_by: SortField::Name,
            sort_ascending: true,
            show_hidden: false,
            status_message: None,
            active_modal: None,
            modal_input: String::new(),
            fuzzy_matcher: SkimMatcherV2::default(),
        };

        let load_task = Task::perform(
            scan_directory(current_path.clone()),
            move |res| Message::DirectoryLoaded(current_path, res),
        );

        (app, load_task)
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        event::listen().map(Message::IcedEvent)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NavigateTo(path) => {
                if path.is_dir() && path != self.current_path {
                    self.history_back.push(self.current_path.clone());
                    self.history_forward.clear();
                    self.current_path = path.clone();
                    self.address_input = path.to_string_lossy().to_string();
                    self.search_query.clear();
                    self.selected_idx = None;

                    let target = path.clone();
                    return Task::perform(
                        scan_directory(target.clone()),
                        move |res| Message::DirectoryLoaded(target, res),
                    );
                }
            }
            Message::HistoryBack => {
                if let Some(prev) = self.history_back.pop() {
                    self.history_forward.push(self.current_path.clone());
                    self.current_path = prev.clone();
                    self.address_input = prev.to_string_lossy().to_string();
                    self.search_query.clear();
                    self.selected_idx = None;

                    let target = prev.clone();
                    return Task::perform(
                        scan_directory(target.clone()),
                        move |res| Message::DirectoryLoaded(target, res),
                    );
                }
            }
            Message::HistoryForward => {
                if let Some(next) = self.history_forward.pop() {
                    self.history_back.push(self.current_path.clone());
                    self.current_path = next.clone();
                    self.address_input = next.to_string_lossy().to_string();
                    self.search_query.clear();
                    self.selected_idx = None;

                    let target = next.clone();
                    return Task::perform(
                        scan_directory(target.clone()),
                        move |res| Message::DirectoryLoaded(target, res),
                    );
                }
            }
            Message::GoParent => {
                if let Some(parent) = self.current_path.parent() {
                    let parent_buf = parent.to_path_buf();
                    return self.update(Message::NavigateTo(parent_buf));
                }
            }
            Message::Refresh => {
                let target = self.current_path.clone();
                return Task::perform(
                    scan_directory(target.clone()),
                    move |res| Message::DirectoryLoaded(target, res),
                );
            }
            Message::ToggleHidden => {
                self.show_hidden = !self.show_hidden;
                self.apply_filter();
            }
            Message::SetViewMode(mode) => {
                self.view_mode = mode;
            }
            Message::ToggleViewMode => {
                self.view_mode = match self.view_mode {
                    ViewMode::Details => ViewMode::Grid,
                    ViewMode::Grid => ViewMode::Details,
                };
            }
            Message::SetSortField(field) => {
                if self.sort_by == field {
                    self.sort_ascending = !self.sort_ascending;
                } else {
                    self.sort_by = field;
                    self.sort_ascending = true;
                }
                self.sort_entries();
                self.apply_filter();
            }
            Message::ToggleAddressMode => {
                self.is_editing_address = !self.is_editing_address;
                if self.is_editing_address {
                    self.address_input = self.current_path.to_string_lossy().to_string();
                }
            }
            Message::AddressInputChanged(val) => {
                self.address_input = val;
            }
            Message::AddressSubmitted => {
                let mut path = PathBuf::from(&self.address_input);
                if self.address_input.starts_with("~/") {
                    if let Ok(home) = std::env::var("HOME") {
                        path = PathBuf::from(home).join(&self.address_input[2..]);
                    }
                }
                self.is_editing_address = false;
                if path.is_dir() {
                    return self.update(Message::NavigateTo(path));
                } else {
                    self.status_message = Some((format!("Invalid directory: {}", path.display()), true));
                }
            }
            Message::SearchInputChanged(query) => {
                self.search_query = query;
                self.apply_filter();
            }
            Message::ClearSearch => {
                self.search_query.clear();
                self.apply_filter();
            }
            Message::SelectItem(filtered_idx) => {
                if filtered_idx < self.filtered_indices.len() {
                    self.selected_idx = Some(filtered_idx);
                }
            }
            Message::OpenItem(filtered_idx) => {
                if let Some(&entry_idx) = self.filtered_indices.get(filtered_idx) {
                    if let Some(entry) = self.entries.get(entry_idx) {
                        let path = entry.path.clone();
                        if entry.is_dir {
                            return self.update(Message::NavigateTo(path));
                        } else {
                            if let Err(e) = FileOperations::open_default(&path) {
                                self.status_message = Some((format!("Failed to open file: {e}"), true));
                            }
                        }
                    }
                }
            }
            Message::OpenTerminal => {
                if let Err(e) = FileOperations::open_terminal(&self.current_path) {
                    self.status_message = Some((format!("Failed to launch terminal: {e}"), true));
                } else {
                    self.status_message = Some(("Terminal launched in current directory".into(), false));
                }
            }
            Message::OpenNewFolderModal => {
                self.active_modal = Some(ModalKind::NewFolder);
                self.modal_input = "New Folder".into();
            }
            Message::OpenNewFileModal => {
                self.active_modal = Some(ModalKind::NewFile);
                self.modal_input = "new_file.txt".into();
            }
            Message::OpenRenameModal => {
                if let Some(sel) = self.selected_idx {
                    if let Some(&entry_idx) = self.filtered_indices.get(sel) {
                        if let Some(entry) = self.entries.get(entry_idx) {
                            self.active_modal = Some(ModalKind::Rename(entry.path.clone(), entry.name.clone()));
                            self.modal_input = entry.name.clone();
                        }
                    }
                }
            }
            Message::OpenDeleteModal => {
                if let Some(sel) = self.selected_idx {
                    if let Some(&entry_idx) = self.filtered_indices.get(sel) {
                        if let Some(entry) = self.entries.get(entry_idx) {
                            self.active_modal = Some(ModalKind::Delete(entry.path.clone(), entry.name.clone()));
                        }
                    }
                }
            }
            Message::ModalInputChanged(val) => {
                self.modal_input = val;
            }
            Message::ModalConfirm => {
                if let Some(modal) = self.active_modal.take() {
                    match modal {
                        ModalKind::NewFolder => {
                            match FileOperations::create_directory(&self.current_path, &self.modal_input) {
                                Ok(p) => {
                                    self.status_message = Some((format!("Created folder: {}", p.file_name().unwrap_or_default().to_string_lossy()), false));
                                    return self.update(Message::Refresh);
                                }
                                Err(e) => {
                                    self.status_message = Some((format!("Create folder error: {e}"), true));
                                }
                            }
                        }
                        ModalKind::NewFile => {
                            match FileOperations::create_empty_file(&self.current_path, &self.modal_input) {
                                Ok(p) => {
                                    self.status_message = Some((format!("Created file: {}", p.file_name().unwrap_or_default().to_string_lossy()), false));
                                    return self.update(Message::Refresh);
                                }
                                Err(e) => {
                                    self.status_message = Some((format!("Create file error: {e}"), true));
                                }
                            }
                        }
                        ModalKind::Rename(old_path, _) => {
                            match FileOperations::rename_item(&old_path, &self.modal_input) {
                                Ok(p) => {
                                    self.status_message = Some((format!("Renamed to: {}", p.file_name().unwrap_or_default().to_string_lossy()), false));
                                    return self.update(Message::Refresh);
                                }
                                Err(e) => {
                                    self.status_message = Some((format!("Rename error: {e}"), true));
                                }
                            }
                        }
                        ModalKind::Delete(path, name) => {
                            match FileOperations::delete_or_trash(&path) {
                                Ok(()) => {
                                    self.status_message = Some((format!("Deleted \"{name}\""), false));
                                    return self.update(Message::Refresh);
                                }
                                Err(e) => {
                                    self.status_message = Some((format!("Delete error: {e}"), true));
                                }
                            }
                        }
                    }
                }
            }
            Message::ModalCancel => {
                self.active_modal = None;
                self.modal_input.clear();
            }
            Message::DirectoryLoaded(loaded_path, result) => {
                if loaded_path == self.current_path {
                    match result {
                        Ok(entries) => {
                            self.entries = entries;
                            self.sort_entries();
                            self.apply_filter();
                        }
                        Err(e) => {
                            self.status_message = Some((format!("Directory read error: {e}"), true));
                            self.entries.clear();
                            self.filtered_indices.clear();
                            self.selected_idx = None;
                        }
                    }
                }
            }
            Message::StatusTimeout => {
                self.status_message = None;
            }
            Message::IcedEvent(Event::Keyboard(KeyboardEvent::KeyPressed { key, modifiers, .. })) => {
                return self.handle_keyboard(key, modifiers);
            }
            _ => {}
        }

        Task::none()
    }

    fn handle_keyboard(&mut self, key: Key, modifiers: Modifiers) -> Task<Message> {
        // Modal active handles Escape & Enter
        if self.active_modal.is_some() {
            match key {
                Key::Named(Named::Escape) => return Task::done(Message::ModalCancel),
                Key::Named(Named::Enter) => return Task::done(Message::ModalConfirm),
                _ => return Task::none(),
            }
        }

        // If editing address bar
        if self.is_editing_address {
            match key {
                Key::Named(Named::Escape) => return Task::done(Message::ToggleAddressMode),
                Key::Named(Named::Enter) => return Task::done(Message::AddressSubmitted),
                _ => return Task::none(),
            }
        }

        // Global shortcuts with Ctrl
        if modifiers.control() {
            match key {
                Key::Character(ref s) if s.eq_ignore_ascii_case("h") => {
                    return Task::done(Message::ToggleHidden);
                }
                Key::Character(ref s) if s.eq_ignore_ascii_case("l") => {
                    return Task::done(Message::ToggleAddressMode);
                }
                Key::Character(ref s) if s.eq_ignore_ascii_case("f") => {
                    return Task::none();
                }
                Key::Character(ref s) if s.eq_ignore_ascii_case("t") => {
                    return Task::done(Message::OpenTerminal);
                }
                Key::Character(ref s) if s.eq_ignore_ascii_case("r") => {
                    return Task::done(Message::Refresh);
                }
                Key::Character(ref s) if s.eq_ignore_ascii_case("n") => {
                    if modifiers.shift() {
                        return Task::done(Message::OpenNewFolderModal);
                    }
                }
                Key::Character(ref s) if s.eq_ignore_ascii_case("c") => {
                    if let Some(sel) = self.selected_idx {
                        if let Some(&entry_idx) = self.filtered_indices.get(sel) {
                            if let Some(entry) = self.entries.get(entry_idx) {
                                FileOperations::copy_path_to_clipboard(&entry.path);
                                self.status_message = Some((format!("Copied path: {}", entry.path.display()), false));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Alt navigation shortcuts
        if modifiers.alt() {
            match key {
                Key::Named(Named::ArrowLeft) => return Task::done(Message::HistoryBack),
                Key::Named(Named::ArrowRight) => return Task::done(Message::HistoryForward),
                Key::Named(Named::ArrowUp) => return Task::done(Message::GoParent),
                _ => {}
            }
        }

        // Plain keybindings when not in input
        match key {
            Key::Named(Named::F5) => return Task::done(Message::Refresh),
            Key::Named(Named::F2) => return Task::done(Message::OpenRenameModal),
            Key::Named(Named::Delete) => return Task::done(Message::OpenDeleteModal),
            Key::Named(Named::Backspace) => return Task::done(Message::GoParent),
            Key::Named(Named::Escape) => {
                if !self.search_query.is_empty() {
                    return Task::done(Message::ClearSearch);
                }
                self.selected_idx = None;
            }
            Key::Named(Named::ArrowDown) => {
                self.move_selection(1);
            }
            Key::Named(Named::ArrowUp) => {
                self.move_selection(-1);
            }
            Key::Named(Named::Enter) => {
                if let Some(sel) = self.selected_idx {
                    return Task::done(Message::OpenItem(sel));
                }
            }
            Key::Character(ref s) => match s.as_str() {
                "j" => self.move_selection(1),
                "k" => self.move_selection(-1),
                "l" => {
                    if let Some(sel) = self.selected_idx {
                        return Task::done(Message::OpenItem(sel));
                    }
                }
                "h" => return Task::done(Message::GoParent),
                _ => {}
            },
            _ => {}
        }

        Task::none()
    }

    fn move_selection(&mut self, delta: isize) {
        if self.filtered_indices.is_empty() {
            self.selected_idx = None;
            return;
        }

        let new_idx = match self.selected_idx {
            Some(cur) => {
                let next = cur as isize + delta;
                if next < 0 {
                    0
                } else if next >= self.filtered_indices.len() as isize {
                    self.filtered_indices.len() - 1
                } else {
                    next as usize
                }
            }
            None => {
                if delta > 0 {
                    0
                } else {
                    self.filtered_indices.len() - 1
                }
            }
        };

        self.selected_idx = Some(new_idx);
    }

    fn sort_entries(&mut self) {
        let asc = self.sort_ascending;
        match self.sort_by {
            SortField::Name => {
                self.entries.sort_by(|a, b| {
                    if a.is_dir != b.is_dir {
                        return b.is_dir.cmp(&a.is_dir); // Directories first
                    }
                    if asc {
                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                    } else {
                        b.name.to_lowercase().cmp(&a.name.to_lowercase())
                    }
                });
            }
            SortField::Size => {
                self.entries.sort_by(|a, b| {
                    if a.is_dir != b.is_dir {
                        return b.is_dir.cmp(&a.is_dir);
                    }
                    if asc {
                        a.size_bytes.cmp(&b.size_bytes)
                    } else {
                        b.size_bytes.cmp(&a.size_bytes)
                    }
                });
            }
            SortField::Modified => {
                self.entries.sort_by(|a, b| {
                    if a.is_dir != b.is_dir {
                        return b.is_dir.cmp(&a.is_dir);
                    }
                    if asc {
                        a.modified_time.cmp(&b.modified_time)
                    } else {
                        b.modified_time.cmp(&a.modified_time)
                    }
                });
            }
            SortField::Type => {
                self.entries.sort_by(|a, b| {
                    if a.is_dir != b.is_dir {
                        return b.is_dir.cmp(&a.is_dir);
                    }
                    if asc {
                        a.extension.cmp(&b.extension)
                    } else {
                        b.extension.cmp(&a.extension)
                    }
                });
            }
        }
    }

    fn apply_filter(&mut self) {
        self.filtered_indices.clear();
        let query = self.search_query.trim().to_lowercase();

        for (idx, entry) in self.entries.iter().enumerate() {
            if !self.show_hidden && entry.is_hidden {
                continue;
            }

            if query.is_empty() {
                self.filtered_indices.push(idx);
            } else if self.fuzzy_matcher.fuzzy_match(&entry.name, &query).is_some()
                || entry.name.to_lowercase().contains(&query)
            {
                self.filtered_indices.push(idx);
            }
        }

        // Adjust selection if it's out of bounds
        if let Some(sel) = self.selected_idx {
            if sel >= self.filtered_indices.len() {
                self.selected_idx = if self.filtered_indices.is_empty() {
                    None
                } else {
                    Some(self.filtered_indices.len() - 1)
                };
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let can_go_back = !self.history_back.is_empty();
        let can_go_forward = !self.history_forward.is_empty();
        let can_go_up = self.current_path.parent().is_some();

        let toolbar = Toolbar::view(
            &self.current_path,
            can_go_back,
            can_go_forward,
            can_go_up,
            self.is_editing_address,
            &self.address_input,
            &self.search_query,
            self.view_mode,
            self.show_hidden,
            Message::HistoryBack,
            Message::HistoryForward,
            Message::GoParent,
            Message::Refresh,
            Message::ToggleAddressMode,
            Message::AddressInputChanged,
            Message::AddressSubmitted,
            Message::SearchInputChanged,
            Message::ClearSearch,
            Message::ToggleViewMode,
            Message::ToggleHidden,
            Message::OpenTerminal,
            Message::OpenNewFolderModal,
            Message::NavigateTo,
        );

        let sidebar = Sidebar::view(&self.current_path, Message::NavigateTo);

        let file_view = FileList::view(
            &self.entries,
            &self.filtered_indices,
            self.selected_idx,
            self.view_mode,
            self.sort_by,
            self.sort_ascending,
            &self.search_query,
            Message::SelectItem,
            Message::OpenItem,
            Message::SetSortField,
        );

        let body_row = row![sidebar, file_view]
            .width(Length::Fill)
            .height(Length::Fill);

        let status_bar = StatusBar::view(
            &self.current_path,
            &self.entries,
            &self.filtered_indices,
            self.selected_idx,
            self.status_message.as_ref(),
        );

        let main_layout = column![toolbar, body_row, status_bar]
            .width(Length::Fill)
            .height(Length::Fill);

        // If modal dialog is open, overlay it
        if let Some(modal) = &self.active_modal {
            let modal_overlay = self.view_modal(modal);
            stack![main_layout, modal_overlay].into()
        } else {
            main_layout.into()
        }
    }

    fn view_modal(&self, modal: &ModalKind) -> Element<'_, Message> {
        let (title, action_label, is_input) = match modal {
            ModalKind::NewFolder => ("󰉍 Create New Folder", "Create", true),
            ModalKind::NewFile => ("󰈙 Create New File", "Create", true),
            ModalKind::Rename(_, _) => ("󰏫 Rename Item", "Rename", true),
            ModalKind::Delete(_, _) => ("󰆴 Confirm Deletion", "Delete", false),
        };

        let header = text(title)
            .size(16)
            .style(|_theme: &Theme| text::Style {
                color: Some(Color::from_rgb8(203, 166, 247)),
            });

        let mut content = column![header].spacing(14).align_x(Alignment::Center);

        if is_input {
            let input = text_input("Enter name...", &self.modal_input)
                .on_input(Message::ModalInputChanged)
                .on_submit(Message::ModalConfirm)
                .padding([8, 12])
                .size(14)
                .width(Length::Fixed(320.0));
            content = content.push(input);
        } else if let ModalKind::Delete(_, name) = modal {
            content = content.push(
                text(format!("Move \"{name}\" to FreeDesktop Trash?"))
                    .size(13)
                    .style(|_theme: &Theme| text::Style {
                        color: Some(Color::from_rgb8(243, 139, 168)),
                    }),
            );
        }

        let confirm_btn = button(text(action_label).size(13))
            .padding([8, 16])
            .on_press(Message::ModalConfirm)
            .style(|_theme: &Theme, _status| {
                button::Style {
                    background: Some(Background::Color(Color::from_rgb8(203, 166, 247))),
                    text_color: Color::from_rgb8(17, 17, 27),
                    border: Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            });

        let cancel_btn = button(text("Cancel").size(13))
            .padding([8, 16])
            .on_press(Message::ModalCancel)
            .style(|_theme: &Theme, _status| {
                button::Style {
                    background: Some(Background::Color(Color::from_rgb8(49, 50, 68))),
                    text_color: Color::from_rgb8(205, 214, 244),
                    border: Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            });

        let buttons_row = row![cancel_btn, confirm_btn].spacing(10);
        content = content.push(buttons_row);

        let modal_box = container(content)
            .padding(24)
            .style(|_theme: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgb8(30, 30, 46))), // Base
                border: Border {
                    color: Color::from_rgb8(203, 166, 247), // Mauve
                    width: 1.5,
                    radius: 12.0.into(),
                },
                shadow: iced::Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.6),
                    offset: iced::Vector::new(0.0, 8.0),
                    blur_radius: 24.0,
                },
                ..Default::default()
            });

        container(modal_box)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.55))),
                ..Default::default()
            })
            .into()
    }
}

async fn scan_directory(path: PathBuf) -> Result<Vec<FileEntry>, String> {
    tokio::task::spawn_blocking(move || {
        let read_dir = std::fs::read_dir(&path)
            .map_err(|e| format!("Cannot read directory: {e}"))?;

        let mut entries = Vec::new();
        for entry_res in read_dir {
            if let Ok(entry) = entry_res {
                let entry_path = entry.path();
                let meta = entry.metadata().ok();
                if let Some(file_entry) = FileEntry::from_path(&entry_path, meta.as_ref()) {
                    entries.push(file_entry);
                }
            }
        }

        Ok(entries)
    })
    .await
    .map_err(|e| format!("Worker panic: {e}"))?
}
