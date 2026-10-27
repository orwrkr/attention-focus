#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use attention_focus::{
    icons, lifecycle, platform, report_ui, source, tracking, tray_interaction::Clicks,
};
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
    PlatformError(String),
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
    activity: tracking::Activity,
    lifecycle: Option<lifecycle::Listener>,
    reports: report_ui::Panel,
    project_search: String,
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
        let lifecycle_commands = commands.clone();
        let lifecycle_sender = sender.clone();
        let ctx = context.egui_ctx.clone();
        let listener = lifecycle::Listener::start(move |reason| {
            let (done, saved) = mpsc::sync_channel(1);
            let command = tracking::Command::Interrupt {
                reason,
                boundary: Some(tracking::Boundary::now()),
                done: Some(done),
            };
            if lifecycle_commands.try_send(command).is_err()
                || saved.recv_timeout(Duration::from_millis(1500)).is_err()
            {
                let _ = lifecycle_sender.send(Action::PlatformError(
                    "Cannot confirm saving before lock or suspend. Check session recovery.".into(),
                ));
                ctx.request_repaint();
            }
        });
        let listener = match listener {
            Ok(listener) => Some(listener),
            Err(error) => {
                let _ = sender.send(Action::PlatformError(format!(
                    "Windows lifecycle monitoring is unavailable: {error}"
                )));
                None
            }
        };
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
            activity: tracking::Activity::default(),
            lifecycle: listener,
            reports: report_ui::Panel::default(),
            project_search: String::new(),
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

    fn activity_picker(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::TextEdit::singleline(&mut self.project_search).hint_text("Search projects"));
        let query = self.project_search.trim().to_lowercase();
        egui::ComboBox::from_id_salt("project")
            .selected_text(self.activity.label())
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(self.activity.project.is_none(), "Unassigned focus")
                    .clicked()
                {
                    self.activity.project = None;
                }
                if let Some(source) = &self.source_selection {
                    for project in self.catalog.projects.iter().filter(|p| {
                        !p.archived
                            && p.status != "Completed"
                            && (query.is_empty()
                                || p.title.to_lowercase().contains(&query)
                                || p.slug.to_lowercase().contains(&query))
                    }) {
                        let selected = self.activity.project.as_ref().is_some_and(|p| {
                            p.source_id == source.id
                                && p.source_path == source.path.to_string_lossy()
                                && p.slug == project.slug
                        });
                        if ui.selectable_label(selected, &project.title).clicked() {
                            self.activity.project = Some(tracking::Project {
                                source_id: source.id,
                                source_path: source.path.to_string_lossy().into_owned(),
                                slug: project.slug.clone(),
                                title: project.title.clone(),
                                task: None,
                            });
                        }
                    }
                }
            });
        if let Some(project) = &mut self.activity.project {
            egui::ComboBox::from_id_salt("task")
                .selected_text(project.task.as_deref().unwrap_or("No task"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut project.task, None, "No task");
                    if let Some(source) = &self.source_selection
                        && source.id == project.source_id
                        && source.path.to_string_lossy() == project.source_path
                        && let Some(snapshot) = self
                            .catalog
                            .projects
                            .iter()
                            .find(|p| p.slug == project.slug)
                    {
                        for task in &snapshot.tasks {
                            ui.selectable_value(&mut project.task, Some(task.clone()), task);
                        }
                    }
                });
        }
        ui.add(
            egui::TextEdit::singleline(&mut self.activity.note).hint_text("Optional session note"),
        );
        if self.activity != self.state.activity {
            ui.label("Selection has not been applied.");
            if ui
                .add_enabled(
                    self.ready
                        && !self.pending
                        && !self.state.needs_review
                        && self.available(&self.activity),
                    egui::Button::new(if self.state.running {
                        "Switch focus"
                    } else {
                        "Apply selection"
                    }),
                )
                .clicked()
            {
                self.command(if self.state.running {
                    tracking::Command::Switch(self.activity.clone())
                } else {
                    tracking::Command::Select(self.activity.clone())
                });
            }
        }
        if !self.available(&self.activity) {
            ui.label(
                "Selected activity is unavailable. Refresh MetaPlan or choose another activity.",
            );
        }
    }

    fn available(&self, activity: &tracking::Activity) -> bool {
        let Some(project) = &activity.project else {
            return true;
        };
        self.source_selection.as_ref().is_some_and(|source| {
            source.id == project.source_id && source.path.to_string_lossy() == project.source_path
        }) && self.catalog.projects.iter().any(|snapshot| {
            snapshot.slug == project.slug
                && !snapshot.archived
                && snapshot.status != "Completed"
                && project
                    .task
                    .as_ref()
                    .is_none_or(|task| snapshot.tasks.contains(task))
        })
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
                Action::PlatformError(error) => {
                    self.error = Some(error);
                    Self::show(ctx);
                }
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
                    if self.ready
                        && !self.state.needs_review
                        && (self.state.running
                            || (self.lifecycle.is_some() && self.available(&self.state.activity)))
                    {
                        self.command(tracking::Command::Toggle);
                    } else {
                        Self::show(ctx);
                    }
                }
                Action::Toggle => {
                    if self.ready
                        && !self.state.needs_review
                        && (self.state.running
                            || (self.lifecycle.is_some() && self.available(&self.state.activity)))
                    {
                        self.command(tracking::Command::Toggle);
                    } else {
                        Self::show(ctx);
                    }
                }
                Action::Exit => self.command(tracking::Command::Exit),
                Action::Tracking(tracking::Event::Report { id, result }) => {
                    self.reports.receive(id, result)
                }
                Action::Tracking(
                    event @ (tracking::Event::State(_) | tracking::Event::Committed(_)),
                ) => {
                    let completed = matches!(event, tracking::Event::Committed(_));
                    let state = match event {
                        tracking::Event::State(state) | tracking::Event::Committed(state) => state,
                        _ => unreachable!(),
                    };
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
                    self.toggle_item.set_enabled(
                        !state.needs_review && (state.running || self.lifecycle.is_some()),
                    );
                    if !self.ready {
                        self.activity = state.activity.clone();
                    }
                    self.state = state;
                    self.ready = true;
                    if completed {
                        self.pending = false;
                    }
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
        self.toggle_item.set_enabled(
            self.ready
                && !self.pending
                && !self.state.needs_review
                && (self.state.running
                    || (self.lifecycle.is_some() && self.available(&self.state.activity))),
        );
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
                    ui.label(self.state.activity.label());
                    if let Some(task) = self
                        .state
                        .activity
                        .project
                        .as_ref()
                        .and_then(|p| p.task.as_ref())
                    {
                        ui.label(task);
                    }
                    self.activity_picker(ui);
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
                            self.ready
                                && !self.pending
                                && !self.state.needs_review
                                && (self.state.running
                                    || (self.lifecycle.is_some()
                                        && self.available(&self.activity))),
                            egui::Button::new(if self.state.running {
                                "Stop tracking"
                            } else {
                                "Start tracking"
                            }),
                        )
                        .clicked()
                    {
                        self.command(if self.state.running {
                            tracking::Command::Stop
                        } else {
                            tracking::Command::Start(self.activity.clone())
                        });
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
                        ui.label("Projects are read only. Use the project picker to choose focus.");
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
                    self.reports.show(ui, &self.commands, self.ready);
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
