use crate::explorer::ExplorerPanel;
use crate::search::SearchPanel;
use crate::terminal::TerminalPanel;
use eframe::Frame;
use egui::{Context, CentralPanel, TopBottomPanel, SidePanel, Ui};
use std::path::PathBuf;

pub struct FileExplorerApp {
    current_path: PathBuf,
    explorer: ExplorerPanel,
    terminal: TerminalPanel,
    search: SearchPanel,
    terminal_height: f32,
    show_terminal: bool,
    status_message: String,
}

impl FileExplorerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load previous app state if available
        if let Some(storage) = cc.storage {
            // TODO: Load persisted state
        }

        let current_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        
        Self {
            current_path: current_path.clone(),
            explorer: ExplorerPanel::new(current_path.clone()),
            terminal: TerminalPanel::new(current_path.clone()),
            search: SearchPanel::new(),
            terminal_height: 250.0,
            show_terminal: true,
            status_message: String::new(),
        }
    }

    fn navigate_to(&mut self, path: PathBuf) {
        self.current_path = path.clone();
        self.explorer.navigate_to(path.clone());
        self.terminal.set_directory(path.clone());
        self.status_message = format!("Navigated to: {}", path.display());
    }

    fn render_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Back/Forward buttons
            if ui.button("◀").clicked() {
                // TODO: Navigation history
            }
            if ui.button("▶").clicked() {
                // TODO: Navigation history
            }
            if ui.button("▲").clicked() {
                if let Some(parent) = self.current_path.parent() {
                    let parent = parent.to_path_buf();
                    self.navigate_to(parent);
                }
            }
            
            ui.separator();
            
            // Path breadcrumb
            ui.label("📁");
            let components: Vec<_> = self.current_path.components().collect();
            let mut click_targets: Vec<(String, PathBuf)> = Vec::new();
            for (i, component) in components.iter().enumerate() {
                let name = component.as_os_str().to_string_lossy();
                let mut path_so_far = PathBuf::new();
                for c in &components[..=i] {
                    path_so_far.push(c);
                }
                click_targets.push((name.to_string(), path_so_far));
            }
            for (i, (name, path)) in click_targets.iter().enumerate() {
                if i > 0 {
                    ui.label("/");
                }
                if ui.selectable_label(false, name.as_str()).clicked() {
                    self.navigate_to(path.clone());
                }
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Search toggle
                if ui.button("🔍").clicked() {
                    self.search.toggle();
                }
                // Terminal toggle
                if ui.button(if self.show_terminal { "🖥️" } else { "🖥️" }).clicked() {
                    self.show_terminal = !self.show_terminal;
                }
            });
        });
    }

    fn render_sidebar(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.heading("Favorites");
            ui.separator();
            
            let favorites = vec![
                ("🏠", "Home", dirs::home_dir()),
                ("🖥️", "Desktop", dirs::desktop_dir()),
                ("📄", "Documents", dirs::document_dir()),
                ("⬇️", "Downloads", dirs::download_dir()),
                ("🖼️", "Pictures", dirs::picture_dir()),
                ("🎵", "Music", dirs::audio_dir()),
                ("🎬", "Videos", dirs::video_dir()),
            ];
            
            for (icon, name, path) in favorites {
                if let Some(path) = path {
                    if ui.selectable_label(
                        self.current_path == path,
                        format!("{} {}", icon, name)
                    ).clicked() {
                        self.navigate_to(path);
                    }
                }
            }
            
            ui.separator();
            ui.heading("Devices");
            // TODO: List mounted volumes
        });
    }

    fn render_status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(&self.status_message);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let item_count = self.explorer.item_count();
                ui.label(format!("{} items", item_count));
            });
        });
    }
}

impl eframe::App for FileExplorerApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        // Update terminal
        self.terminal.update(ctx);
        
        // Handle search
        if let Some(search_path) = self.search.check_search() {
            // TODO: Execute search
        }
        
        // Handle explorer navigation
        if let Some(new_path) = self.explorer.check_navigation() {
            self.navigate_to(new_path);
        }

        // Toolbar
        TopBottomPanel::top("toolbar").show(ctx, |ui| {
            self.render_toolbar(ui);
        });

        // Status bar
        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            self.render_status_bar(ui);
        });

        // Terminal panel (if visible)
        if self.show_terminal {
            TopBottomPanel::bottom("terminal")
                .resizable(true)
                .default_height(self.terminal_height)
                .height_range(100.0..=500.0)
                .show(ctx, |ui| {
                    self.terminal.render(ui);
                });
        }

        // Sidebar
        SidePanel::left("sidebar")
            .resizable(true)
            .default_width(150.0)
            .show(ctx, |ui| {
                self.render_sidebar(ui);
            });

        // Main content area
        CentralPanel::default().show(ctx, |ui| {
            self.explorer.render(ui);
        });

        // Search modal
        if self.search.is_visible() {
            self.search.render(ctx);
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // TODO: Persist state
    }
}
