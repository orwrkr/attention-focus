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
    active_start: Option<i64>,
    needs_review: bool,
    growing: bool,
    was_open: bool,
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
            active_start: None,
            needs_review: false,
            growing: false,
            was_open: false,
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
    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(
            storage,
            "report_filter_v1",
            &(self.request.clone(), self.preset),
        );
        eframe::set_value(storage, "timezone_search", &self.zone_search);
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
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }
    pub fn tracking_changed(&mut self, state: &tracking::Snapshot, committed: bool, wall_ms: i64) {
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
    fn edited(&mut self, now: Instant) {
        self.dirty = true;
        self.debounce = Some(now + Duration::from_millis(350));
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
        if self.debounce.is_some_and(|deadline| now < deadline) {
            return;
        }
        self.debounce = None;
        if self.pending {
            return;
        }
        if let Some(days) = self.preset
            && let Ok(request) = reports::Request::recent(days, &self.request.timezone)
            && request != self.request
        {
            self.request = request;
            self.dirty = true;
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
                        ui.label("From (included)");
                        dates_changed |=
                            self.from_calendar.show(ui, &mut self.request.from, "from");
                        ui.end_row();
                        ui.label("From time (HH:MM)");
                        dates_changed |= ui.add(egui::TextEdit::singleline(&mut self.request.from_time).desired_width(70.0)).changed();
                        ui.end_row();
                        ui.label("Until (excluded)");
                        dates_changed |=
                            self.through_calendar
                                .show(ui, &mut self.request.through, "through");
                        ui.end_row();
                        ui.label("Until time (HH:MM)");
                        dates_changed |= ui.add(egui::TextEdit::singleline(&mut self.request.through_time).desired_width(70.0)).on_hover_text("24:00 means the end of the selected date. The Until boundary is excluded.").changed();
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
            ui.weak(if self.pending {"Updating report..."} else if self.growing {"Auto-refresh: tracking is inside this range"} else {"Auto-refresh paused: no tracking time is growing in this range"});
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
    fn results(ui: &mut egui::Ui, report: &reports::Report) {
        let request = &report.range.request;
        ui.label(format!(
            "Results: {} {} to {} {} (end excluded) | {}",
            request.from,
            request.from_time,
            request.through,
            request.through_time,
            request.timezone
        ));
        ui.weak(format!(
            "Snapshot: {}",
            report.range.format_time(report.generated_ms)
        ));
        ui.strong(format!("Saved focus: {}", duration(report.saved_ms)));
        ui.label(format!("Running (not finalized): {}",duration(report.running_ms))).on_hover_text("Elapsed time from the active interval clipped to this filter. It is not included in Saved focus until the interval ends.");
        ui.label(format!("Needs review (excluded): {}",duration(report.review_ms))).on_hover_text("Saved checkpoint time from interrupted intervals within this filter. It is excluded from Saved focus until you accept recovery.");
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
        panel.invalidate();
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
        panel.invalidate();
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
