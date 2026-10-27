//! Report controls; database access remains on the tracking worker.
use crate::{date_picker::Calendar, reports, tracking};
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
    from_calendar: Calendar,
    through_calendar: Calendar,
    zones: Vec<String>,
    zone_search: String,
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
            from_calendar: Calendar::default(),
            through_calendar: Calendar::default(),
            zones,
            zone_search: String::new(),
        }
    }
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
impl Panel {
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
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }
    fn edited(&mut self, now: Instant) {
        self.dirty = true;
        self.debounce = Some(now + Duration::from_millis(350));
    }
    fn tick(&mut self, commands: &SyncSender<tracking::Command>, open: bool, now: Instant) {
        if !open || self.debounce.is_some_and(|deadline| now < deadline) {
            return;
        }
        self.debounce = None;
        if self.pending {
            return;
        }
        if !self.dirty
            && self
                .last_attempt
                .is_some_and(|last| now.saturating_duration_since(last) < REFRESH_INTERVAL)
        {
            return;
        }
        if let Some(days) = self.preset
            && let Ok(request) = reports::Request::recent(days, &self.request.timezone)
        {
            self.request = request;
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
            Err(error) => self.error = Some(format!("Cannot request report: {error}")),
        }
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        commands: &SyncSender<tracking::Command>,
        ready: bool,
    ) {
        let now = Instant::now();
        let response = ui.collapsing("Focus history and reports", |ui| {
            ui.add_enabled_ui(ready, |ui| {
                let mut dates_changed = false;
                let old_zone = self.request.timezone.clone();
                egui::Grid::new("report-range")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("From (YYYY-MM-DD)");
                        dates_changed |=
                            self.from_calendar.show(ui, &mut self.request.from, "from");
                        ui.end_row();
                        ui.label("Through (inclusive)");
                        dates_changed |=
                            self.through_calendar
                                .show(ui, &mut self.request.through, "through");
                        ui.end_row();
                        ui.label("Time zone");
                        egui::ComboBox::from_id_salt("report-timezone")
                            .selected_text(&self.request.timezone)
                            .width(235.0)
                            .show_ui(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.zone_search)
                                        .hint_text("Search time zones"),
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
                                    ui.label("No matching time zones.");
                                }
                            });
                        ui.end_row();
                    });
                if dates_changed {
                    self.preset = None;
                    self.edited(now);
                }
                if old_zone != self.request.timezone {
                    self.edited(now);
                }
                ui.horizontal(|ui| {
                    for (label, days) in [("Today", 1), ("Last 7 days", 7), ("Last 30 days", 30)] {
                        if ui.button(label).clicked() {
                            match reports::Request::recent(days, &self.request.timezone) {
                                Ok(request) => {
                                    self.request = request;
                                    self.preset = Some(days);
                                    self.debounce = None;
                                    self.dirty = true;
                                }
                                Err(error) => {
                                    self.error = Some(format!("Cannot set dates: {error}"))
                                }
                            }
                        }
                    }
                    if ui.button("Refresh report").clicked() {
                        self.debounce = None;
                        self.dirty = true;
                    }
                });
            });
            if !ready {
                ui.label("History is not available yet.");
            }
            if self.pending {
                ui.weak("Updating report...");
            }
            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
            if let Some(report) = &self.report {
                Self::results(ui, report);
            } else {
                ui.label("The report loads automatically when history is ready.");
            }
        });
        let open = response.body_returned.is_some() && ui.is_visible();
        self.tick(commands, ready && open, now);
        if open && ready {
            ui.ctx()
                .request_repaint_after(self.debounce.map_or(REFRESH_INTERVAL, |deadline| {
                    deadline.saturating_duration_since(now)
                }));
        }
    }
    fn results(ui: &mut egui::Ui, report: &reports::Report) {
        let request = &report.range.request;
        ui.label(format!(
            "Results: {} through {} | {}",
            request.from, request.through, request.timezone
        ));
        ui.weak(format!(
            "Snapshot: {}",
            report.range.format_time(report.generated_ms)
        ));
        ui.strong(format!("Saved focus: {}", duration(report.saved_ms)));
        ui.label(format!(
            "Running (not finalized): {} | Needs review (excluded): {}",
            duration(report.running_ms),
            duration(report.review_ms)
        ));
        for diagnostic in &report.diagnostics {
            ui.colored_label(egui::Color32::YELLOW, diagnostic);
        }
        ui.collapsing("By project", |ui| {
            egui::Grid::new("project-totals")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Project");
                    ui.strong("Saved time");
                    ui.end_row();
                    for group in &report.projects {
                        ui.label(&group.project)
                            .on_hover_text(group.source.as_deref().unwrap_or("Unassigned"));
                        ui.label(duration(group.saved_ms));
                        ui.end_row();
                    }
                });
        });
        ui.collapsing("By project and task", |ui| {
            egui::Grid::new("task-totals").striped(true).show(ui, |ui| {
                ui.strong("Project");
                ui.strong("Task");
                ui.strong("Saved time");
                ui.end_row();
                for group in &report.groups {
                    ui.label(&group.project)
                        .on_hover_text(group.source.as_deref().unwrap_or("Unassigned"));
                    ui.label(group.task.as_deref().unwrap_or("No task"));
                    ui.label(duration(group.saved_ms));
                    ui.end_row();
                }
            });
        });
        ui.collapsing("By day", |ui| {
            egui::ScrollArea::vertical()
                .id_salt("daily-focus")
                .max_height(200.0)
                .show(ui, |ui| {
                    egui::Grid::new("daily-totals")
                        .striped(true)
                        .show(ui, |ui| {
                            ui.strong("Day");
                            ui.strong("Saved time");
                            ui.end_row();
                            for day in &report.range.days {
                                ui.label(&day.date);
                                ui.label(duration(day.saved_ms));
                                ui.end_row();
                            }
                        });
                });
        });
        ui.collapsing(format!("Sessions ({})", report.matching_sessions), |ui| {
            if report.matching_sessions > report.history.len() {
                ui.label(
                    "Showing the 200 most recent sessions. Totals include all matching sessions.",
                );
            }
            ui.label("Session time below is clipped to the selected period.");
            egui::ScrollArea::vertical()
                .id_salt("session-history")
                .max_height(320.0)
                .show(ui, |ui| {
                    for row in &report.history {
                        let record = &row.record;
                        let status = match record.state {
                            reports::State::Saved => "Saved",
                            reports::State::Running => "Running",
                            reports::State::Review => "Needs review",
                        };
                        ui.push_id(record.id, |ui| {
                            ui.collapsing(
                                format!(
                                    "{} | {} | {}",
                                    report.range.format_time(record.start_ms),
                                    duration(row.clipped_ms),
                                    status
                                ),
                                |ui| {
                                    ui.strong(record.activity.label());
                                    if let Some(project) = &record.activity.project
                                        && let Some(task) = &project.task
                                    {
                                        ui.label(task);
                                    }
                                    if !record.activity.note.is_empty() {
                                        ui.label(&record.activity.note);
                                    }
                                    ui.label(format!(
                                        "Start: {}",
                                        report.range.format_time(record.start_ms)
                                    ));
                                    ui.label(format!(
                                        "End / snapshot: {}",
                                        report.range.format_time(record.end_ms)
                                    ));
                                    ui.label(format!(
                                        "Full recorded time: {}",
                                        duration(record.elapsed_ms)
                                    ));
                                    let reason = match record.reason.as_str() {
                                        "stop" => "Stopped",
                                        "switch" => "Focus changed",
                                        "exit" => "Application exit",
                                        "lock" => "Windows locked",
                                        "suspend" => "Sleep",
                                        "crash" => "Interrupted before restart",
                                        "clock_or_heartbeat" => "Clock change or missed heartbeat",
                                        _ => "Tracking interrupted",
                                    };
                                    if record.state != reports::State::Running {
                                        ui.label(format!("Ended: {reason}"));
                                    }
                                },
                            );
                        });
                    }
                });
        });
        if report.matching_sessions == 0 {
            ui.label("No sessions in this period.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn fixed() -> Panel {
        Panel {
            request: reports::Request {
                from: "2026-10-01".into(),
                through: "2026-10-02".into(),
                timezone: "UTC".into(),
            },
            preset: None,
            ..Panel::default()
        }
    }
    fn response(request: &reports::Request) -> reports::Report {
        reports::compute(reports::Range::new(request).unwrap(), Vec::new(), 1_000_000)
    }
    #[test]
    fn auto_refresh_stops_when_closed_bounds_inflight_and_obeys_interval() {
        let mut panel = fixed();
        let (sender, receiver) = mpsc::sync_channel(8);
        let now = Instant::now();
        panel.tick(&sender, false, now);
        assert!(receiver.try_recv().is_err());
        panel.tick(&sender, true, now);
        assert!(matches!(
            receiver.try_recv().unwrap(),
            tracking::Command::Report { .. }
        ));
        panel.tick(&sender, true, now + Duration::from_secs(10));
        assert!(receiver.try_recv().is_err());
        panel.receive(panel.request_id, Ok(response(&panel.request)));
        panel.tick(&sender, true, now + Duration::from_millis(1500));
        assert!(receiver.try_recv().is_err());
        panel.tick(&sender, true, now + REFRESH_INTERVAL);
        assert!(matches!(
            receiver.try_recv().unwrap(),
            tracking::Command::Report { .. }
        ));
    }
    #[test]
    fn edited_filters_reject_stale_replies_and_debounce_without_losing_commit_invalidation() {
        let mut panel = fixed();
        let (sender, receiver) = mpsc::sync_channel(8);
        let now = Instant::now();
        panel.tick(&sender, true, now);
        receiver.try_recv().unwrap();
        let old = response(&panel.request);
        panel.request.through = "2026-10-03".into();
        panel.edited(now);
        panel.receive(panel.request_id, Ok(old));
        assert!(panel.report.is_none());
        panel.tick(&sender, true, now + Duration::from_millis(349));
        assert!(receiver.try_recv().is_err());
        panel.tick(&sender, true, now + Duration::from_millis(350));
        receiver.try_recv().unwrap();
        let id = panel.request_id;
        panel.receive(id - 1, Err("Stale failure".into()));
        assert!(panel.pending);
        panel.invalidate();
        panel.receive(id, Ok(response(&panel.request)));
        panel.tick(&sender, true, now + Duration::from_millis(351));
        assert!(matches!(
            receiver.try_recv().unwrap(),
            tracking::Command::Report { .. }
        ));
        assert!(panel.error.is_none());
    }
    #[test]
    fn report_errors_keep_last_good_result_and_back_off_after_full_queue() {
        let mut panel = fixed();
        let (sender, receiver) = mpsc::sync_channel(1);
        let now = Instant::now();
        panel.report = Some(response(&panel.request));
        panel.tick(&sender, true, now);
        receiver.try_recv().unwrap();
        panel.receive(panel.request_id, Err("Synthetic read failure".into()));
        assert!(panel.report.is_some());
        assert!(panel.error.is_some());
        sender.try_send(tracking::Command::Stop).unwrap();
        panel.invalidate();
        panel.tick(&sender, true, now);
        assert!(!panel.pending);
        receiver.try_recv().unwrap();
        panel.tick(&sender, true, now + Duration::from_secs(1));
        assert!(receiver.try_recv().is_err());
        panel.tick(&sender, true, now + REFRESH_INTERVAL);
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
