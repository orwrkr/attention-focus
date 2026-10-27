#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use attention_focus::{icons, platform, source, tracking, tray_interaction::Clicks};
use eframe::egui;
use std::{
    sync::mpsc::{self, Receiver, SyncSender},
    time::{Duration, Instant},
};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem},
};

enum Action {
    Show,
    LeftRelease(Instant),
    DoubleClick,
    Toggle,
    Exit,
    Tracking(tracking::Event),
    Source(Result<(source::Selection, source::Catalog), String>),
}

struct Desktop {
    tray: Option<TrayIcon>,
    toggle_item: MenuItem,
    actions: Receiver<Action>,
    commands: SyncSender<tracking::Command>,
    state: tracking::Snapshot,
    ready: bool,
    pending: bool,
    error: Option<String>,
    exiting: bool,
    last_geometry: Option<egui::Rect>,
    clicks: Clicks,
    directory: std::path::PathBuf,
    sender: mpsc::Sender<Action>,
    source_path: String,
    source_selection: Option<source::Selection>,
    catalog: source::Catalog,
    source_pending: bool,
    source_error: Option<String>,
}

fn tray_icon(running: bool) -> Icon {
    Icon::from_rgba(
        icons::rgba(
            32,
            if running {
                icons::State::Running
            } else {
                icons::State::Stopped
            },
        ),
        32,
        32,
    )
    .expect("Generated icon dimensions are valid")
}

fn duration(ms: u64) -> String {
    let seconds = ms / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

impl Desktop {
    fn new(
        context: &eframe::CreationContext<'_>,
        directory: std::path::PathBuf,
        selection: Option<source::Selection>,
    ) -> Self {
        let (sender, actions) = mpsc::channel();
        let menu = Menu::new();
        let show = MenuItem::new("Show Focus", true, None);
        let toggle = MenuItem::new("Start tracking", false, None);
        let exit = MenuItem::new("Exit", true, None);
        let tray_result = (|| -> Result<TrayIcon, Box<dyn std::error::Error>> {
            menu.append_items(&[&show, &toggle, &exit])?;
            Ok(TrayIconBuilder::new()
                .with_tooltip("Attention Focus - opening history...")
                .with_icon(tray_icon(false))
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .build()?)
        })();
        let (tray, error) = match tray_result {
            Ok(tray) => (Some(tray), None),
            Err(error) => (None, Some(format!("Tray unavailable: {error}"))),
        };
        let menu_sender = sender.clone();
        let ctx = context.egui_ctx.clone();
        let show_id = show.id().clone();
        let toggle_id = toggle.id().clone();
        let exit_id = exit.id().clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let action = if event.id == show_id {
                Some(Action::Show)
            } else if event.id == toggle_id {
                Some(Action::Toggle)
            } else if event.id == exit_id {
                Some(Action::Exit)
            } else {
                None
            };
            if let Some(action) = action {
                let _ = menu_sender.send(action);
                ctx.request_repaint();
            }
        }));
        let tray_sender = sender.clone();
        let ctx = context.egui_ctx.clone();
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            let action = match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => Some(Action::LeftRelease(Instant::now())),
                TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                } => Some(Action::DoubleClick),
                _ => None,
            };
            if let Some(action) = action {
                let _ = tray_sender.send(action);
                ctx.request_repaint();
            }
        }));
        let ctx = context.egui_ctx.clone();
        let tracking_sender = sender.clone();
        let commands = tracking::spawn(directory.join("focus.sqlite3"), move |event| {
            let _ = tracking_sender.send(Action::Tracking(event));
            ctx.request_repaint();
        });
        let mut desktop = Self {
            tray,
            toggle_item: toggle,
            actions,
            commands,
            state: tracking::Snapshot::default(),
            ready: false,
            pending: false,
            error,
            exiting: false,
            last_geometry: None,
            clicks: Clicks::default(),
            directory,
            sender,
            source_path: selection
                .as_ref()
                .map(|s| s.path.display().to_string())
                .unwrap_or_default(),
            source_selection: selection,
            catalog: source::Catalog::default(),
            source_pending: false,
            source_error: None,
        };
        if desktop.source_selection.is_some() {
            desktop.load_source(&context.egui_ctx, false);
        }
        desktop
    }

    fn load_source(&mut self, ctx: &egui::Context, save: bool) {
        self.source_pending = true;
        self.source_error = None;
        let path = std::path::PathBuf::from(self.source_path.trim());
        let directory = self.directory.clone();
        let selection = self.source_selection.clone();
        let sender = self.sender.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
                let path = source::validate_boundary(&path, &directory)?;
                let catalog = source::read(&path)?;
                let selection = if save {
                    source::save(&directory, &path)?
                } else {
                    selection.ok_or("No source selected.")?
                };
                Ok((selection, catalog))
            })()
            .map_err(|e| format!("Cannot load MetaPlan: {e}"));
            let _ = sender.send(Action::Source(result));
            ctx.request_repaint();
        });
    }

    fn command(&mut self, command: tracking::Command) {
        if self.pending {
            return;
        }
        match self.commands.try_send(command) {
            Ok(()) => {
                self.pending = true;
                self.error = None;
            }
            Err(error) => self.error = Some(format!("Cannot send tracking command: {error}")),
        }
    }

    fn show(ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    fn hide(ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        // Ensure a final geometry autosave after the window becomes idle/hidden.
        ctx.request_repaint_after(Duration::from_secs(3));
    }
}

impl eframe::App for Desktop {
    fn auto_save_interval(&self) -> Duration {
        Duration::from_secs(2)
    }
    fn persist_egui_memory(&self) -> bool {
        false
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(action) = self.actions.try_recv() {
            match action {
                Action::Source(result) => {
                    self.source_pending = false;
                    match result {
                        Ok((selection, catalog)) => {
                            self.source_path = selection.path.display().to_string();
                            self.source_selection = Some(selection);
                            self.catalog = catalog;
                        }
                        Err(error) => {
                            self.source_error = Some(error);
                            self.catalog = source::Catalog::default();
                        }
                    }
                }
                Action::Show => Self::show(ctx),
                Action::LeftRelease(now) => {
                    let interval = unsafe {
                        windows_sys::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime()
                    };
                    self.clicks
                        .release(now, Duration::from_millis(u64::from(interval)));
                }
                Action::DoubleClick => {
                    self.clicks.double_click();
                    if self.ready && !self.state.needs_review {
                        self.command(tracking::Command::Toggle);
                    } else {
                        Self::show(ctx);
                    }
                }
                Action::Toggle => {
                    if self.ready && !self.state.needs_review {
                        self.command(tracking::Command::Toggle);
                    } else {
                        Self::show(ctx);
                    }
                }
                Action::Exit => self.command(tracking::Command::Exit),
                Action::Tracking(tracking::Event::State(state)) => {
                    if (self.state.running != state.running || !self.ready)
                        && let Some(tray) = &self.tray
                        && let Err(error) = tray.set_icon(Some(tray_icon(state.running)))
                    {
                        self.error = Some(format!("Cannot update tray icon: {error}"));
                    }
                    // Update each snapshot, including while hidden: total includes the
                    // active session once and excludes unreviewed recovery time.
                    if let Some(tray) = &self.tray {
                        let label = format!(
                            "Attention Focus - {}\nSession: {}\nTotal: {}{}",
                            if state.running { "tracking" } else { "stopped" },
                            duration(state.elapsed_ms),
                            duration(state.total_ms.saturating_add(state.elapsed_ms)),
                            if state.needs_review {
                                "\nInterrupted time needs review"
                            } else {
                                ""
                            }
                        );
                        if let Err(error) = tray.set_tooltip(Some(label)) {
                            self.error = Some(format!("Cannot update tray status: {error}"));
                        }
                    }
                    self.toggle_item.set_text(if state.running {
                        "Stop tracking"
                    } else {
                        "Start tracking"
                    });
                    self.toggle_item.set_enabled(!state.needs_review);
                    self.state = state;
                    self.ready = true;
                    self.pending = false;
                }
                Action::Tracking(tracking::Event::Error(error)) => {
                    self.pending = false;
                    self.error = Some(error);
                    Self::show(ctx);
                }
                Action::Tracking(tracking::Event::ExitReady) => {
                    self.exiting = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        let (show, delay) = self.clicks.poll(Instant::now());
        if show {
            Self::show(ctx);
        }
        if let Some(delay) = delay {
            ctx.request_repaint_after(delay);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let geometry = ctx.input(|input| input.viewport().outer_rect);
        if geometry != self.last_geometry {
            self.last_geometry = geometry;
            ctx.request_repaint_after(Duration::from_secs(3));
        }
        // Never reuse the stale close input available to hidden-window logic.
        if ctx.input(|input| input.viewport().close_requested()) && !self.exiting {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if self.tray.is_some() {
                Self::hide(&ctx);
            } else {
                self.command(tracking::Command::Exit);
            }
        }
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("window-content")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.heading("Attention Focus");
                    ui.label("Unassigned focus");
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new(duration(self.state.elapsed_ms))
                            .size(36.0)
                            .monospace(),
                    );
                    ui.label(if !self.ready {
                        "Opening history..."
                    } else if self.state.running {
                        "Tracking"
                    } else {
                        "Stopped"
                    });
                    ui.add_space(12.0);
                    if ui
                        .add_enabled(
                            self.ready && !self.pending && !self.state.needs_review,
                            egui::Button::new(if self.state.running {
                                "Stop tracking"
                            } else {
                                "Start tracking"
                            }),
                        )
                        .clicked()
                    {
                        self.command(tracking::Command::Toggle);
                    }
                    ui.separator();
                    ui.label(format!(
                        "Saved focus: {} in {} sessions",
                        duration(self.state.total_ms),
                        self.state.completed
                    ));
                    if self.state.needs_review {
                        ui.label("An interrupted session was saved up to its last checkpoint.");
                        ui.label(
                            "Time after that checkpoint is excluded. Tracking has not resumed.",
                        );
                        if ui
                            .add_enabled(!self.pending, egui::Button::new("Accept saved time"))
                            .clicked()
                        {
                            self.command(tracking::Command::AcceptRecovery);
                        }
                    }
                    if let Some(error) = &self.error {
                        ui.colored_label(egui::Color32::LIGHT_RED, error);
                    }
                    ui.add_space(12.0);
                    ui.label("Double-click the tray icon to start or stop.");
                    ui.label("Click the tray icon to show this window. Hover to see tracked time.");
                    ui.collapsing("MetaPlan source and projects", |ui| {
                        ui.label("Instance folder (read-only)");
                        ui.add_enabled(
                            !self.source_pending,
                            egui::TextEdit::singleline(&mut self.source_path)
                                .desired_width(f32::INFINITY),
                        );
                        if ui
                            .add_enabled(
                                !self.source_pending,
                                egui::Button::new("Save and refresh"),
                            )
                            .clicked()
                        {
                            self.load_source(&ctx, true);
                        }
                        if self.source_pending {
                            ui.label("Loading projects...");
                        }
                        if let Some(error) = &self.source_error {
                            ui.colored_label(egui::Color32::LIGHT_RED, error);
                        }
                        ui.label("Preview only. Tracking is still unassigned.");
                        egui::ScrollArea::vertical()
                            .id_salt("source-preview")
                            .max_height(230.0)
                            .show(ui, |ui| {
                                for project in &self.catalog.projects {
                                    ui.push_id((&project.slug, project.archived), |ui| {
                                        ui.collapsing(
                                            format!(
                                                "{}{}",
                                                project.title,
                                                if project.archived { " (Archived)" } else { "" }
                                            ),
                                            |ui| {
                                                ui.label(format!(
                                                    "{} | {} | {}",
                                                    project.slug, project.status, project.priority
                                                ));
                                                if project.tasks.is_empty() {
                                                    ui.label("No unfinished current-cycle tasks.");
                                                }
                                                for task in &project.tasks {
                                                    ui.label(task);
                                                }
                                            },
                                        );
                                    });
                                }
                                for diagnostic in &self.catalog.diagnostics {
                                    ui.colored_label(egui::Color32::YELLOW, diagnostic);
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(self.tray.is_some(), egui::Button::new("Hide to tray"))
                            .clicked()
                        {
                            Self::hide(&ctx);
                        }
                        if ui
                            .add_enabled(!self.pending, egui::Button::new("Exit"))
                            .clicked()
                        {
                            self.command(tracking::Command::Exit);
                        }
                    });
                });
        });
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let _instance = platform::Instance::acquire()?;
    let directory = platform::data_dir()?;
    // Restore read-only and validate source overlap before SQLite or eframe can write.
    let selection =
        source::restore(&directory).map_err(|error| -> Box<dyn std::error::Error> { error })?;
    let directory = source::resolve(&directory)?;
    std::fs::create_dir_all(&directory)?;
    let window_state = directory.join("window.ron");
    eframe::run_native(
        "Attention Focus",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_app_id("AttentionFocus")
                .with_inner_size([620.0, 640.0])
                .with_min_inner_size([420.0, 360.0])
                .with_icon(egui::IconData {
                    rgba: icons::rgba(64, icons::State::App),
                    width: 64,
                    height: 64,
                }),
            persist_window: true,
            persistence_path: Some(window_state),
            ..Default::default()
        },
        Box::new(move |context| Ok(Box::new(Desktop::new(context, directory, selection)))),
    )?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        let message: Vec<u16> = error.to_string().encode_utf16().chain(Some(0)).collect();
        let title: Vec<u16> = "Attention Focus".encode_utf16().chain(Some(0)).collect();
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                std::ptr::null_mut(),
                message.as_ptr(),
                title.as_ptr(),
                windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                    | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
            );
        }
    }
}
