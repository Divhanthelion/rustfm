use egui::{Ui, ScrollArea, Grid, RichText, Color32, Response, Sense, Vec2};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use chrono::{DateTime, Local};

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub icon: String,
}

impl FileEntry {
    pub fn from_path(path: PathBuf) -> Option<Self> {
        let metadata = std::fs::metadata(&path).ok()?;
        let name = path.file_name()?.to_string_lossy().to_string();
        let is_dir = metadata.is_dir();
        
        let icon = if is_dir {
            "📁".to_string()
        } else {
            get_file_icon(&name)
        };
        
        Some(Self {
            path,
            name,
            is_dir,
            size: if is_dir { 0 } else { metadata.len() },
            modified: metadata.modified().ok(),
            icon,
        })
    }
    
    pub fn format_size(&self) -> String {
        if self.is_dir {
            "--".to_string()
        } else {
            format_size(self.size)
        }
    }
    
    pub fn format_modified(&self) -> String {
        self.modified
            .and_then(|t| DateTime::<Local>::from(t).format("%Y-%m-%d %H:%M").to_string().into())
            .unwrap_or_else(|| "--".to_string())
    }
}

fn get_file_icon(name: &str) -> String {
    let ext = name.split('.').last().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "rs" => "🦀",
        "py" => "🐍",
        "js" | "ts" | "jsx" | "tsx" => "📜",
        "html" | "htm" => "🌐",
        "css" => "🎨",
        "json" | "xml" | "yaml" | "yml" | "toml" => "⚙️",
        "md" | "txt" | "doc" | "docx" => "📝",
        "pdf" => "📄",
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" => "🖼️",
        "mp3" | "wav" | "flac" | "aac" | "ogg" => "🎵",
        "mp4" | "avi" | "mkv" | "mov" | "wmv" => "🎬",
        "zip" | "tar" | "gz" | "bz2" | "7z" | "rar" => "🗜️",
        "exe" | "bin" | "app" => "⚙️",
        "sh" | "bash" | "zsh" | "fish" => "🐚",
        _ => "📄",
    }.to_string()
}

fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_idx = 0;
    
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    
    format!("{:.1} {}", size, UNITS[unit_idx])
}

pub struct ExplorerPanel {
    current_path: PathBuf,
    entries: Vec<FileEntry>,
    selected: Option<PathBuf>,
    view_mode: ViewMode,
    pending_navigation: Option<PathBuf>,
    sort_by: SortBy,
    sort_descending: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    Icons,
    List,
}

#[derive(Clone, Copy, PartialEq)]
enum SortBy {
    Name,
    Size,
    Modified,
}

impl ExplorerPanel {
    pub fn new(initial_path: PathBuf) -> Self {
        let mut panel = Self {
            current_path: initial_path.clone(),
            entries: Vec::new(),
            selected: None,
            view_mode: ViewMode::Icons,
            pending_navigation: None,
            sort_by: SortBy::Name,
            sort_descending: false,
        };
        panel.refresh();
        panel
    }
    
    pub fn navigate_to(&mut self, path: PathBuf) {
        self.current_path = path;
        self.refresh();
    }
    
    pub fn check_navigation(&mut self) -> Option<PathBuf> {
        self.pending_navigation.take()
    }
    
    pub fn item_count(&self) -> usize {
        self.entries.len()
    }
    
    pub fn refresh(&mut self) {
        self.entries.clear();
        
        if let Ok(entries) = std::fs::read_dir(&self.current_path) {
            for entry in entries.flatten() {
                if let Some(file_entry) = FileEntry::from_path(entry.path()) {
                    self.entries.push(file_entry);
                }
            }
        }
        
        self.sort_entries();
    }
    
    fn sort_entries(&mut self) {
        match self.sort_by {
            SortBy::Name => {
                self.entries.sort_by(|a, b| {
                    // Directories first
                    match (a.is_dir, b.is_dir) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                    }
                });
            }
            SortBy::Size => {
                self.entries.sort_by(|a, b| a.size.cmp(&b.size));
            }
            SortBy::Modified => {
                self.entries.sort_by(|a, b| a.modified.cmp(&b.modified));
            }
        }
        
        if self.sort_descending {
            self.entries.reverse();
        }
    }
    
    pub fn render(&mut self, ui: &mut Ui) {
        // View controls
        ui.horizontal(|ui| {
            ui.label("View:");
            if ui.selectable_label(self.view_mode == ViewMode::Icons, "Icons").clicked() {
                self.view_mode = ViewMode::Icons;
            }
            if ui.selectable_label(self.view_mode == ViewMode::List, "List").clicked() {
                self.view_mode = ViewMode::List;
            }
            ui.separator();
            
            // Sort controls
            ui.label("Sort:");
            egui::ComboBox::from_id_source("sort_by")
                .selected_text(match self.sort_by {
                    SortBy::Name => "Name",
                    SortBy::Size => "Size",
                    SortBy::Modified => "Modified",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_label(self.sort_by == SortBy::Name, "Name").clicked() {
                        self.sort_by = SortBy::Name;
                        self.sort_entries();
                    }
                    if ui.selectable_label(self.sort_by == SortBy::Size, "Size").clicked() {
                        self.sort_by = SortBy::Size;
                        self.sort_entries();
                    }
                    if ui.selectable_label(self.sort_by == SortBy::Modified, "Modified").clicked() {
                        self.sort_by = SortBy::Modified;
                        self.sort_entries();
                    }
                });
            
            if ui.button(if self.sort_descending { "▼" } else { "▲" }).clicked() {
                self.sort_descending = !self.sort_descending;
                self.sort_entries();
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄").clicked() {
                    self.refresh();
                }
            });
        });
        
        ui.separator();
        
        // Content area
        match self.view_mode {
            ViewMode::Icons => self.render_icon_view(ui),
            ViewMode::List => self.render_list_view(ui),
        }
    }
    
    fn render_icon_view(&mut self, ui: &mut Ui) {
        let available_width = ui.available_width();
        let icon_size = 80.0;
        let spacing = 10.0;
        let columns = ((available_width + spacing) / (icon_size + spacing)) as usize;
        let columns = columns.max(1);
        
        let entries = self.entries.clone();
        let mut clicked_entry: Option<usize> = None;
        let mut double_clicked_entry: Option<usize> = None;
        
        ScrollArea::vertical().show(ui, |ui| {
            Grid::new("icon_grid")
                .spacing([spacing, spacing])
                .min_col_width(icon_size)
                .max_col_width(icon_size)
                .show(ui, |ui| {
                    for (i, entry) in entries.iter().enumerate() {
                        if i > 0 && i % columns == 0 {
                            ui.end_row();
                        }
                        
                        let response = self.render_icon_item(ui, entry, icon_size);
                        if response.clicked() {
                            clicked_entry = Some(i);
                        }
                        if response.double_clicked() {
                            double_clicked_entry = Some(i);
                        }
                    }
                });
        });
        
        // Apply interactions after the loop
        if let Some(idx) = clicked_entry {
            if let Some(entry) = entries.get(idx) {
                self.selected = Some(entry.path.clone());
                if entry.is_dir {
                    self.pending_navigation = Some(entry.path.clone());
                }
            }
        }
        
        if let Some(idx) = double_clicked_entry {
            if let Some(entry) = entries.get(idx) {
                if !entry.is_dir {
                    open_file(&entry.path);
                }
            }
        }
    }
    
    fn render_icon_item(&self, ui: &mut Ui, entry: &FileEntry, size: f32) -> Response {
        let is_selected = self.selected.as_ref() == Some(&entry.path);
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(size, size + 30.0),
            Sense::click(),
        );
        
        let visuals = ui.style().interact(&response);
        let bg_color = if is_selected {
            ui.visuals().selection.bg_fill
        } else if response.hovered() {
            visuals.bg_fill
        } else {
            Color32::TRANSPARENT
        };
        
        ui.painter().rect_filled(rect, 4.0, bg_color);
        
        // Icon
        let icon_pos = rect.center() - Vec2::new(0.0, 10.0);
        ui.painter().text(
            icon_pos,
            egui::Align2::CENTER_CENTER,
            &entry.icon,
            egui::FontId::proportional(32.0),
            ui.visuals().text_color(),
        );
        
        // Name
        let text_pos = rect.left_bottom() - Vec2::new(0.0, 5.0);
        let truncated_name = if entry.name.len() > 15 {
            format!("{}...", &entry.name[..12])
        } else {
            entry.name.clone()
        };
        ui.painter().text(
            text_pos,
            egui::Align2::LEFT_BOTTOM,
            truncated_name,
            egui::FontId::proportional(11.0),
            ui.visuals().text_color(),
        );
        
        response
    }
    
    fn render_list_view(&mut self, ui: &mut Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            Grid::new("list_grid")
                .num_columns(4)
                .striped(true)
                .show(ui, |ui| {
                    // Header
                    ui.strong("Name");
                    ui.strong("Size");
                    ui.strong("Modified");
                    ui.strong("Kind");
                    ui.end_row();
                    
                    for entry in &self.entries {
                        let is_selected = self.selected.as_ref() == Some(&entry.path);
                        
                        let mut name_text = RichText::new(format!("{} {}", entry.icon, entry.name));
                        if is_selected {
                            name_text = name_text.color(ui.visuals().selection.stroke.color);
                        }
                        
                        let response = ui.selectable_label(is_selected, name_text);
                        ui.label(entry.format_size());
                        ui.label(entry.format_modified());
                        ui.label(if entry.is_dir { "Folder" } else { "File" });
                        
                        if response.clicked() {
                            self.selected = Some(entry.path.clone());
                            if entry.is_dir {
                                self.pending_navigation = Some(entry.path.clone());
                            }
                        }
                        
                        if response.double_clicked() && !entry.is_dir {
                            open_file(&entry.path);
                        }
                        
                        ui.end_row();
                    }
                });
        });
    }
}

fn open_file(path: &Path) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg(path)
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(path)
            .spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", path.to_str().unwrap_or("")])
            .spawn();
    }
}
