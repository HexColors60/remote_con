use eframe::egui;
use std::time::{Duration, Instant};
use crate::process::{enumerate_cmd_processes, CmdProcessInfo};
use crate::worker::{ConsoleWorker, WorkerMessage, UiMessage, WorkerConfig};
use crate::console::{attach_to_console, send_command, send_ctrl_c, send_control_char, detach_from_console};

/// Main application state
pub struct RemoteConApp {
    // Process list state
    cmd_processes: Vec<CmdProcessInfo>,
    selected_pid: Option<u32>,
    show_refresh_error: Option<String>,

    // Worker for background polling
    worker: Option<ConsoleWorker>,

    // Console output state
    console_output: Vec<String>,
    output_update_timestamp: Option<Instant>,
    lines_to_display: usize,
    refresh_interval_ms: u64,
    auto_scroll: bool,

    // Input state
    command_input: String,
    command_input_top: String,

    // Attachment state
    attached_pid: Option<u32>,
    attach_error: Option<String>,

    // Status bar
    status_message: String,
    last_error: Option<String>,

    // Context menu state
    show_context_menu: bool,
    context_menu_pid: Option<u32>,
}

impl Default for RemoteConApp {
    fn default() -> Self {
        Self {
            cmd_processes: Vec::new(),
            selected_pid: None,
            show_refresh_error: None,
            worker: None,
            console_output: Vec::new(),
            output_update_timestamp: None,
            lines_to_display: 400,
            refresh_interval_ms: 500,
            auto_scroll: true,
            command_input: String::new(),
            command_input_top: String::new(),
            attached_pid: None,
            attach_error: None,
            status_message: "Not attached".to_string(),
            last_error: None,
            show_context_menu: false,
            context_menu_pid: None,
        }
    }
}

impl RemoteConApp {
    /// Create a new application instance
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        // Initial process enumeration
        app.refresh_process_list();
        app
    }

    /// Refresh the list of cmd.exe processes
    fn refresh_process_list(&mut self) {
        match enumerate_cmd_processes() {
            Ok(processes) => {
                self.cmd_processes = processes;
                self.show_refresh_error = None;
            }
            Err(e) => {
                self.show_refresh_error = Some(format!("Failed to enumerate processes: {}", e));
            }
        }
    }

    /// Attach to the selected console
    fn attach_to_console(&mut self) {
        if let Some(pid) = self.selected_pid {
            // Detach from previous if any
            if self.attached_pid.is_some() {
                self.detach_from_console();
            }

            // Create worker for this PID
            let config = WorkerConfig {
                interval: Duration::from_millis(self.refresh_interval_ms),
                lines: self.lines_to_display,
            };

            self.worker = Some(ConsoleWorker::new(config));

            // Send attach message
            if let Some(worker) = &self.worker {
                match worker.send(UiMessage::Attach(pid)) {
                    Ok(()) => {
                        self.attached_pid = Some(pid);
                        self.attach_error = None;
                        self.status_message = format!("Attaching to PID {}...", pid);
                    }
                    Err(e) => {
                        self.attach_error = Some(format!("Failed to send attach message: {}", e));
                        self.worker = None;
                    }
                }
            }
        }
    }

    /// Detach from the current console
    fn detach_from_console(&mut self) {
        if let Some(worker) = &self.worker {
            let _ = worker.send(UiMessage::Detach);
        }
        self.worker = None;
        self.attached_pid = None;
        self.console_output.clear();
        self.status_message = "Not attached".to_string();
    }

    /// Send a command to the console
    fn send_command(&mut self) {
        if self.attached_pid.is_none() {
            self.last_error = Some("Not attached to any console".to_string());
            return;
        }

        let command = self.command_input.trim();
        if command.is_empty() {
            return;
        }

        let pid = self.attached_pid.unwrap();

        // Attach, send command, detach
        match attach_to_console(pid) {
            Ok(()) => {
                match send_command(command) {
                    Ok(()) => {
                        self.command_input.clear();
                        self.last_error = None;
                    }
                    Err(e) => {
                        self.last_error = Some(format!("Failed to send command: {}", e));
                    }
                }
                let _ = detach_from_console();
            }
            Err(e) => {
                self.last_error = Some(format!("Failed to attach for command: {}", e));
            }
        }
    }

    /// Send a command from the top input field to the console
    fn send_command_from_top(&mut self) {
        if self.attached_pid.is_none() {
            self.last_error = Some("Not attached to any console".to_string());
            return;
        }

        let command = self.command_input_top.trim();
        if command.is_empty() {
            return;
        }

        let pid = self.attached_pid.unwrap();

        // Attach, send command, detach
        match attach_to_console(pid) {
            Ok(()) => {
                match send_command(command) {
                    Ok(()) => {
                        self.command_input_top.clear();
                        self.last_error = None;
                    }
                    Err(e) => {
                        self.last_error = Some(format!("Failed to send command: {}", e));
                    }
                }
                let _ = detach_from_console();
            }
            Err(e) => {
                self.last_error = Some(format!("Failed to attach for command: {}", e));
            }
        }
    }

    /// Send Ctrl+C to the console
    fn send_ctrl_c(&mut self) {
        if self.attached_pid.is_none() {
            self.last_error = Some("Not attached to any console".to_string());
            return;
        }

        let pid = self.attached_pid.unwrap();

        match attach_to_console(pid) {
            Ok(()) => {
                match send_ctrl_c() {
                    Ok(()) => {
                        self.last_error = None;
                    }
                    Err(e) => {
                        self.last_error = Some(format!("Failed to send Ctrl+C: {}", e));
                    }
                }
                let _ = detach_from_console();
            }
            Err(e) => {
                self.last_error = Some(format!("Failed to attach for Ctrl+C: {}", e));
            }
        }
    }

    /// Send Ctrl+J (Line Feed - \n, 0x0A) to the console
    fn send_ctrl_j(&mut self) {
        if self.attached_pid.is_none() {
            self.last_error = Some("Not attached to any console".to_string());
            return;
        }

        let pid = self.attached_pid.unwrap();

        match attach_to_console(pid) {
            Ok(()) => {
                // Send Ctrl+J (Line Feed - 0x0A)
                match send_control_char(0x0A) {
                    Ok(()) => {
                        self.last_error = None;
                    }
                    Err(e) => {
                        self.last_error = Some(format!("Failed to send Ctrl+J: {}", e));
                    }
                }
                let _ = detach_from_console();
            }
            Err(e) => {
                self.last_error = Some(format!("Failed to attach for Ctrl+J: {}", e));
            }
        }
    }

    /// Send Ctrl+M (Carriage Return - \r, 0x0D) to the console
    fn send_ctrl_m(&mut self) {
        if self.attached_pid.is_none() {
            self.last_error = Some("Not attached to any console".to_string());
            return;
        }

        let pid = self.attached_pid.unwrap();

        match attach_to_console(pid) {
            Ok(()) => {
                // Send Ctrl+M (Carriage Return - 0x0D)
                match send_control_char(0x0D) {
                    Ok(()) => {
                        self.last_error = None;
                    }
                    Err(e) => {
                        self.last_error = Some(format!("Failed to send Ctrl+M: {}", e));
                    }
                }
                let _ = detach_from_console();
            }
            Err(e) => {
                self.last_error = Some(format!("Failed to attach for Ctrl+M: {}", e));
            }
        }
    }

    /// Send \n\r (Line Feed + Carriage Return) to the console
    fn send_newline_carriage_return(&mut self) {
        if self.attached_pid.is_none() {
            self.last_error = Some("Not attached to any console".to_string());
            return;
        }

        let pid = self.attached_pid.unwrap();

        match attach_to_console(pid) {
            Ok(()) => {
                // Send Line Feed (0x0A) followed by Carriage Return (0x0D)
                match send_control_char(0x0A) {
                    Ok(()) => {
                        match send_control_char(0x0D) {
                            Ok(()) => {
                                self.last_error = None;
                            }
                            Err(e) => {
                                self.last_error = Some(format!("Failed to send \\r: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        self.last_error = Some(format!("Failed to send \\n: {}", e));
                    }
                }
                let _ = detach_from_console();
            }
            Err(e) => {
                self.last_error = Some(format!("Failed to attach for \\n\\r: {}", e));
            }
        }
    }

    /// Update the console output from worker messages
    fn update_from_worker(&mut self) {
        // Take the worker out temporarily to avoid borrow conflicts
        let mut disconnected = false;
        if self.worker.is_some() {
            // Process all available messages
            loop {
                let msg = {
                    // Borrow worker only for the try_recv call
                    if let Some(ref worker) = self.worker {
                        worker.try_recv()
                    } else {
                        break;
                    }
                };

                match msg {
                    Some(WorkerMessage::Output { lines, timestamp }) => {
                        self.console_output = lines;
                        self.output_update_timestamp = Some(timestamp);
                        self.attach_error = None;
                        self.last_error = None;
                        if let Some(pid) = self.attached_pid {
                            self.status_message = format!("Attached to PID {} - Last update: {:?}", pid, timestamp);
                        }
                    }
                    Some(WorkerMessage::Error(e)) => {
                        self.last_error = Some(e);
                    }
                    Some(WorkerMessage::Status(s)) => {
                        self.status_message = s;
                    }
                    Some(WorkerMessage::Disconnected) => {
                        disconnected = true;
                        self.attached_pid = None;
                        self.status_message = "Disconnected".to_string();
                        self.last_error = Some("Console disconnected".to_string());
                    }
                    None => break,
                }
            }

            if disconnected {
                self.worker = None;
            }
        }
    }

    /// Render the left panel (process list)
    fn render_process_list(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("process_list").show(ctx, |ui| {
            ui.heading("CMD Processes");

            // Refresh button
            if ui.button("Refresh").clicked() {
                self.refresh_process_list();
            }

            // Show error if any
            if let Some(ref err) = self.show_refresh_error {
                ui.colored_label(egui::Color32::RED, err);
            }

            ui.separator();

            // Process list
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.cmd_processes.is_empty() {
                    ui.label("No cmd.exe processes found.");
                    ui.label("Make sure cmd.exe is running in the same session.");
                } else {
                    let mut attach_on_double_click = None;

                    for proc in &self.cmd_processes {
                        let is_selected = self.selected_pid == Some(proc.pid);

                        // Process row
                        let response = ui.group(|ui| {
                            ui.horizontal(|ui| {
                                // Radio button for selection
                                if ui.selectable_label(is_selected, format!("PID: {}", proc.pid)).clicked() {
                                    self.selected_pid = Some(proc.pid);
                                }

                                ui.vertical(|ui| {
                                    // Window title
                                    if let Some(ref title) = proc.window_title {
                                        ui.label(format!("Title: {}", title));
                                    } else {
                                        ui.label("Title: (no window)");
                                    }

                                    // Session and window info
                                    ui.label(format!("Session: {} | Window: {}",
                                        proc.session_id,
                                        if proc.has_window { "Yes" } else { "No" }
                                    ));

                                    // Status
                                    let status = if proc.attachable {
                                        egui::Color32::DARK_GREEN
                                    } else {
                                        egui::Color32::GRAY
                                    };
                                    ui.colored_label(status,
                                        if proc.attachable { "Attachable" } else { "Not attachable" }
                                    );
                                });
                            });
                        }).response;

                        // Double-click to attach
                        if response.double_clicked() && proc.attachable {
                            self.selected_pid = Some(proc.pid);
                            attach_on_double_click = Some(proc.pid);
                        }

                        // Right-click context menu
                        if response.secondary_clicked() && proc.attachable {
                            self.selected_pid = Some(proc.pid);
                            self.context_menu_pid = Some(proc.pid);
                            self.show_context_menu = true;
                        }
                    }

                    // Handle double-click attach outside the loop to avoid borrow conflict
                    if let Some(pid) = attach_on_double_click {
                        if self.selected_pid == Some(pid) {
                            self.attach_to_console();
                        }
                    }
                }
            });

            ui.separator();

            // Selected process info bar
            if let Some(pid) = self.selected_pid {
                if let Some(proc) = self.cmd_processes.iter().find(|p| p.pid == pid) {
                    ui.horizontal(|ui| {
                        ui.label("Selected PID:");
                        ui.label(egui::RichText::new(format!("{}", pid)).size(16.0).color(egui::Color32::LIGHT_BLUE));
                        ui.separator();
                        ui.label(format!("Session: {}", proc.session_id));
                        ui.label(format!("Window: {}", if proc.has_window { "Yes" } else { "No" }));
                    });
                }
            } else {
                ui.label(egui::RichText::new("No process selected").italics().weak());
            }

            ui.separator();

            // Attach button section
            let can_attach = self.selected_pid.is_some() &&
                self.cmd_processes.iter()
                    .any(|p| p.pid == self.selected_pid.unwrap() && p.attachable);

            // Attach button
            ui.add_enabled_ui(can_attach, |ui| {
                if ui.button("Attach").clicked() {
                    self.attach_to_console();
                }
            });

            ui.separator();

            // Detach button (always visible)
            ui.add_enabled_ui(self.attached_pid.is_some(), |ui| {
                if ui.button("Detach").clicked() {
                    self.detach_from_console();
                }
            });
        });
    }

    /// Show the context menu for attaching to a process
    fn show_context_menu_ui(&mut self, ctx: &egui::Context) {
        if !self.show_context_menu {
            return;
        }

        let mouse_pos = ctx.input(|i| i.pointer.hover_pos().unwrap_or_default());

        egui::Area::new(egui::Id::new("popup_context_menu"))
            .fixed_pos(mouse_pos)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.style_mut().visuals.panel_fill = egui::Color32::from_gray(240);
                ui.style_mut().visuals.window_shadow = egui::epaint::Shadow {
                    offset: [4, 4],
                    blur: 8,
                    spread: 0,
                    color: egui::Color32::BLACK.linear_multiply(0.2),
                };

                egui::Frame::popup(ui.style())
                    .show(ui, |ui| {
                        ui.set_min_width(150.0);
                        ui.vertical(|ui| {
                            ui.label(format!("Attach to PID: {}",
                                self.context_menu_pid.unwrap_or(0)));
                            ui.separator();
                            if ui.button("Attach").clicked() {
                                if let Some(pid) = self.context_menu_pid {
                                    self.selected_pid = Some(pid);
                                    self.attach_to_console();
                                }
                                self.show_context_menu = false;
                            }
                            ui.separator();
                            if ui.button("Cancel").clicked() {
                                self.show_context_menu = false;
                            }
                        });
                    });

                // Close menu when clicking outside
                if ui.input(|i| i.pointer.any_released()) {
                    // Check if click was outside the menu
                    let menu_rect = ui.min_rect();
                    if let Some(click_pos) = ui.input(|i| i.pointer.press_origin()) {
                        if !menu_rect.contains(click_pos) {
                            self.show_context_menu = false;
                        }
                    }
                }
            });
    }

    /// Render the right panel (console viewer)
    fn render_console_viewer(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Console Output");

            // Attach/Detach buttons at top
            let can_attach = self.selected_pid.is_some() &&
                self.cmd_processes.iter()
                    .any(|p| p.pid == self.selected_pid.unwrap() && p.attachable);

            ui.horizontal(|ui| {
                // Attach button
                ui.add_enabled_ui(can_attach, |ui| {
                    if ui.button("Attach").clicked() {
                        self.attach_to_console();
                    }
                });

                ui.separator();

                // Detach button
                ui.add_enabled_ui(self.attached_pid.is_some(), |ui| {
                    if ui.button("Detach").clicked() {
                        self.detach_from_console();
                    }
                });
            });

            ui.separator();

            // Quick command input at top
            ui.horizontal(|ui| {
                ui.label("Quick Command:");
                let response = ui.add_sized(
                    [ui.available_width() - 80.0, 20.0],
                    egui::TextEdit::singleline(&mut self.command_input_top)
                        .hint_text("Type quick command here...")
                        .desired_width(f32::INFINITY)
                );

                // Send on Enter
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.send_command_from_top();
                }

                // Send button
                ui.add_enabled_ui(self.attached_pid.is_some() && !self.command_input_top.trim().is_empty(), |ui| {
                    if ui.button("Send").clicked() {
                        self.send_command_from_top();
                    }
                });
            });

            // Control character buttons
            ui.horizontal(|ui| {
                ui.label("Send:");

                // Ctrl-J button (Line Feed - \n, 0x0A)
                ui.add_enabled_ui(self.attached_pid.is_some(), |ui| {
                    if ui.button("Ctrl-J").clicked() {
                        self.send_ctrl_j();
                    }
                });

                // Ctrl-M button (Carriage Return - \r, 0x0D)
                ui.add_enabled_ui(self.attached_pid.is_some(), |ui| {
                    if ui.button("Ctrl-M").clicked() {
                        self.send_ctrl_m();
                    }
                });

                ui.separator();

                // \n\r button (Line Feed + Carriage Return)
                ui.add_enabled_ui(self.attached_pid.is_some(), |ui| {
                    if ui.button("\\n\\r").clicked() {
                        self.send_newline_carriage_return();
                    }
                });
            });

            ui.separator();

            // Status bar
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
                ui.separator();

                // Lines to display slider
                ui.label("Lines:");
                ui.add(egui::Slider::new(&mut self.lines_to_display, 10..=500));

                // Refresh interval slider
                ui.label("Interval (ms):");
                let mut interval = self.refresh_interval_ms as i32;
                if ui.add(egui::Slider::new(&mut interval, 50..=2000)).changed() {
                    self.refresh_interval_ms = interval as u64;
                    // Update worker interval
                    if let Some(worker) = &self.worker {
                        let _ = worker.send(UiMessage::SetInterval(Duration::from_millis(self.refresh_interval_ms)));
                    }
                }
            });

            // Auto-scroll checkbox
            ui.checkbox(&mut self.auto_scroll, "Auto-scroll to bottom");

            ui.separator();

            // Console output area
            egui::ScrollArea::vertical()
                .show(ui, |ui| {
                    if self.console_output.is_empty() {
                        if self.attached_pid.is_some() {
                            ui.label("Waiting for console output...");
                        } else {
                            ui.label("Not attached to any console.");
                            ui.label("Select a cmd.exe process and click Attach.");
                        }
                    } else {
                        egui::Grid::new("console_output").show(ui, |ui| {
                            for line in &self.console_output {
                                ui.label(line);
                                ui.end_row();
                            }
                        });
                    }

                    // Scroll to bottom if auto-scroll is enabled
                    if self.auto_scroll && !self.console_output.is_empty() {
                        ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                    }
                });

            ui.separator();

            // Show error if any
            if let Some(ref err) = self.last_error {
                ui.colored_label(egui::Color32::RED, err);
            }

            ui.separator();

            // Input area
            ui.horizontal(|ui| {
                ui.label("Command:");
                let response = ui.add_sized(
                    [ui.available_width() - 150.0, 20.0],
                    egui::TextEdit::singleline(&mut self.command_input)
                        .hint_text("Type command here...")
                        .desired_width(f32::INFINITY)
                );

                // Send on Enter
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.send_command();
                }

                // Send button
                ui.add_enabled_ui(self.attached_pid.is_some() && !self.command_input.trim().is_empty(), |ui| {
                    if ui.button("Send").clicked() {
                        self.send_command();
                    }
                });

                // Ctrl+C button
                ui.add_enabled_ui(self.attached_pid.is_some(), |ui| {
                    if ui.button("Ctrl+C").clicked() {
                        self.send_ctrl_c();
                    }
                });
            });
        });
    }
}

impl eframe::App for RemoteConApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update from worker messages
        self.update_from_worker();

        // Render UI
        self.render_process_list(ctx);
        self.render_console_viewer(ctx);

        // Show context menu if active
        self.show_context_menu_ui(ctx);

        // Request continuous repaint
        ctx.request_repaint();
    }
}
