use egui::{Ui, ScrollArea, TextEdit, Color32, RichText, Key, Modifiers};
use portable_pty::{CommandBuilder, NativePtySystem, PtyPair, PtySize, PtySystem};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::thread;
use termwiz::surface::Surface;

const TERMINAL_COLS: u16 = 80;
const TERMINAL_ROWS: u16 = 24;

pub struct TerminalPanel {
    current_dir: PathBuf,
    _surface: Surface,
    scrollback: Vec<String>,
    input_buffer: String,
    pty_pair: Option<Box<PtyPair>>,
    pty_writer: Option<Box<dyn Write + Send>>,
    output_receiver: std::sync::mpsc::Receiver<String>,
    output_sender: std::sync::mpsc::Sender<String>,
    command_history: Vec<String>,
    history_index: Option<usize>,
    focus_input: bool,
}

impl TerminalPanel {
    pub fn new(initial_dir: PathBuf) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        
        let mut terminal = Self {
            current_dir: initial_dir.clone(),
            _surface: Surface::new(TERMINAL_COLS as usize, TERMINAL_ROWS as usize),
            scrollback: Vec::new(),
            input_buffer: String::new(),
            pty_pair: None,
            pty_writer: None,
            output_receiver: rx,
            output_sender: tx,
            command_history: Vec::new(),
            history_index: None,
            focus_input: true,
        };
        
        terminal.spawn_shell(initial_dir);
        terminal
    }
    
    pub fn set_directory(&mut self, path: PathBuf) {
        if self.current_dir != path {
            self.current_dir = path.clone();
            // Send cd command to terminal
            let cd_command = format!("cd \"{}\"\n", path.display());
            if let Some(writer) = &mut self.pty_writer {
                let _ = writer.write_all(cd_command.as_bytes());
                let _ = writer.flush();
            }
        }
    }
    
    fn spawn_shell(&mut self, working_dir: PathBuf) {
        let pty_system = NativePtySystem::default();
        
        let pair = match pty_system.openpty(PtySize {
            rows: TERMINAL_ROWS,
            cols: TERMINAL_COLS,
            pixel_width: 0,
            pixel_height: 0,
        }) {
            Ok(p) => p,
            Err(e) => {
                self.scrollback.push(format!("Failed to open PTY: {}", e));
                return;
            }
        };
        
        // Get the shell
        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            if cfg!(target_os = "windows") {
                "cmd.exe".to_string()
            } else {
                "/bin/sh".to_string()
            }
        });
        
        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(working_dir);
        
        // Spawn the slave
        match pair.slave.spawn_command(cmd) {
            Ok(_) => {}
            Err(e) => {
                self.scrollback.push(format!("Failed to spawn shell: {}", e));
                return;
            }
        }
        
        // Get writer for sending input
        let writer = match pair.master.take_writer() {
            Ok(w) => w,
            Err(e) => {
                self.scrollback.push(format!("Failed to get PTY writer: {}", e));
                return;
            }
        };
        
        self.pty_writer = Some(writer);
        
        // Spawn reader thread
        let mut reader = match pair.master.try_clone_reader() {
            Ok(r) => r,
            Err(e) => {
                self.scrollback.push(format!("Failed to get PTY reader: {}", e));
                return;
            }
        };
        
        let sender = self.output_sender.clone();
        thread::spawn(move || {
            let mut buf = [0u8; 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let output = String::from_utf8_lossy(&buf[..n]);
                        if sender.send(output.to_string()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        
        self.pty_pair = Some(Box::new(pair));
        
        // Display welcome message
        self.scrollback.push(format!(
            "🖥️  Terminal ready in: {}\n",
            self.current_dir.display()
        ));
    }
    
    pub fn update(&mut self, _ctx: &egui::Context) {
        // Read any available output
        while let Ok(output) = self.output_receiver.try_recv() {
            self.process_output(&output);
        }
    }
    
    fn process_output(&mut self, output: &str) {
        // Process ANSI escape sequences and add to scrollback
        // For now, we'll do simple line-based processing
        let lines: Vec<&str> = output.split('\n').collect();
        for (i, line) in lines.iter().enumerate() {
            if i == lines.len() - 1 && !output.ends_with('\n') {
                // Last line without newline - append to last scrollback entry
                if let Some(last) = self.scrollback.last_mut() {
                    last.push_str(line);
                } else {
                    self.scrollback.push(line.to_string());
                }
            } else {
                self.scrollback.push(line.to_string());
            }
        }
        
        // Limit scrollback size
        while self.scrollback.len() > 1000 {
            self.scrollback.remove(0);
        }
    }
    
    fn execute_command(&mut self) {
        let command = self.input_buffer.clone();
        if command.trim().is_empty() {
            // Just send newline
            if let Some(writer) = &mut self.pty_writer {
                let _ = writer.write_all(b"\n");
                let _ = writer.flush();
            }
            return;
        }
        
        // Add to history
        self.command_history.push(command.clone());
        if self.command_history.len() > 100 {
            self.command_history.remove(0);
        }
        self.history_index = None;
        
        // Send to PTY
        if let Some(writer) = &mut self.pty_writer {
            let cmd_with_newline = format!("{}\n", command);
            let _ = writer.write_all(cmd_with_newline.as_bytes());
            let _ = writer.flush();
        }
        
        self.input_buffer.clear();
    }
    
    fn history_prev(&mut self) {
        if self.command_history.is_empty() {
            return;
        }
        
        match self.history_index {
            None => {
                self.history_index = Some(self.command_history.len() - 1);
                self.input_buffer = self.command_history.last().unwrap().clone();
            }
            Some(idx) if idx > 0 => {
                self.history_index = Some(idx - 1);
                self.input_buffer = self.command_history[idx - 1].clone();
            }
            _ => {}
        }
    }
    
    fn history_next(&mut self) {
        match self.history_index {
            Some(idx) if idx < self.command_history.len() - 1 => {
                self.history_index = Some(idx + 1);
                self.input_buffer = self.command_history[idx + 1].clone();
            }
            Some(_) => {
                self.history_index = None;
                self.input_buffer.clear();
            }
            None => {}
        }
    }
    
    pub fn render(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            // Terminal header
            ui.horizontal(|ui| {
                ui.label(RichText::new("🖥️  Terminal").strong());
                ui.separator();
                ui.label(format!("{}", self.current_dir.display()));
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Clear").clicked() {
                        self.scrollback.clear();
                    }
                });
            });
            
            ui.separator();
            
            // Scrollback display
            let available_height = ui.available_height() - 40.0; // Reserve space for input
            
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true)
                .max_height(available_height)
                .show(ui, |ui| {
                    ui.style_mut().override_font_id = Some(egui::FontId::monospace(12.0));
                    
                    for line in &self.scrollback {
                        // Strip ANSI escape sequences for display
                        let clean_line = strip_ansi_escapes(line);
                        ui.label(RichText::new(clean_line).color(Color32::LIGHT_GRAY));
                    }
                });
            
            // Input line
            ui.horizontal(|ui| {
                ui.label(RichText::new("❯").color(Color32::GREEN).monospace());
                
                let response = ui.add(
                    TextEdit::singleline(&mut self.input_buffer)
                        .font(egui::FontId::monospace(12.0))
                        .desired_width(f32::INFINITY)
                        .hint_text("Type command...")
                );
                
                if self.focus_input {
                    response.request_focus();
                    self.focus_input = false;
                }
                
                // Handle input
                if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    self.execute_command();
                    self.focus_input = true;
                }
                
                if response.has_focus() {
                    if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
                        self.history_prev();
                    }
                    if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
                        self.history_next();
                    }
                    if ui.input(|i| i.key_pressed(Key::C) && i.modifiers.contains(Modifiers::CTRL)) {
                        // Ctrl+C - send interrupt
                        if let Some(writer) = &mut self.pty_writer {
                            let _ = writer.write_all(&[0x03]); // ETX (Ctrl+C)
                            let _ = writer.flush();
                        }
                    }
                }
            });
        });
    }
}

fn strip_ansi_escapes(s: &str) -> String {
    // Simple ANSI escape sequence stripper
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Start of escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Consume until we hit a letter
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    
    result
}

impl Drop for TerminalPanel {
    fn drop(&mut self) {
        // Clean up PTY
        self.pty_writer = None;
        self.pty_pair = None;
    }
}
