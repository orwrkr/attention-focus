//! Report controls; database access remains on the tracking worker.
use crate::{reports, tracking};
use eframe::egui;
use std::sync::mpsc::SyncSender;

#[derive(Default)]
pub struct Panel {
    request: reports::Request,
    report: Option<reports::Report>,
    pending: bool,
    request_id: u64,
    error: Option<String>,
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
        match result {
            Ok(report) => {
                self.report = Some(report);
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }
    fn refresh(&mut self, commands: &SyncSender<tracking::Command>) {
        self.request_id = self.request_id.wrapping_add(1);
        self.error = None;
        match commands.try_send(tracking::Command::Report {
            id: self.request_id,
            request: self.request.clone(),
        }) {
            Ok(()) => self.pending = true,
            Err(error) => self.error = Some(format!("Cannot request report: {error}")),
        }
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        commands: &SyncSender<tracking::Command>,
        ready: bool,
    ) {
        ui.collapsing("Focus history and reports", |ui| {
            ui.add_enabled_ui(!self.pending && ready, |ui| {
                egui::Grid::new("report-range")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("From (YYYY-MM-DD)");
                        ui.text_edit_singleline(&mut self.request.from);
                        ui.end_row();
                        ui.label("Through (inclusive)");
                        ui.text_edit_singleline(&mut self.request.through);
                        ui.end_row();
                        ui.label("Time zone");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.request.timezone)
                                .hint_text("Europe/Istanbul or UTC"),
                        );
                        ui.end_row();
                    });
                ui.horizontal(|ui| {
                    for (label, days) in [("Today", 1), ("Last 7 days", 7), ("Last 30 days", 30)] {
                        if ui.button(label).clicked() {
                            match reports::Request::recent(days, &self.request.timezone) {
                                Ok(request) => {
                                    self.request = request;
                                    self.refresh(commands);
                                }
                                Err(error) => {
                                    self.error = Some(format!("Cannot set dates: {error}"))
                                }
                            }
                        }
                    }
                    if ui.button("Refresh report").clicked() {
                        self.refresh(commands);
                    }
                });
            });
            if !ready {
                ui.label("History is not available yet.");
            }
            if self.pending {
                ui.label("Loading report...");
            }
            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
            if let Some(report) = &self.report {
                Self::results(ui, report);
            } else {
                ui.label("Choose a period and refresh the report.");
            }
        });
    }
    fn results(ui: &mut egui::Ui, report: &reports::Report) {
        let request = &report.range.request;
        ui.label(format!(
            "Results: {} through {} | {}",
            request.from, request.through, request.timezone
        ));
        ui.label(format!(
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
