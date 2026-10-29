#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use attention_focus::{
    export, icons, lifecycle, platform, presentation,
    profiles::{Paths, Profile},
    report_ui, source, tracking,
    tray_interaction::Clicks,
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
    Folder(Result<Option<std::path::PathBuf>, String>),
    Export(Result<String, String>),
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
    folder_pending: bool,
    source_error: Option<String>,
    activity: tracking::Activity,
    lifecycle: Option<lifecycle::Listener>,
    reports: report_ui::Panel,
    project_search: String,
    restored_note: Option<String>,
    profile: Profile,
    storage_root: std::path::PathBuf,
    stay_on_top: bool,
    motion_epoch: Instant,
    tray_level: Option<(bool, u8)>,
    running_icons: Vec<Icon>,
    drag_regions: platform::HeaderDragRegions,
    startup: attention_focus::startup_ui::Panel,
    settings_open: bool,
}

fn tray_icon(running: bool, profile: Profile) -> Icon {
    tray_icon_level(running, profile, 4)
}

fn tray_icon_level(running: bool, profile: Profile, level: u8) -> Icon {
    Icon::from_rgba(
        icons::tracking_rgba(
            32,
            match (running, profile) {
                (true, Profile::Production) => icons::State::Running,
                (false, Profile::Production) => icons::State::Stopped,
                (true, Profile::Development) => icons::State::RunningDevelopment,
                (false, Profile::Development) => icons::State::StoppedDevelopment,
            },
            level,
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
        paths: Paths,
        selection: Option<source::Selection>,
        source_warning: Option<String>,
        launch: attention_focus::startup::Launch,
    ) -> Self {
        let directory = paths.directory;
        let profile = paths.profile;
        let drag_regions = platform::HeaderDragRegions::default();
        let width_error = platform::constrain_window_bounds(
            context,
            context
                .winit_window()
                .map_or(1.0, |window| window.scale_factor()),
            context
                .storage
                .and_then(|storage| storage.get_string("window"))
                .is_none(),
            &drag_regions,
        )
        .err()
        .map(|error| format!("Cannot apply window width limit: {error}"));
        let stay_on_top = context
            .storage
            .and_then(|storage| eframe::get_value(storage, "stay_on_top"))
            .unwrap_or(false);
        context
            .egui_ctx
            .send_viewport_cmd(egui::ViewportCommand::WindowLevel(if stay_on_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            }));
        let (sender, actions) = mpsc::channel();
        let menu = Menu::new();
        let show = MenuItem::new(
            if profile == Profile::Development {
                "Show Development"
            } else {
                "Show Focus"
            },
            true,
            None,
        );
        let toggle = MenuItem::new("Start tracking", false, None);
        let exit = MenuItem::new("Exit", true, None);
        let tray_result = (|| -> Result<TrayIcon, Box<dyn std::error::Error>> {
            menu.append_items(&[&show, &toggle, &exit])?;
            Ok(TrayIconBuilder::new()
                .with_tooltip(format!("{} - opening history...", profile.title()))
                .with_icon(tray_icon(false, profile))
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .build()?)
        })();
        let (tray, error) = match tray_result {
            Ok(tray) => (Some(tray), None),
            Err(error) => (None, Some(format!("Tray unavailable: {error}"))),
        };
        if launch.hide_window(tray.is_some()) {
            drag_regions.suppress_first_show();
        }
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
                    "Cannot confirm saving before the Windows interruption. Check session recovery.".into(),
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
            settings_open: false,
            startup: attention_focus::startup_ui::Panel::new(
                profile,
                context
                    .storage
                    .and_then(|s| eframe::get_value(s, "startup_tray"))
                    .unwrap_or(false),
            ),
            drag_regions,
            motion_epoch: Instant::now(),
            tray_level: Some((false, 4)),
            running_icons: (0..=4)
                .map(|level| tray_icon_level(true, profile, level))
                .collect(),
            tray,
            toggle_item: toggle,
            actions,
            commands,
            state: tracking::Snapshot::default(),
            ready: false,
            pending: false,
            error: error.or(width_error),
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
            folder_pending: false,
            source_error: source_warning,
            activity: tracking::Activity::default(),
            lifecycle: listener,
            reports: report_ui::Panel::restore(context.storage),
            project_search: context
                .storage
                .and_then(|s| eframe::get_value(s, "project_search"))
                .unwrap_or_default(),
            restored_note: context
                .storage
                .and_then(|s| eframe::get_value::<Option<String>>(s, "note_draft"))
                .flatten(),
            profile,
            storage_root: paths.root,
            stay_on_top,
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
        let storage_root = self.storage_root.clone();
        let selection = self.source_selection.clone();
        let sender = self.sender.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
                let path = source::validate_boundary(&path, &storage_root)?;
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
        let before = self.activity.project.clone();
        let mut split = self.state.change_mode == tracking::ChangeMode::NewInterval;
        if ui
            .checkbox(
                &mut split,
                "Start a new interval when project or task changes",
            )
            .changed()
        {
            self.command(tracking::Command::SetChangeMode(if split {
                tracking::ChangeMode::NewInterval
            } else {
                tracking::ChangeMode::EditCurrent
            }));
        }
        if !split {
            ui.label("Changes reassign the entire current interval, including its elapsed time.");
        }
        egui::ComboBox::from_id_salt("project")
            .width(ui.available_width().min(320.0))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .selected_text(self.activity.label())
            .show_ui(ui, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.project_search)
                        .hint_text("Search projects"),
                )
                .on_hover_text(
                    "Filter projects by name or folder ID. Searching does not change the activity.",
                );
                ui.separator();
                let query = self.project_search.trim().to_lowercase();
                let mut choices = vec![tracking::Activity::default()];
                if let Some(source) = &self.source_selection {
                    for project in self.catalog.projects.iter().filter(|p| {
                        !p.archived
                            && p.status != "Completed"
                            && (query.is_empty()
                                || p.title.to_lowercase().contains(&query)
                                || p.slug.to_lowercase().contains(&query))
                    }) {
                        choices.push(tracking::Activity {
                            project: Some(tracking::Project {
                                source_id: source.id,
                                source_path: source.path.to_string_lossy().into_owned(),
                                slug: project.slug.clone(),
                                title: project.title.clone(),
                                task: None,
                            }),
                            note: String::new(),
                        });
                    }
                }
                self.state.sort_projects(&mut choices);
                for choice in choices {
                    let selected = match (&self.activity.project, &choice.project) {
                        (None, None) => true,
                        (Some(a), Some(b)) => {
                            a.source_id == b.source_id
                                && a.source_path == b.source_path
                                && a.slug == b.slug
                        }
                        _ => false,
                    };
                    let label = if choice.project.is_none() {
                        egui::RichText::new("Unassigned focus (no project)")
                            .strong()
                            .italics()
                            .color(ui.visuals().hyperlink_color)
                    } else {
                        egui::RichText::new(choice.label())
                    };
                    if ui.selectable_label(selected, label).clicked() {
                        if !selected {
                            self.activity.project = choice.project;
                        }
                        ui.close();
                    }
                }
            });
        if let Some(project) = &mut self.activity.project {
            egui::ComboBox::from_id_salt("task")
                .selected_text(project.task.as_deref().unwrap_or("No task"))
                .show_ui(ui, |ui| {
                    let mut no_task = project.clone();
                    no_task.task = None;
                    let mut choices = vec![tracking::Activity {
                        project: Some(no_task),
                        note: String::new(),
                    }];
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
                            let mut candidate = project.clone();
                            candidate.task = Some(task.clone());
                            choices.push(tracking::Activity {
                                project: Some(candidate),
                                note: String::new(),
                            });
                        }
                    }
                    self.state.sort_tasks(&mut choices);
                    for choice in choices {
                        let task = choice.project.unwrap().task;
                        let label = task.as_deref().unwrap_or("No task").to_owned();
                        ui.selectable_value(&mut project.task, task, label);
                    }
                });
        }
        ui.add(
            egui::TextEdit::singleline(&mut self.activity.note).hint_text("Optional session note"),
        );
        if self.activity.project != before && self.available(&self.activity) {
            self.command(tracking::Command::ChangeActivity(self.activity.clone()));
        }
        if self.activity != self.state.activity {
            ui.label(if self.pending {
                "Saving selection..."
            } else {
                "Changes have not been saved."
            });
            if ui
                .add_enabled(
                    self.ready
                        && !self.pending
                        && !self.state.needs_review
                        && self.available(&self.activity),
                    egui::Button::new(if self.state.running && split {
                        "Start new interval with changes"
                    } else if self.state.running {
                        "Save changes to current interval"
                    } else {
                        "Apply selection"
                    }),
                )
                .clicked()
            {
                self.command(tracking::Command::ChangeActivity(self.activity.clone()));
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
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        let color = visuals.panel_fill;
        [
            color.r() as f32 / 255.0,
            color.g() as f32 / 255.0,
            color.b() as f32 / 255.0,
            1.0,
        ]
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "stay_on_top", &self.stay_on_top);
        eframe::set_value(storage, "startup_tray", &self.startup.tray);
        eframe::set_value(storage, "project_search", &self.project_search);
        let draft =
            (self.activity.note != self.state.activity.note).then(|| self.activity.note.clone());
        eframe::set_value(storage, "note_draft", &draft);
        self.reports.save(storage);
    }
    fn auto_save_interval(&self) -> Duration {
        Duration::from_secs(2)
    }
    fn persist_egui_memory(&self) -> bool {
        true
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.drag_regions.take_startup_hide() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        while let Ok(action) = self.actions.try_recv() {
            match action {
                Action::Folder(result) => {
                    self.folder_pending = false;
                    match result {
                        Ok(Some(path)) => {
                            self.source_path = path.to_string_lossy().into_owned();
                            self.source_error = None;
                        }
                        Ok(None) => {}
                        Err(error) => self.source_error = Some(error),
                    }
                }
                Action::Export(result) => self.reports.export_finished(result),
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
                    self.reports.receive(id, result.map(|report| *report))
                }
                Action::Tracking(
                    event @ (tracking::Event::State(_) | tracking::Event::Committed(_)),
                ) => {
                    let completed = matches!(event, tracking::Event::Committed(_));
                    let state = match event {
                        tracking::Event::State(state) | tracking::Event::Committed(state) => state,
                        _ => unreachable!(),
                    };
                    if self.state.running != state.running {
                        self.motion_epoch = Instant::now();
                    }
                    // Update the selected project's local-day total even while hidden.
                    if let Some(tray) = &self.tray {
                        let label = presentation::tooltip(self.profile, &state);
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
                        if let Some(note) = self.restored_note.take() {
                            self.activity.note = note;
                        }
                    }
                    self.reports.tracking_changed(
                        &state,
                        completed,
                        jiff::Timestamp::now().as_millisecond(),
                    );
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
        let level = attention_focus::motion::tray_level(
            self.motion_epoch.elapsed().as_secs_f64(),
            attention_focus::motion::allowed(),
        );
        let key = (
            self.state.running,
            if self.state.running { level } else { 4 },
        );
        if self.tray_level != Some(key)
            && let Some(tray) = &self.tray
        {
            let icon = if self.state.running {
                self.running_icons[level as usize].clone()
            } else {
                tray_icon(false, self.profile)
            };
            if let Err(error) = tray.set_icon(Some(icon)) {
                self.error = Some(format!("Cannot update tray icon: {error}"));
            }
            self.tray_level = Some(key);
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

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let secondary = if ui.visuals().dark_mode {
            egui::Color32::from_rgb(151, 193, 226)
        } else {
            egui::Color32::from_rgb(43, 79, 117)
        };
        ui.style_mut().visuals.weak_text_color = Some(secondary);
        ui.style_mut().visuals.disabled_alpha = 0.7;
        let ctx = ui.ctx().clone();
        let geometry = ctx.input(|input| input.viewport().outer_rect);
        if geometry != self.last_geometry {
            self.last_geometry = geometry;
            ctx.request_repaint();
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
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, ui.visuals().panel_fill);
        let mut drag_regions = Vec::new();
        let mut reveal_settings = false;
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            ui.set_max_width(ui.available_width().min(744.0));
            // These actions stay outside the scrolling body at every content height.
            let heading = ui.horizontal(|ui| {
                ui.set_min_width(ui.available_width());
                ui.heading("Attention Focus");
                ui.weak(format!(
                    "{}v{}",
                    if self.profile == Profile::Development {
                        "Development "
                    } else {
                        ""
                    },
                    env!("CARGO_PKG_VERSION")
                ));
            });
            drag_regions.push(heading.response.rect);
            ui.horizontal(|ui| {
                if ui.checkbox(&mut self.stay_on_top, "Stay on top").changed() {
                    if let Some(storage) = frame.storage_mut() {
                        eframe::set_value(storage, "stay_on_top", &self.stay_on_top);
                        storage.flush();
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                        if self.stay_on_top {
                            egui::WindowLevel::AlwaysOnTop
                        } else {
                            egui::WindowLevel::Normal
                        },
                    ));
                }
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
                if ui
                    .add(egui::Button::new("Settings").selected(self.settings_open))
                    .clicked()
                {
                    self.settings_open = !self.settings_open;
                    reveal_settings = self.settings_open;
                }
                let free = ui.available_rect_before_wrap();
                let rect = egui::Rect::from_min_size(
                    free.min,
                    egui::vec2(free.width().max(0.0), ui.spacing().interact_size.y),
                );
                drag_regions.push(rect);
            });
            ui.separator();
            attention_focus::ui_support::body(ui, |ui| {
                if self.settings_open {
                    let settings = ui.group(|ui| {
                        ui.heading("Application settings");
                        self.startup.show(ui);
                    });
                    if reveal_settings {
                        settings.response.scroll_to_me(Some(egui::Align::Min));
                    }
                    ui.separator();
                }
                ui.label(
                    egui::RichText::new(if self.state.running {
                        "Tracking now (saved activity)"
                    } else {
                        "Next tracking activity (saved selection)"
                    })
                    .color(secondary)
                    .strong(),
                );
                ui.strong(self.state.activity.label());
                if let Some(task) = self
                    .state
                    .activity
                    .project
                    .as_ref()
                    .and_then(|p| p.task.as_ref())
                {
                    ui.label(task);
                }
                ui.add_enabled_ui(
                    self.ready && !self.pending && !self.state.needs_review,
                    |ui| self.activity_picker(ui),
                );
                ui.add_space(12.0);
                let enabled = self.ready
                    && !self.pending
                    && !self.state.needs_review
                    && (self.state.running
                        || (self.lifecycle.is_some() && self.available(&self.activity)));
                if attention_focus::counters::show(ui, &self.state, self.ready, enabled).toggle {
                    self.command(if self.state.running {
                        tracking::Command::Stop
                    } else {
                        tracking::Command::Start(self.activity.clone())
                    });
                }
                ui.add_space(8.0);
                ui.separator();
                ui.label(format!(
                    "All projects, all time (saved): {} in {} sessions",
                    duration(self.state.total_ms),
                    self.state.completed
                ));
                if self.state.needs_review {
                    ui.label("An interrupted session was saved up to its last checkpoint.");
                    ui.label("Time after that checkpoint is excluded. Tracking has not resumed.");
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
                    ui.horizontal(|ui| {
                        let width = (ui.available_width() - 84.0).max(80.0);
                        ui.add_enabled_ui(!self.source_pending, |ui| {
                            ui.add_sized(
                                [width, 22.0],
                                egui::TextEdit::singleline(&mut self.source_path),
                            );
                        });
                        if ui
                            .add_enabled(
                                !self.folder_pending && !self.source_pending,
                                egui::Button::new("Browse..."),
                            )
                            .clicked()
                        {
                            self.folder_pending = true;
                            let initial = std::path::PathBuf::from(&self.source_path);
                            let owner = unsafe {
                                windows_sys::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow()
                            } as usize;
                            let sender = self.sender.clone();
                            let ctx = ctx.clone();
                            std::thread::spawn(move || {
                                let result = attention_focus::folder_picker::pick(owner, &initial)
                                    .map_err(|error| format!("Cannot choose folder: {error}"));
                                let _ = sender.send(Action::Folder(result));
                                ctx.request_repaint();
                            });
                        }
                    });
                    if ui
                        .add_enabled(!self.source_pending, egui::Button::new("Save and refresh"))
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
                if let Some((format, request)) = self.reports.take_export() {
                    let directory = self.directory.clone();
                    let root = self.storage_root.clone();
                    let sender = self.sender.clone();
                    let ctx = ctx.clone();
                    let owner = unsafe {
                        windows_sys::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow()
                    } as usize;
                    std::thread::spawn(move || {
                        let result =
                            (|| -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
                                let snapshot = tracking::export_snapshot(
                                    &directory.join("focus.sqlite3"),
                                    &request,
                                )?;
                                let Some(path) = export::pick_path(owner, format)? else {
                                    return Ok("Export canceled.".into());
                                };
                                let mut protected = source::known_paths(&directory)?;
                                protected.extend(snapshot.sources);
                                protected.push(root);
                                let path =
                                    export::write(&snapshot.report, &path, format, &protected)?;
                                Ok(format!(
                                    "Exported {} sessions to {}",
                                    snapshot.report.matching_sessions,
                                    path.display()
                                ))
                            })()
                            .map_err(|error| format!("Cannot export report: {error}"));
                        let _ = sender.send(Action::Export(result));
                        ctx.request_repaint();
                    });
                }
            });
        });
        self.drag_regions.set(&drag_regions, ctx.pixels_per_point());
        attention_focus::ui_support::maximized_edge(&ctx);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let launch = attention_focus::startup::Launch::parse(
        &std::env::args().skip(1).collect::<Vec<_>>(),
        cfg!(debug_assertions),
    )?;
    let profile = launch.profile;
    let paths = Paths::new(&platform::data_root()?, profile)?;
    let _instance = match platform::Instance::acquire(&paths.directory, profile.title()) {
        Ok(instance) => instance,
        Err(error) if launch.autostart && error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    // Restore read-only and validate source overlap before SQLite or eframe can write.
    let (selection, warning) = paths
        .prepare()
        .map_err(|error| -> Box<dyn std::error::Error> { error })?;
    std::fs::create_dir_all(&paths.directory)?;
    let window_state = paths.directory.join("window.ron");
    eframe::run_native(
        profile.title(),
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_app_id(profile.app_id())
                .with_inner_size([620.0, 640.0])
                .with_min_inner_size([420.0, 360.0])
                .with_max_inner_size([760.0, 16_384.0])
                .with_icon(egui::IconData {
                    rgba: icons::rgba(
                        64,
                        if profile == Profile::Development {
                            icons::State::AppDevelopment
                        } else {
                            icons::State::App
                        },
                    ),
                    width: 64,
                    height: 64,
                }),
            persist_window: true,
            persistence_path: Some(window_state),
            ..Default::default()
        },
        Box::new(move |context| {
            Ok(Box::new(Desktop::new(
                context, paths, selection, warning, launch,
            )))
        }),
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
