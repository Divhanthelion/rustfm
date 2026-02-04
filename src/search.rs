use egui::{Context, Window, ScrollArea, TextEdit, ProgressBar, RichText, Color32};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone)]
pub struct SearchResult {
    pub path: PathBuf,
    pub line_number: usize,
    pub line_content: String,
    pub matched_text: String,
}

pub struct SearchPanel {
    visible: bool,
    query: String,
    results: Arc<Mutex<Vec<SearchResult>>>,
    is_searching: Arc<Mutex<bool>>,
    pending_search: Option<String>,
    search_path: Option<PathBuf>,
    include_pattern: String,
    exclude_pattern: String,
    case_sensitive: bool,
    search_in_progress: bool,
}

impl SearchPanel {
    pub fn new() -> Self {
        Self {
            visible: false,
            query: String::new(),
            results: Arc::new(Mutex::new(Vec::new())),
            is_searching: Arc::new(Mutex::new(false)),
            pending_search: None,
            search_path: None,
            include_pattern: String::from("*"),
            exclude_pattern: String::from(".git,node_modules,target"),
            case_sensitive: false,
            search_in_progress: false,
        }
    }
    
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
    
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    pub fn set_search_path(&mut self, path: PathBuf) {
        self.search_path = Some(path);
    }
    
    pub fn check_search(&mut self) -> Option<PathBuf> {
        self.pending_search.take().map(|_| self.search_path.clone()).flatten()
    }
    
    fn execute_search(&mut self) {
        if self.query.is_empty() {
            return;
        }
        
        let query = self.query.clone();
        let path = self.search_path.clone().unwrap_or_else(|| PathBuf::from("."));
        let case_sensitive = self.case_sensitive;
        let results = Arc::clone(&self.results);
        let is_searching = Arc::clone(&self.is_searching);
        
        // Clear previous results
        if let Ok(mut r) = results.lock() {
            r.clear();
        }
        
        // Set searching flag
        if let Ok(mut s) = is_searching.lock() {
            *s = true;
        }
        self.search_in_progress = true;
        
        // Spawn search thread
        thread::spawn(move || {
            search_directory(&path, &query, case_sensitive, &results);
            
            if let Ok(mut s) = is_searching.lock() {
                *s = false;
            }
        });
    }
    
    pub fn render(&mut self, ctx: &Context) {
        let mut execute_search = false;
        let is_searching_flag = self.is_searching.lock().map(|s| *s).unwrap_or(false);
        
        // Build the window
        let mut window_open = self.visible;
        Window::new("🔍 Search")
            .open(&mut window_open)
            .default_size([600.0, 500.0])
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Search input
                    ui.horizontal(|ui| {
                        ui.label("Search:");
                        let response = ui.add(
                            TextEdit::singleline(&mut self.query)
                                .desired_width(300.0)
                                .hint_text("Type to search...")
                        );
                        
                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            execute_search = true;
                        }
                        
                        if ui.button("Search").clicked() {
                            execute_search = true;
                        }
                        
                        if self.search_in_progress {
                            if ui.button("⏹").clicked() {
                                // TODO: Cancel search
                            }
                        }
                    });
                    
                    // Options
                    ui.collapsing("Options", |ui| {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.case_sensitive, "Case sensitive");
                        });
                        ui.horizontal(|ui| {
                            ui.label("Include:");
                            ui.text_edit_singleline(&mut self.include_pattern);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Exclude:");
                            ui.text_edit_singleline(&mut self.exclude_pattern);
                        });
                    });
                    
                    ui.separator();
                    
                    // Progress / status
                    if is_searching_flag {
                        ui.add(ProgressBar::new(0.5).animate(true));
                        ui.label("Searching...");
                    } else {
                        let result_count = self.results.lock().map(|r| r.len()).unwrap_or(0);
                        ui.label(format!("Found {} results", result_count));
                    }
                    
                    ui.separator();
                    
                    // Results
                    let query_clone = self.query.clone();
                    let case_sensitive = self.case_sensitive;
                    
                    ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            if let Ok(results) = self.results.lock() {
                                for (i, result) in results.iter().enumerate() {
                                    ui.group(|ui| {
                                        // File path and line number
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{}:{}",
                                                    result.path.display(),
                                                    result.line_number
                                                ))
                                                .color(Color32::YELLOW)
                                                .monospace()
                                            );
                                        });
                                        
                                        // Line content with highlighted match
                                        let line = &result.line_content;
                                        let query = &query_clone;
                                        
                                        // Simple highlight
                                        if let Some(pos) = if case_sensitive {
                                            line.find(query)
                                        } else {
                                            line.to_lowercase().find(&query.to_lowercase())
                                        } {
                                            let before = &line[..pos];
                                            let matched = &line[pos..pos + query.len()];
                                            let after = &line[pos + query.len()..];
                                            
                                            ui.horizontal(|ui| {
                                                ui.monospace(before);
                                                ui.label(
                                                    RichText::new(matched)
                                                        .color(Color32::BLACK)
                                                        .background_color(Color32::YELLOW)
                                                        .monospace()
                                                );
                                                ui.monospace(after);
                                            });
                                        } else {
                                            ui.monospace(line);
                                        }
                                    });
                                    
                                    if i < results.len() - 1 {
                                        ui.separator();
                                    }
                                }
                            }
                        });
                });
            });
        
        self.visible = window_open;
        
        // Execute search if requested (after window closes to avoid borrow issues)
        if execute_search {
            self.execute_search();
        }
    }
}

fn search_directory(
    path: &PathBuf,
    query: &str,
    case_sensitive: bool,
    results: &Arc<Mutex<Vec<SearchResult>>>,
) {
    use walkdir::WalkDir;
    
    let walker = WalkDir::new(path)
        .follow_links(false)
        .max_depth(10)
        .into_iter();
    
    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        
        // Skip directories
        if !entry.file_type().is_file() {
            continue;
        }
        
        // Skip binary files and large files
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        
        if metadata.len() > 10 * 1024 * 1024 {
            // Skip files larger than 10MB
            continue;
        }
        
        // Try to read and search the file
        if let Ok(content) = std::fs::read_to_string(path) {
            for (line_num, line) in content.lines().enumerate() {
                let found = if case_sensitive {
                    line.contains(query)
                } else {
                    line.to_lowercase().contains(&query.to_lowercase())
                };
                
                if found {
                    let result = SearchResult {
                        path: path.to_path_buf(),
                        line_number: line_num + 1,
                        line_content: line.to_string(),
                        matched_text: query.to_string(),
                    };
                    
                    if let Ok(mut r) = results.lock() {
                        r.push(result);
                        
                        // Limit results
                        if r.len() >= 1000 {
                            return;
                        }
                    }
                }
            }
        }
    }
}
