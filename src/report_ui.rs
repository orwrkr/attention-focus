//! Report controls; database access remains on the tracking worker.
use crate::{
    date_picker::Calendar,
    i18n::{
        message, message_plural, paragraph, paragraph_colored, paragraph_weak, t, tf, tooltip, user,
    },
    presentation::duration,
    reports, tracking,
};
use eframe::egui;
use std::{
    sync::mpsc::SyncSender,
    time::{Duration, Instant},
};

const REFRESH_INTERVAL: Duration = Duration::from_secs(2);
pub struct Panel {
    request: reports::Request,
    report: Option<reports::Report>,
    pending: bool,
    request_id: u64,
    error: Option<String>,
    sent_request: Option<reports::Request>,
    last_attempt: Option<Instant>,
    debounce: Option<Instant>,
    dirty: bool,
    preset: Option<usize>,
    from_time_picker: crate::time_picker::TimePicker,
    through_time_picker: crate::time_picker::TimePicker,
    from_calendar: Calendar,
    through_calendar: Calendar,
    zones: Vec<String>,
    zone_search: String,
    active_start: Option<i64>,
    needs_review: bool,
    project_titles: Vec<(i64, String)>,
    chart_view: crate::charts::View,
    project_colors: crate::charts::ProjectColors,
    growing: bool,
    was_open: bool,
    export_request: Option<crate::export::Format>,
    export_pending: bool,
    export_message: Option<String>,
}

impl Default for Panel {
    fn default() -> Self {
        let mut zones: Vec<_> = jiff::tz::db()
            .available()
            .map(|name| name.to_string())
            .collect();
        if !zones.iter().any(|name| name == "UTC") {
            zones.push("UTC".into());
        }
        zones.sort_unstable_by_key(|name| (name != "UTC", name.clone()));
        Self {
            request: reports::Request::default(),
            report: None,
            pending: false,
            request_id: 0,
            error: None,
            sent_request: None,
            last_attempt: None,
            debounce: None,
            dirty: true,
            preset: Some(1),
            from_time_picker: Default::default(),
            through_time_picker: Default::default(),
            from_calendar: Calendar::default(),
            through_calendar: Calendar::default(),
            zones,
            zone_search: String::new(),
            active_start: None,
            needs_review: false,
            project_titles: Vec::new(),
            chart_view: Default::default(),
            project_colors: Default::default(),
            growing: false,
            was_open: false,
            export_request: None,
            export_pending: false,
            export_message: None,
        }
    }
}
/// The hint under the period buttons, in logical order.
fn period_hint(preset: Option<usize>, timezone: &str) -> String {
    match preset {
        Some(1) => message("reports.period_today_hint", &[("timezone", timezone)]),
        Some(days) => message_plural(
            "reports.period_days_hint",
            days as u64,
            &[("timezone", timezone)],
        ),
        None => message("reports.period_fixed_hint", &[]),
    }
}
/// A project name for the totals; time without a project carries the English label from the worker.
fn group_project(group: &reports::Group) -> std::borrow::Cow<'_, str> {
    if group.project_id.is_none() {
        std::borrow::Cow::Borrowed(t("activity.unassigned"))
    } else {
        user(&group.project)
    }
}
impl Panel {
    pub fn take_export(&mut self) -> Option<(crate::export::Format, reports::Request)> {
        self.export_request
            .take()
            .map(|format| (format, self.request.clone()))
    }
    pub fn export_finished(&mut self, result: Result<String, String>) {
        self.export_pending = false;
        match result {
            Ok(message) => self.export_message = Some(message),
            Err(error) => self.error = Some(error),
        }
    }
    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(
            storage,
            "report_filter_v1",
            &(self.request.clone(), self.preset),
        );
        eframe::set_value(storage, "timezone_search", &self.zone_search);
        eframe::set_value(storage, "chart_view", &self.chart_view);
        eframe::set_value(storage, "project_colors", &self.project_colors);
    }
    pub fn restore(storage: Option<&dyn eframe::Storage>) -> Self {
        let mut panel = Self::default();
        if let Some(storage) = storage {
            if let Some((request, preset)) =
                eframe::get_value::<(reports::Request, Option<usize>)>(storage, "report_filter_v1")
            {
                panel.request = request;
                panel.preset = preset.filter(|days| [1, 7, 30].contains(days));
                if let Some(days) = panel.preset
                    && let Ok(request) = reports::Request::recent(days, &panel.request.timezone)
                {
                    panel.request = request;
                }
            }
            panel.zone_search = eframe::get_value(storage, "timezone_search").unwrap_or_default();
            panel.chart_view = eframe::get_value(storage, "chart_view").unwrap_or_default();
            panel.project_colors = eframe::get_value(storage, "project_colors").unwrap_or_default();
        }
        panel
    }
    pub fn receive(&mut self, id: u64, result: std::result::Result<reports::Report, String>) {
        if id != self.request_id {
            return;
        }
        self.pending = false;
        if self.sent_request.take().as_ref() != Some(&self.request) {
            self.dirty = true;
            return;
        }
        match result {
            Ok(report) => {
                self.report = Some(report);
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }
    pub fn tracking_changed(&mut self, state: &tracking::Snapshot, committed: bool, wall_ms: i64) {
        self.project_colors.update(&state.projects);
        // Totals show current project names, so a rename must refresh a report on screen.
        let titles: Vec<_> = state
            .projects
            .iter()
            .map(|project| (project.id, project.title.clone()))
            .collect();
        if titles != self.project_titles {
            self.project_titles = titles;
            self.dirty = true;
        }
        let next = if state.running {
            state.active_start_ms
        } else {
            None
        };
        if committed || self.active_start != next || self.needs_review != state.needs_review {
            if self.request.bounds().is_ok_and(|(begin, end)| {
                [self.active_start, next]
                    .into_iter()
                    .flatten()
                    .any(|start| start < end && wall_ms >= begin && wall_ms >= start)
            }) {
                self.dirty = true;
            }
            if self.needs_review != state.needs_review
                && self.report.as_ref().is_some_and(|report| {
                    report.range.request == self.request
                        && (report.running_ms > 0
                            || report.review_ms > 0
                            || report
                                .history
                                .iter()
                                .any(|row| row.record.state != reports::State::Saved))
                })
            {
                self.dirty = true;
            }
        }
        self.active_start = next;
        self.needs_review = state.needs_review;
    }
    /// Report labels come from the worker, so a new language needs a new report.
    pub fn language_changed(&mut self) {
        self.dirty = true;
    }
    fn edited(&mut self, now: Instant) {
        self.dirty = true;
        self.debounce = Some(now + Duration::from_millis(350));
    }
    /// Runs before debounce and in-flight gating, so visible filters and exports always
    /// describe the current date. A reply for the previous date is rejected in `receive`.
    fn roll_period(&mut self, wall_ms: i64) {
        if let Some(days) = self.preset
            && let Ok(now) = jiff::Timestamp::from_millisecond(wall_ms)
            && let Ok(request) = reports::Request::recent_at(days, &self.request.timezone, now)
            && request != self.request
        {
            self.request = request;
            self.dirty = true;
        }
    }
    fn tick_at(
        &mut self,
        commands: &SyncSender<tracking::Command>,
        open: bool,
        now: Instant,
        wall_ms: i64,
    ) {
        if !open {
            self.was_open = false;
            return;
        }
        if !self.was_open {
            self.dirty = true;
            self.was_open = true;
        }
        self.roll_period(wall_ms);
        if self.debounce.is_some_and(|deadline| now < deadline) {
            return;
        }
        self.debounce = None;
        if self.pending {
            return;
        }
        let growing = self.active_start.is_some_and(|start| {
            self.request
                .bounds()
                .is_ok_and(|(begin, end)| start <= wall_ms && begin <= wall_ms && wall_ms < end)
        });
        if self.growing && !growing {
            self.dirty = true;
        }
        self.growing = growing;
        if !self.dirty
            && (!growing
                || self
                    .last_attempt
                    .is_some_and(|last| now.saturating_duration_since(last) < REFRESH_INTERVAL))
        {
            return;
        }
        self.last_attempt = Some(now);
        self.dirty = false;
        self.request_id = self.request_id.wrapping_add(1);
        match commands.try_send(tracking::Command::Report {
            id: self.request_id,
            request: self.request.clone(),
        }) {
            Ok(()) => {
                self.pending = true;
                self.sent_request = Some(self.request.clone());
            }
            Err(error) => {
                self.error = Some(message(
                    "errors.report_request_failed",
                    &[("error", &error.to_string())],
                ));
            }
        }
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        commands: &SyncSender<tracking::Command>,
        ready: bool,
    ) {
        let now = Instant::now();
        let response = egui::CollapsingHeader::new(t("reports.title"))
            // The identity of the header stays fixed when the language changes.
            .id_salt("Focus history and reports")
            .show(ui, |ui| {
                crate::i18n::mirrored(ui, |ui| {
                    ui.add_enabled_ui(ready, |ui| {
                        let mut dates_changed = false;
                        let old_zone = self.request.timezone.clone();
                        // Translated labels do not fit beside their fields in a narrow window.
                        let stacked = ui.available_width() < 470.0;
                        crate::i18n::unmirrored(ui, |ui| {
                            egui::Grid::new("report-range")
                                .num_columns(2)
                                .show(ui, |ui| {
                                    ui.label(t("reports.from"));
                                    if stacked {
                                        ui.end_row();
                                    }
                                    dates_changed |= self.from_calendar.show(
                                        ui,
                                        &mut self.request.from,
                                        "from",
                                        &self.request.timezone,
                                    );
                                    ui.end_row();
                                    ui.label(t("reports.from_time"));
                                    if stacked {
                                        ui.end_row();
                                    }
                                    dates_changed |= self.from_time_picker.show(
                                        ui,
                                        &mut self.request.from_time,
                                        "from-time",
                                        false,
                                    );
                                    ui.end_row();
                                    ui.label(t("reports.until"));
                                    if stacked {
                                        ui.end_row();
                                    }
                                    dates_changed |= self.through_calendar.show(
                                        ui,
                                        &mut self.request.through,
                                        "through",
                                        &self.request.timezone,
                                    );
                                    ui.end_row();
                                    ui.label(t("reports.until_time"));
                                    if stacked {
                                        ui.end_row();
                                    }
                                    dates_changed |= self.through_time_picker.show(
                                        ui,
                                        &mut self.request.through_time,
                                        "until-time",
                                        true,
                                    );
                                    ui.end_row();
                                    ui.label(t("reports.time_zone"));
                                    if stacked {
                                        ui.end_row();
                                    }
                                    egui::ComboBox::from_id_salt("report-timezone")
                                        .selected_text(&self.request.timezone)
                                        .width(235.0)
                                        .show_ui(ui, |ui| {
                                            ui.add(
                                                egui::TextEdit::singleline(&mut self.zone_search)
                                                    .hint_text(t("reports.search_time_zones")),
                                            );
                                            let query = self.zone_search.trim().to_lowercase();
                                            let mut found = false;
                                            for zone in self
                                                .zones
                                                .iter()
                                                .filter(|zone| zone.to_lowercase().contains(&query))
                                            {
                                                found = true;
                                                ui.selectable_value(
                                                    &mut self.request.timezone,
                                                    zone.clone(),
                                                    zone,
                                                );
                                            }
                                            if !found {
                                                ui.label(t("reports.no_matching_zones"));
                                            }
                                        });
                                    ui.end_row();
                                });
                        });
                        if dates_changed {
                            self.preset = None;
                            self.edited(now);
                        }
                        if old_zone != self.request.timezone {
                            self.edited(now);
                        }
                        ui.horizontal_wrapped(|ui| {
                            for (label, days) in [
                                (t("reports.today"), 1),
                                (t("reports.last_7_days"), 7),
                                (t("reports.last_30_days"), 30),
                            ] {
                                if ui
                                    .add(
                                        egui::Button::new(label)
                                            .selected(self.preset == Some(days)),
                                    )
                                    .clicked()
                                {
                                    match reports::Request::recent(days, &self.request.timezone) {
                                        Ok(request) => {
                                            self.request = request;
                                            self.preset = Some(days);
                                            self.debounce = None;
                                            self.dirty = true;
                                        }
                                        Err(error) => {
                                            self.error = Some(message(
                                                "errors.set_dates_failed",
                                                &[("error", &error.to_string())],
                                            ))
                                        }
                                    }
                                }
                            }
                            let custom = ui.add(
                                egui::Button::new(t("reports.custom"))
                                    .selected(self.preset.is_none()),
                            );
                            if tooltip(custom, &message("reports.custom_hint", &[])).clicked() {
                                self.preset = None;
                            }
                            if ui.button(t("reports.refresh")).clicked() {
                                self.debounce = None;
                                self.dirty = true;
                            }
                        });
                        paragraph_weak(ui, &period_hint(self.preset, &self.request.timezone));
                    });
                    if !ready {
                        ui.label(t("reports.history_unavailable"));
                    }
                    ui.horizontal_wrapped(|ui| {
                        for (label, format) in [
                            (t("reports.export_csv"), crate::export::Format::Csv),
                            (
                                t("reports.export_markdown"),
                                crate::export::Format::Markdown,
                            ),
                        ] {
                            if ui
                                .add_enabled(
                                    ready && !self.export_pending && self.request.bounds().is_ok(),
                                    egui::Button::new(label),
                                )
                                .clicked()
                            {
                                self.export_request = Some(format);
                                self.export_pending = true;
                                self.export_message = None;
                            }
                        }
                    });
                    if self.export_pending {
                        ui.weak(t("reports.preparing_export"));
                    }
                    if let Some(text) = &self.export_message {
                        paragraph(ui, text);
                    }
                    paragraph_weak(
                        ui,
                        &if self.pending {
                            message("reports.updating", &[])
                        } else if self.growing {
                            message("reports.auto_refresh", &[])
                        } else {
                            message("reports.auto_refresh_paused", &[])
                        },
                    );
                    if let Some(error) = &self.error {
                        paragraph_colored(ui, ui.visuals().error_fg_color, error);
                    }
                    if let Some(report) = &self.report {
                        Self::results(ui, report, &mut self.chart_view, &self.project_colors);
                    } else {
                        paragraph(ui, &message("reports.loads_automatically", &[]));
                    }
                });
            });
        let open = response.body_returned.is_some() && ui.is_visible();
        self.tick_at(
            commands,
            ready && open,
            now,
            jiff::Timestamp::now().as_millisecond(),
        );
        if open && ready {
            ui.ctx()
                .request_repaint_after(self.debounce.map_or(REFRESH_INTERVAL, |deadline| {
                    deadline.saturating_duration_since(now)
                }));
        }
    }
    fn results(
        ui: &mut egui::Ui,
        report: &reports::Report,
        chart_view: &mut crate::charts::View,
        project_colors: &crate::charts::ProjectColors,
    ) {
        let request = &report.range.request;
        paragraph(
            ui,
            &message(
                "reports.results",
                &[
                    ("from", &request.from),
                    ("from_time", &request.from_time),
                    ("through", &request.through),
                    ("through_time", &request.through_time),
                    ("timezone", &request.timezone),
                ],
            ),
        );
        ui.weak(tf(
            "reports.snapshot",
            &[("time", &report.range.format_time(report.generated_ms))],
        ));
        ui.strong(tf(
            "reports.total_with_running",
            &[(
                "duration",
                &duration(report.saved_ms.saturating_add(report.running_ms)),
            )],
        ));
        ui.label(tf(
            "reports.saved_focus",
            &[("duration", &duration(report.saved_ms))],
        ));
        let running = ui.label(tf(
            "reports.running_now",
            &[("duration", &duration(report.running_ms))],
        ));
        tooltip(running, &message("reports.running_now_hint", &[]));
        if report.review_ms > 0 {
            let interrupted = ui.label(tf(
                "reports.interrupted",
                &[("duration", &duration(report.review_ms))],
            ));
            tooltip(interrupted, &message("reports.interrupted_hint", &[]));
        }
        // Exports carry the English diagnostics; the interface describes the same sessions itself.
        for id in &report.invalid_sessions {
            paragraph_colored(
                ui,
                ui.visuals().warn_fg_color,
                &message("reports.invalid_interval", &[("id", &id.to_string())]),
            );
        }
        if let Some(chart) = &report.chart {
            egui::CollapsingHeader::new(t("reports.charts"))
                .id_salt("Charts")
                .default_open(true)
                .show(ui, |ui| {
                    crate::i18n::mirrored(ui, |ui| {
                        crate::charts::show(ui, chart, chart_view, project_colors);
                    });
                });
        }
        egui::CollapsingHeader::new(t("reports.by_project"))
            .id_salt("By project")
            .show(ui, |ui| {
                egui::Grid::new("project-totals")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong(t("reports.project"));
                        ui.strong(t("reports.saved_time"));
                        ui.end_row();
                        for group in &report.projects {
                            ui.label(group_project(group).as_ref());
                            ui.label(duration(group.saved_ms));
                            ui.end_row();
                        }
                        ui.strong(t("reports.total_saved"));
                        ui.strong(duration(report.saved_ms));
                        ui.end_row();
                    });
            });
        egui::CollapsingHeader::new(t("reports.by_project_and_task"))
            .id_salt("By project and task")
            .show(ui, |ui| {
                egui::Grid::new("task-totals").striped(true).show(ui, |ui| {
                    ui.strong(t("reports.project"));
                    ui.strong(t("reports.task"));
                    ui.strong(t("reports.saved_time"));
                    ui.end_row();
                    for group in &report.groups {
                        ui.label(group_project(group).as_ref());
                        ui.label(group.task.as_deref().map_or_else(
                            || t("main.no_task").to_owned(),
                            |task| user(task).into_owned(),
                        ));
                        ui.label(duration(group.saved_ms));
                        ui.end_row();
                    }
                    ui.strong(t("reports.total_saved"));
                    ui.label("");
                    ui.strong(duration(report.saved_ms));
                    ui.end_row();
                });
            });
        egui::CollapsingHeader::new(t("reports.by_day"))
            .id_salt("By day")
            .show(ui, |ui| {
                crate::ui_support::inner_area("daily-focus", 240.0).show(ui, |ui| {
                    egui::Grid::new("daily-totals")
                        .striped(true)
                        .show(ui, |ui| {
                            ui.strong(t("reports.day"));
                            ui.strong(t("reports.saved_time"));
                            ui.end_row();
                            for day in &report.range.days {
                                ui.label(&day.date);
                                ui.label(duration(day.saved_ms));
                                ui.end_row();
                            }
                            ui.strong(t("reports.total_saved"));
                            ui.strong(duration(report.saved_ms));
                            ui.end_row();
                        });
                });
            });
        // The text holds a count and is translated, so it cannot identify the section.
        egui::CollapsingHeader::new(tf(
            "reports.sessions",
            &[("count", &report.matching_sessions.to_string())],
        ))
        .id_salt("Sessions")
        .show(ui, |ui| {
            if report.matching_sessions > report.history.len() {
                paragraph(ui, &message("reports.showing_recent", &[]));
            }
            paragraph(ui, &message("reports.clipped_hint", &[]));
            crate::ui_support::inner_area("session-history", 360.0).show(ui, |ui| {
                for row in &report.history {
                    let record = &row.record;
                    let status = match record.state {
                        reports::State::Saved => message("reports.status_saved", &[]),
                        reports::State::Running => message("reports.status_running", &[]),
                        reports::State::Review => message("reports.status_interrupted", &[]),
                    };
                    ui.push_id(record.id, |ui| {
                        // The text holds a running duration, so it cannot identify the row.
                        egui::CollapsingHeader::new(tf(
                            "reports.session_row",
                            &[
                                ("start", &report.range.format_time(record.start_ms)),
                                ("duration", &duration(row.clipped_ms)),
                                ("status", &status),
                            ],
                        ))
                        .id_salt("session")
                        .show(ui, |ui| {
                            ui.strong(record.activity.painted_label().as_ref());
                            if let Some(project) = &record.activity.project
                                && let Some(task) = &project.task
                            {
                                ui.label(user(task).as_ref());
                            }
                            if !record.activity.note.is_empty() {
                                ui.label(user(&record.activity.note).as_ref());
                            }
                            ui.label(tf(
                                "reports.start",
                                &[("time", &report.range.format_time(record.start_ms))],
                            ));
                            ui.label(tf(
                                "reports.end",
                                &[("time", &report.range.format_time(record.end_ms))],
                            ));
                            ui.label(tf(
                                "reports.full_time",
                                &[("duration", &duration(record.elapsed_ms))],
                            ));
                            let reason = match record.reason.as_str() {
                                "stop" => message("reports.reason.stop", &[]),
                                "switch" => message("reports.reason.switch", &[]),
                                "day_boundary" => message("reports.reason.day_boundary", &[]),
                                "exit" => message("reports.reason.exit", &[]),
                                "lock" => message("reports.reason.lock", &[]),
                                "logoff" => message("reports.reason.logoff", &[]),
                                "shutdown" => message("reports.reason.shutdown", &[]),
                                "suspend" => message("reports.reason.suspend", &[]),
                                "controller_lost" => message("reports.reason.controller_lost", &[]),
                                "crash" => message("reports.reason.crash", &[]),
                                "clock_or_heartbeat" => {
                                    message("reports.reason.clock_or_heartbeat", &[])
                                }
                                _ => message("reports.reason.unknown", &[]),
                            };
                            if record.state != reports::State::Running {
                                ui.label(tf("reports.ended", &[("reason", &reason)]));
                            }
                        });
                    });
                }
            });
        });
        if report.matching_sessions == 0 {
            ui.label(t("reports.no_sessions"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::Storage;
    use std::sync::mpsc;

    #[derive(Default)]
    struct MemoryStorage(std::collections::HashMap<String, String>);
    impl eframe::Storage for MemoryStorage {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.into(), value);
        }
        fn remove_string(&mut self, key: &str) {
            self.0.remove(key);
        }
        fn flush(&mut self) {}
    }
    #[test]
    fn restore_keeps_custom_minutes_zone_and_search_but_not_inflight_work() {
        let mut panel = fixed();
        panel.request.from_time = "09:15".into();
        panel.request.through_time = "10:30".into();
        panel.zone_search = "UTC".into();
        panel.pending = true;
        let mut storage = MemoryStorage::default();
        panel.save(&mut storage);
        let restored = Panel::restore(Some(&storage));
        assert_eq!(restored.request, panel.request);
        assert_eq!(restored.preset, None);
        assert_eq!(restored.zone_search, "UTC");
        assert!(!restored.pending && restored.report.is_none() && restored.dirty);
        panel.preset = Some(7);
        panel.save(&mut storage);
        assert_eq!(
            Panel::restore(Some(&storage)).request,
            reports::Request::recent(7, "UTC").unwrap()
        );
        storage.set_string("report_filter_v1", "broken preferences".into());
        assert_eq!(Panel::restore(Some(&storage)).preset, Some(1));
    }

    fn moment(text: &str) -> jiff::Timestamp {
        text.parse().unwrap()
    }
    fn following(days: usize, timezone: &str, now: jiff::Timestamp) -> Panel {
        Panel {
            request: reports::Request::recent_at(days, timezone, now).unwrap(),
            preset: Some(days),
            ..Panel::default()
        }
    }
    #[test]
    fn today_moves_to_the_new_date_at_midnight_even_with_a_reply_in_flight() {
        let (before, after) = (
            moment("2026-03-10T23:59:59Z"),
            moment("2026-03-11T00:00:01Z"),
        );
        let mut panel = following(1, "UTC", before);
        let (sender, receiver) = mpsc::sync_channel(8);
        let now = Instant::now();
        panel.tick_at(&sender, true, now, before.as_millisecond());
        receiver.try_recv().unwrap();
        let stale = response(&panel.request);
        let id = panel.request_id;
        panel.tick_at(&sender, true, now, after.as_millisecond());
        assert_eq!(panel.request.from, "2026-03-11");
        assert_eq!(panel.request.through, "2026-03-11");
        assert!(receiver.try_recv().is_err(), "one query stays in flight");
        panel.receive(id, Ok(stale));
        assert!(
            panel.report.is_none(),
            "the previous date must not be shown"
        );
        panel.tick_at(&sender, true, now, after.as_millisecond());
        receiver.try_recv().unwrap();
        panel.receive(panel.request_id, Ok(response(&panel.request)));
        assert_eq!(
            panel.report.as_ref().unwrap().range.request.from,
            "2026-03-11"
        );
        // The rollover triggers exactly one query while nothing grows in the range.
        panel.tick_at(
            &sender,
            true,
            now + REFRESH_INTERVAL,
            after.as_millisecond(),
        );
        assert!(receiver.try_recv().is_err());
    }
    #[test]
    fn rolling_periods_keep_their_length_and_use_the_selected_time_zone() {
        let early = moment("2026-03-10T14:59:00Z");
        let late = moment("2026-03-10T15:01:00Z");
        let mut panel = following(7, "Asia/Tokyo", early);
        assert_eq!(panel.request.through, "2026-03-10");
        panel.roll_period(late.as_millisecond());
        assert_eq!(panel.request.from, "2026-03-05");
        assert_eq!(panel.request.through, "2026-03-11");
        assert_eq!(panel.request.timezone, "Asia/Tokyo");
        assert_eq!(panel.preset, Some(7));
    }
    #[test]
    fn custom_dates_stay_fixed_across_midnight_and_restart() {
        let mut panel = fixed();
        let original = panel.request.clone();
        let (sender, receiver) = mpsc::sync_channel(8);
        let (begin, end) = panel.request.bounds().unwrap();
        let now = Instant::now();
        panel.tick_at(&sender, true, now, begin + 1000);
        receiver.try_recv().unwrap();
        panel.receive(panel.request_id, Ok(response(&panel.request)));
        panel.tick_at(&sender, true, now + REFRESH_INTERVAL, end + 86_400_000);
        assert_eq!(panel.request, original);
        assert!(receiver.try_recv().is_err());
        let mut storage = MemoryStorage::default();
        panel.save(&mut storage);
        let restored = Panel::restore(Some(&storage));
        assert_eq!((restored.request, restored.preset), (original, None));
    }
    #[test]
    fn period_buttons_switch_between_following_and_fixed_dates() {
        let context = egui::Context::default();
        let mut panel = Panel {
            request: reports::Request::recent(1, "UTC").unwrap(),
            ..Panel::default()
        };
        let today = panel.request.clone();
        let (sender, _receiver) = mpsc::sync_channel(64);
        let frame = |panel: &mut Panel, events| {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(420.0, 900.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| panel.show(ui, &sender, true),
            );
            output.textures_delta.clear();
            output
        };
        fn pointer(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ]
        }
        let click = |panel: &mut Panel, label: &str| {
            frame(panel, vec![]);
            let output = frame(panel, vec![]);
            let pos = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(text) if text.galley.job.text == label => {
                        Some(text.pos + egui::vec2(3.0, 3.0))
                    }
                    _ => None,
                })
                .expect("Report control must be visible");
            frame(panel, pointer(pos, true));
            frame(panel, pointer(pos, false));
        };
        click(&mut panel, "Focus history and reports");
        assert_eq!(panel.preset, Some(1));
        click(&mut panel, "Custom");
        assert_eq!(panel.preset, None);
        assert_eq!(panel.request, today, "Custom keeps the visible dates");
        click(&mut panel, "Last 7 days");
        assert_eq!(panel.preset, Some(7));
        click(&mut panel, "Today");
        assert_eq!(panel.preset, Some(1));
        assert_eq!(panel.request.through, panel.request.from);
    }
    #[test]
    fn chart_view_and_project_colors_survive_restart() {
        let mut panel = fixed();
        panel.chart_view = crate::charts::View::Rhythm;
        panel.tracking_changed(
            &tracking::Snapshot {
                projects: vec![tracking::CatalogProject {
                    id: 4,
                    title: "Alpha".into(),
                    archived: false,
                    used: true,
                    tasks: Vec::new(),
                }],
                ..tracking::Snapshot::default()
            },
            false,
            0,
        );
        assert_ne!(panel.project_colors, Default::default());
        let mut storage = MemoryStorage::default();
        panel.save(&mut storage);
        let restored = Panel::restore(Some(&storage));
        assert_eq!(restored.chart_view, crate::charts::View::Rhythm);
        assert_eq!(restored.project_colors, panel.project_colors);
        // Unreadable preferences fall back to defaults instead of blocking the report.
        storage.set_string("chart_view", "broken preferences".into());
        storage.set_string("project_colors", "broken preferences".into());
        let restored = Panel::restore(Some(&storage));
        assert_eq!(restored.chart_view, crate::charts::View::Timeline);
        assert_eq!(restored.project_colors, Default::default());
    }
    #[test]
    fn the_sessions_section_stays_open_when_its_count_or_language_changes() {
        let context = egui::Context::default();
        let panel = fixed();
        let (begin, _) = panel.request.bounds().unwrap();
        let report_with = |count: i64| {
            let records = (0..count)
                .map(|id| reports::Record {
                    id: id + 1,
                    start_ms: begin + id * 3_600_000,
                    end_ms: begin + (id + 1) * 3_600_000,
                    elapsed_ms: 3_600_000,
                    state: reports::State::Saved,
                    activity: tracking::Activity::default(),
                    reason: "stop".into(),
                })
                .collect();
            reports::compute(reports::Range::new(&panel.request).unwrap(), records, begin)
        };
        let mut step = 0;
        let mut pass = |report: &reports::Report, events: Vec<egui::Event>| {
            step += 1;
            let mut output = context.run_ui(
                egui::RawInput {
                    time: Some(f64::from(step) * 0.2),
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(700.0, 1600.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let mut view = crate::charts::View::default();
                    let colors = crate::charts::ProjectColors::default();
                    Panel::results(ui, report, &mut view, &colors);
                },
            );
            output.textures_delta.clear();
            output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => {
                        Some((text.galley.job.text.clone(), text.visual_bounding_rect()))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let one = report_with(1);
        let shown = pass(&one, vec![]);
        let header = shown
            .iter()
            .find(|(text, _)| text == "Sessions (1)")
            .expect("the section header is painted")
            .1
            .center();
        pass(&one, vec![egui::Event::PointerMoved(header)]);
        pass(
            &one,
            vec![
                egui::Event::PointerButton {
                    pos: header,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::default(),
                },
                egui::Event::PointerButton {
                    pos: header,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::default(),
                },
            ],
        );
        for _ in 0..6 {
            pass(&one, vec![]);
        }
        let has = |texts: &[(String, egui::Rect)], wanted: &str| {
            texts.iter().any(|(text, _)| text == wanted)
        };
        let hint = message("reports.clipped_hint", &[]);
        assert!(
            has(&pass(&one, vec![]), &hint),
            "the click opens the section"
        );
        let two = report_with(2);
        pass(&two, vec![]);
        assert!(has(&pass(&two, vec![]), &hint), "a new count keeps it open");
        crate::i18n::with_language(crate::i18n::Language::German, || {
            let hint = message("reports.clipped_hint", &[]);
            pass(&two, vec![]);
            assert!(
                has(&pass(&two, vec![]), &hint),
                "a new language keeps it open"
            );
        });
    }

    #[test]
    fn reports_fit_the_narrowest_window_in_every_language() {
        let overflow = std::cell::RefCell::new(Vec::new());
        for language in crate::i18n::Language::ALL {
            crate::i18n::with_language(language, || {
                let context = egui::Context::default();
                crate::fonts::Fallbacks::new(language).apply(&context);
                let mut panel = fixed();
                let (begin, _) = panel.request.bounds().unwrap();
                let record = |id, state| reports::Record {
                    id,
                    start_ms: begin + id * 3_600_000,
                    end_ms: begin + (id + 1) * 3_600_000,
                    elapsed_ms: 3_600_000,
                    state,
                    activity: tracking::Activity::default(),
                    reason: "stop".into(),
                };
                panel.report = Some(reports::compute(
                    reports::Range::new(&panel.request).unwrap(),
                    vec![
                        record(1, reports::State::Saved),
                        record(2, reports::State::Review),
                    ],
                    begin,
                ));
                let (sender, _receiver) = mpsc::sync_channel(8);
                let header = std::cell::Cell::new(egui::Rect::NOTHING);
                let pass = |panel: &mut Panel, step: i32, events| {
                    let mut output = context.run_ui(
                        egui::RawInput {
                            time: Some(f64::from(step) * 0.2),
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(380.0, 1600.0),
                            )),
                            events,
                            ..Default::default()
                        },
                        |ui| {
                            let right = ui.max_rect().right();
                            panel.show(ui, &sender, true);
                            // A failure inside a frame would abort the whole test process.
                            if ui.max_rect().right() > right + 0.5 {
                                let excess = ui.max_rect().right() - right;
                                let mut list = overflow.borrow_mut();
                                if !list.iter().any(|(known, _)| *known == language) {
                                    list.push((language, excess));
                                }
                            }
                        },
                    );
                    output.textures_delta.clear();
                    for clipped in &output.shapes {
                        if let egui::Shape::Text(text) = &clipped.shape
                            && text.galley.job.text == crate::i18n::t("reports.title")
                        {
                            header.set(text.visual_bounding_rect());
                        }
                    }
                };
                pass(&mut panel, 0, vec![]);
                pass(&mut panel, 1, vec![]);
                let at = header.get().center();
                pass(&mut panel, 2, vec![egui::Event::PointerMoved(at)]);
                pass(
                    &mut panel,
                    3,
                    vec![
                        egui::Event::PointerButton {
                            pos: at,
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers: egui::Modifiers::default(),
                        },
                        egui::Event::PointerButton {
                            pos: at,
                            button: egui::PointerButton::Primary,
                            pressed: false,
                            modifiers: egui::Modifiers::default(),
                        },
                    ],
                );
                for step in 4..12 {
                    pass(&mut panel, step, vec![]);
                }
            });
        }
        assert!(overflow.borrow().is_empty(), "{:?}", overflow.borrow());
    }

    #[test]
    fn reports_on_screen_include_the_charts_and_keep_refreshing_them() {
        let context = egui::Context::default();
        let mut panel = fixed();
        let (begin, _) = panel.request.bounds().unwrap();
        let records = vec![reports::Record {
            id: 1,
            start_ms: begin + 3_600_000,
            end_ms: begin + 7_200_000,
            elapsed_ms: 3_600_000,
            state: reports::State::Saved,
            activity: tracking::Activity::default(),
            reason: "stop".into(),
        }];
        panel.report = Some(reports::compute(
            reports::Range::new(&panel.request).unwrap(),
            records,
            begin,
        ));
        let (sender, _receiver) = mpsc::sync_channel(8);
        let frame = |panel: &mut Panel, events| {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(700.0, 1200.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| panel.show(ui, &sender, true),
            );
            output.textures_delta.clear();
            output
        };
        let texts = |output: &egui::FullOutput| -> Vec<(String, egui::Pos2)> {
            output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Text(text) => Some((text.galley.job.text.clone(), text.pos)),
                    _ => None,
                })
                .collect()
        };
        frame(&mut panel, vec![]);
        let output = frame(&mut panel, vec![]);
        let header = texts(&output)
            .into_iter()
            .find(|(text, _)| text == "Focus history and reports")
            .unwrap()
            .1
            + egui::vec2(3.0, 3.0);
        let press = |pos, pressed| {
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ]
        };
        frame(&mut panel, press(header, true));
        frame(&mut panel, press(header, false));
        let mut output = frame(&mut panel, vec![]);
        for _ in 0..30 {
            output = frame(&mut panel, vec![]);
        }
        let shown = texts(&output);
        let has = |label: &str| shown.iter().any(|(text, _)| text == label);
        assert!(has("Charts") && has("Timeline") && has("By day") && has("Weekly rhythm"));
        assert!(has("Thu 1 Oct"), "the row for 1 October 2026");
        assert!(has("Unassigned focus"), "the legend names the project");
    }
    #[test]
    fn renaming_a_project_refreshes_the_report_even_while_stopped() {
        let mut panel = fixed();
        let (_, end) = panel.request.bounds().unwrap();
        let (sender, receiver) = mpsc::sync_channel(8);
        let now = Instant::now();
        let state = |title: &str| tracking::Snapshot {
            projects: vec![tracking::CatalogProject {
                id: 1,
                title: title.into(),
                archived: false,
                used: true,
                tasks: Vec::new(),
            }],
            ..tracking::Snapshot::default()
        };
        panel.tracking_changed(&state("Old"), false, end + 1000);
        panel.tick_at(&sender, true, now, end + 1000);
        receiver.try_recv().unwrap();
        panel.receive(panel.request_id, Ok(response(&panel.request)));
        panel.tracking_changed(&state("Old"), true, end + 2000);
        panel.tick_at(&sender, true, now + REFRESH_INTERVAL, end + 2000);
        assert!(
            receiver.try_recv().is_err(),
            "unchanged names do not reload"
        );
        panel.tracking_changed(&state("New"), true, end + 3000);
        panel.tick_at(&sender, true, now + REFRESH_INTERVAL * 2, end + 3000);
        assert!(receiver.try_recv().is_ok());
    }
    #[test]
    fn period_hint_states_whether_dates_follow_the_current_day() {
        assert!(period_hint(Some(1), "UTC").contains("new date at midnight (UTC)"));
        assert!(period_hint(Some(7), "UTC").contains("last 7 days"));
        assert!(period_hint(None, "UTC").contains("Fixed dates"));
    }

    fn fixed() -> Panel {
        Panel {
            request: reports::Request {
                from: "2026-10-01".into(),
                through: "2026-10-02".into(),
                timezone: "UTC".into(),
                from_time: "00:00".into(),
                through_time: "24:00".into(),
            },
            preset: None,
            ..Panel::default()
        }
    }
    fn response(request: &reports::Request) -> reports::Report {
        reports::compute(reports::Range::new(request).unwrap(), Vec::new(), 1_000_000)
    }
    #[test]
    fn only_active_tracking_inside_current_filter_is_polled_periodically() {
        for active in [false, true] {
            for location in [-1, 0, 1] {
                let mut panel = fixed();
                let (begin, end) = panel.request.bounds().unwrap();
                let wall = match location {
                    -1 => begin - 1000,
                    0 => begin + 1000,
                    _ => end + 1000,
                };
                panel.active_start = active.then_some(wall - 1000);
                let (sender, receiver) = mpsc::sync_channel(8);
                let now = Instant::now();
                panel.tick_at(&sender, true, now, wall);
                receiver.try_recv().unwrap();
                panel.receive(panel.request_id, Ok(response(&panel.request)));
                panel.tick_at(&sender, true, now + REFRESH_INTERVAL, wall);
                assert_eq!(receiver.try_recv().is_ok(), active && location == 0);
            }
        }
    }
    #[test]
    fn leaving_filter_gets_one_final_snapshot_then_stop_reclassifies_overlap_once() {
        let mut panel = fixed();
        panel.request.through = panel.request.from.clone();
        panel.request.through_time = "00:02".into();
        let (begin, end) = panel.request.bounds().unwrap();
        panel.active_start = Some(begin);
        let (sender, receiver) = mpsc::sync_channel(8);
        let now = Instant::now();
        panel.tick_at(&sender, true, now, end - 1000);
        receiver.try_recv().unwrap();
        panel.receive(panel.request_id, Ok(response(&panel.request)));
        panel.tick_at(&sender, true, now + REFRESH_INTERVAL, end);
        receiver.try_recv().unwrap();
        panel.receive(panel.request_id, Ok(response(&panel.request)));
        panel.tick_at(&sender, true, now + REFRESH_INTERVAL * 2, end + 1000);
        assert!(receiver.try_recv().is_err());
        panel.tracking_changed(&tracking::Snapshot::default(), true, end + 10_000);
        panel.tick_at(&sender, true, now + REFRESH_INTERVAL * 3, end + 10_000);
        receiver.try_recv().unwrap();
        panel.receive(panel.request_id, Ok(response(&panel.request)));
        panel.tick_at(&sender, true, now + REFRESH_INTERVAL * 4, end + 20_000);
        assert!(receiver.try_recv().is_err());
    }
    #[test]
    fn unrelated_commits_do_not_reload_history_but_recovery_acceptance_does() {
        let mut panel = fixed();
        let (begin, end) = panel.request.bounds().unwrap();
        let now = Instant::now();
        let (sender, receiver) = mpsc::sync_channel(8);
        panel.tick_at(&sender, true, now, end + 1000);
        receiver.try_recv().unwrap();
        panel.receive(panel.request_id, Ok(response(&panel.request)));
        panel.tracking_changed(
            &tracking::Snapshot {
                running: true,
                active_start_ms: Some(end + 500),
                ..tracking::Snapshot::default()
            },
            true,
            end + 1000,
        );
        panel.tick_at(&sender, true, now + REFRESH_INTERVAL, end + 2000);
        assert!(receiver.try_recv().is_err());
        panel.tracking_changed(&tracking::Snapshot::default(), true, end + 3000);
        panel.tick_at(&sender, true, now + REFRESH_INTERVAL * 2, end + 4000);
        assert!(receiver.try_recv().is_err());
        panel.report = Some(reports::compute(
            reports::Range::new(&panel.request).unwrap(),
            vec![reports::Record {
                id: 1,
                start_ms: begin,
                end_ms: begin + 1000,
                elapsed_ms: 1000,
                state: reports::State::Review,
                activity: tracking::Activity::default(),
                reason: "crash".into(),
            }],
            end,
        ));
        panel.needs_review = true;
        panel.tracking_changed(&tracking::Snapshot::default(), true, end + 5000);
        panel.tick_at(&sender, true, now + REFRESH_INTERVAL * 3, end + 5000);
        assert!(receiver.try_recv().is_ok());
        panel.receive(panel.request_id, Ok(response(&panel.request)));
        panel.report = Some(reports::compute(
            reports::Range::new(&panel.request).unwrap(),
            vec![reports::Record {
                id: 2,
                start_ms: begin,
                end_ms: begin + 1000,
                elapsed_ms: 1000,
                state: reports::State::Running,
                activity: tracking::Activity::default(),
                reason: String::new(),
            }],
            begin + 1000,
        ));
        panel.needs_review = false;
        panel.active_start = Some(begin);
        panel.tracking_changed(
            &tracking::Snapshot {
                needs_review: true,
                ..tracking::Snapshot::default()
            },
            false,
            begin - 1000,
        );
        panel.tick_at(&sender, true, now + REFRESH_INTERVAL * 4, begin - 1000);
        assert!(receiver.try_recv().is_ok());
    }
    #[test]
    fn auto_refresh_stops_when_closed_bounds_inflight_and_obeys_interval() {
        let mut panel = fixed();
        let wall = panel.request.bounds().unwrap().0 + 1000;
        panel.active_start = Some(wall - 1000);
        let (sender, receiver) = mpsc::sync_channel(8);
        let now = Instant::now();
        panel.tick_at(&sender, false, now, wall);
        assert!(receiver.try_recv().is_err());
        panel.tick_at(&sender, true, now, wall);
        assert!(matches!(
            receiver.try_recv().unwrap(),
            tracking::Command::Report { .. }
        ));
        panel.tick_at(&sender, true, now + Duration::from_secs(10), wall);
        assert!(receiver.try_recv().is_err());
        panel.receive(panel.request_id, Ok(response(&panel.request)));
        panel.tick_at(&sender, true, now + Duration::from_millis(1500), wall);
        assert!(receiver.try_recv().is_err());
        panel.tick_at(&sender, true, now + REFRESH_INTERVAL, wall);
        assert!(matches!(
            receiver.try_recv().unwrap(),
            tracking::Command::Report { .. }
        ));
    }
    #[test]
    fn edited_filters_reject_stale_replies_and_debounce_without_losing_commit_invalidation() {
        let mut panel = fixed();
        let wall = panel.request.bounds().unwrap().0 + 1000;
        panel.active_start = Some(wall - 1000);
        let (sender, receiver) = mpsc::sync_channel(8);
        let now = Instant::now();
        panel.tick_at(&sender, true, now, wall);
        receiver.try_recv().unwrap();
        let old = response(&panel.request);
        panel.request.through = "2026-10-03".into();
        panel.edited(now);
        panel.receive(panel.request_id, Ok(old));
        assert!(panel.report.is_none());
        panel.tick_at(&sender, true, now + Duration::from_millis(349), wall);
        assert!(receiver.try_recv().is_err());
        panel.tick_at(&sender, true, now + Duration::from_millis(350), wall);
        receiver.try_recv().unwrap();
        let id = panel.request_id;
        panel.receive(id - 1, Err("Stale failure".into()));
        assert!(panel.pending);
        panel.dirty = true;
        panel.receive(id, Ok(response(&panel.request)));
        panel.tick_at(&sender, true, now + Duration::from_millis(351), wall);
        assert!(matches!(
            receiver.try_recv().unwrap(),
            tracking::Command::Report { .. }
        ));
        assert!(panel.error.is_none());
    }
    #[test]
    fn report_errors_keep_last_good_result_and_back_off_after_full_queue() {
        let mut panel = fixed();
        let wall = panel.request.bounds().unwrap().0 + 1000;
        panel.active_start = Some(wall - 1000);
        let (sender, receiver) = mpsc::sync_channel(1);
        let now = Instant::now();
        panel.report = Some(response(&panel.request));
        panel.tick_at(&sender, true, now, wall);
        receiver.try_recv().unwrap();
        panel.receive(panel.request_id, Err("Synthetic read failure".into()));
        assert!(panel.report.is_some());
        assert!(panel.error.is_some());
        sender.try_send(tracking::Command::Stop).unwrap();
        panel.dirty = true;
        panel.tick_at(&sender, true, now, wall);
        assert!(!panel.pending);
        receiver.try_recv().unwrap();
        panel.tick_at(&sender, true, now + Duration::from_secs(1), wall);
        assert!(receiver.try_recv().is_err());
        panel.tick_at(&sender, true, now + REFRESH_INTERVAL, wall);
        assert!(matches!(
            receiver.try_recv().unwrap(),
            tracking::Command::Report { .. }
        ));
    }
    #[test]
    fn timezone_choices_include_utc_and_existing_timezone_rules() {
        let panel = Panel::default();
        assert_eq!(panel.zones.first().unwrap(), "UTC");
        assert!(panel.zones.iter().any(|zone| zone == "Europe/Istanbul"));
        assert!(
            panel
                .zones
                .iter()
                .all(|zone| jiff::tz::TimeZone::get(zone).is_ok())
        );
    }
}
