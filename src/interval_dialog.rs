//! Drafts preserve untouched millisecond boundaries; the worker owns validation and writes.
use crate::{
    date_picker::Calendar,
    i18n::{message, paragraph, paragraph_colored, t, tf, user},
    presentation::duration,
    reports::{self, Record, State},
    time_picker::TimePicker,
    tracking::{Activity, CatalogProject, Command, IntervalEdit, Project},
};
use eframe::egui;
use jiff::{Timestamp, tz::TimeZone};

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Edit(i64),
    Delete(i64),
    Add(Option<(usize, f32, f32)>),
    AddAfter(i64),
}

pub fn locked_reason(state: State) -> Option<&'static str> {
    match state {
        State::Saved => None,
        State::Running => Some("reports.interval_running_locked"),
        State::Review => Some("reports.interval_review_locked"),
    }
}
/// The same choices for a timeline mark and a session header.
pub fn menu(ui: &mut egui::Ui, interval: Option<(i64, State)>, add: Action) -> Option<Action> {
    let mut action = None;
    if let Some((id, state)) = interval {
        let reason = locked_reason(state);
        if ui
            .add_enabled(
                reason.is_none(),
                egui::Button::new(t("reports.interval_edit")),
            )
            .clicked()
        {
            action = Some(Action::Edit(id));
        }
        if ui
            .add_enabled(
                reason.is_none(),
                egui::Button::new(t("reports.interval_delete")),
            )
            .clicked()
        {
            action = Some(Action::Delete(id));
        }
        if let Some(reason) = reason {
            paragraph_colored(ui, ui.visuals().weak_text_color(), &message(reason, &[]));
        }
        ui.separator();
    }
    if ui.button(t("reports.interval_add")).clicked() {
        action = Some(add);
    }
    if action.is_some() {
        ui.close();
    }
    action
}
pub fn buttons(ui: &mut egui::Ui, id: i64, state: State) -> Option<Action> {
    let mut action = None;
    let reason = locked_reason(state);
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(
                reason.is_none(),
                egui::Button::new(t("reports.interval_edit")),
            )
            .clicked()
        {
            action = Some(Action::Edit(id));
        }
        if ui
            .add_enabled(
                reason.is_none(),
                egui::Button::new(t("reports.interval_delete")),
            )
            .clicked()
        {
            action = Some(Action::Delete(id));
        }
    });
    if let Some(reason) = reason {
        paragraph_colored(ui, ui.visuals().weak_text_color(), &message(reason, &[]));
    }
    action
}

struct Boundary {
    date: String,
    time: String,
    initial_date: String,
    initial_time: String,
    original_ms: Option<i64>,
    calendar: Calendar,
    picker: TimePicker,
}
impl Boundary {
    fn local(date: &str, time: &str) -> Self {
        Self {
            date: date.into(),
            time: time.into(),
            initial_date: date.into(),
            initial_time: time.into(),
            original_ms: None,
            calendar: Calendar::default(),
            picker: TimePicker::default(),
        }
    }
    fn new(ms: i64, timezone: &str) -> Self {
        let zone = TimeZone::get(timezone).unwrap_or_else(|_| TimeZone::system());
        let local = Timestamp::from_millisecond(ms)
            .expect("Report timestamp is valid")
            .to_zoned(zone);
        let date = local.date().to_string();
        let time = if local.second() == 0 && local.millisecond() == 0 {
            local.strftime("%H:%M").to_string()
        } else {
            local.strftime("%H:%M:%S").to_string()
        };
        Self {
            initial_date: date.clone(),
            initial_time: time.clone(),
            date,
            time,
            original_ms: Some(ms),
            calendar: Calendar::default(),
            picker: TimePicker::default(),
        }
    }
    fn resolve(&self, end: bool, timezone: &str) -> reports::Result<i64> {
        if self.date == self.initial_date
            && self.time == self.initial_time
            && let Some(ms) = self.original_ms
        {
            Ok(ms)
        } else {
            reports::local_moment(&self.date, &self.time, end, timezone)
        }
    }
    fn show(&mut self, ui: &mut egui::Ui, id: &str, end: bool, timezone: &str) {
        ui.push_id(id, |ui| {
            self.calendar.show(ui, &mut self.date, "date", timezone);
            self.picker.show(ui, &mut self.time, "time", end);
        });
    }
}

pub struct Dialog {
    pub(crate) original: Option<Record>,
    start: Boundary,
    end: Boundary,
    project: Option<i64>,
    task: Option<String>,
    note: String,
    timezone: String,
    pub(crate) error: Option<String>,
    pub(crate) saving: bool,
}
pub enum Outcome {
    None,
    Cancel,
    Send(Command),
}
impl Dialog {
    pub fn edit(record: Record, timezone: &str) -> Self {
        let mut draft = Self::new(
            Boundary::new(record.start_ms, timezone),
            Boundary::new(record.end_ms, timezone),
            timezone,
        );
        draft.project = record.activity.project.as_ref().map(|p| p.id);
        draft.task = record
            .activity
            .project
            .as_ref()
            .and_then(|p| p.task.clone());
        draft.note = record.activity.note.clone();
        draft.original = Some(record);
        draft
    }
    pub fn add(start: i64, end: i64, timezone: &str) -> Self {
        let mut start = Boundary::new(start, timezone);
        let mut end = Boundary::new(end, timezone);
        start.original_ms = None;
        end.original_ms = None;
        Self::new(start, end, timezone)
    }
    fn new(start: Boundary, end: Boundary, timezone: &str) -> Self {
        Self {
            original: None,
            start,
            end,
            project: None,
            task: None,
            note: String::new(),
            timezone: timezone.into(),
            error: None,
            saving: false,
        }
    }
    /// New civil drafts can start in an invalid local hour; keep the text for correction.
    pub fn add_local(date: &str, start: &str, end: &str, timezone: &str) -> Self {
        Self::new(
            Boundary::local(date, start),
            Boundary::local(date, end),
            timezone,
        )
    }
    fn resolve(&self, catalog: &[CatalogProject]) -> reports::Result<IntervalEdit> {
        let start_ms = self.start.resolve(false, &self.timezone)?;
        let end_ms = self.end.resolve(true, &self.timezone)?;
        if end_ms <= start_ms {
            return Err(std::io::Error::other(message("errors.interval_order", &[])).into());
        }
        let recorded = self
            .original
            .as_ref()
            .and_then(|r| r.activity.project.as_ref());
        let project = self.project.map(|id| Project {
            id,
            title: recorded
                .filter(|p| p.id == id)
                .map(|p| p.title.clone())
                .or_else(|| catalog.iter().find(|p| p.id == id).map(|p| p.title.clone()))
                .unwrap_or_default(),
            task: self.task.clone(),
        });
        Ok(IntervalEdit {
            id: self.original.as_ref().map(|r| r.id),
            recorded: self.original.as_ref().map(|r| (r.start_ms, r.end_ms)),
            start_ms,
            end_ms,
            activity: Activity {
                project,
                note: self.note.clone(),
            },
            timezone: self.timezone.clone(),
        })
    }
    fn project_picker(&mut self, ui: &mut egui::Ui, catalog: &[CatalogProject]) {
        ui.label(t("reports.project"));
        let recorded = self
            .original
            .as_ref()
            .and_then(|r| r.activity.project.as_ref());
        let selected = recorded
            .filter(|p| Some(p.id) == self.project)
            .map(|p| user(&p.title).into_owned())
            .or_else(|| {
                catalog
                    .iter()
                    .find(|p| Some(p.id) == self.project)
                    .map(|p| user(&p.title).into_owned())
            })
            .unwrap_or_else(|| t("activity.unassigned").into());
        let old = self.project;
        let width = ui.available_width();
        egui::ComboBox::from_id_salt("interval-project")
            .width(width)
            .wrap_mode(egui::TextWrapMode::Wrap)
            .selected_text(selected)
            .show_ui(ui, |ui| {
                ui.set_max_width(width);
                ui.selectable_value(&mut self.project, None, t("activity.unassigned"));
                if let Some(recorded) = recorded
                    && !catalog
                        .iter()
                        .any(|p| p.id == recorded.id && p.title == recorded.title)
                {
                    ui.selectable_value(
                        &mut self.project,
                        Some(recorded.id),
                        user(&recorded.title).as_ref(),
                    );
                }
                for p in catalog {
                    let label = if p.archived {
                        tf("reports.interval_archived", &[("title", &p.title)])
                    } else {
                        user(&p.title).into_owned()
                    };
                    ui.selectable_value(&mut self.project, Some(p.id), label);
                }
            });
        if old != self.project {
            self.task = None;
        }
        ui.label(t("reports.task"));
        egui::ComboBox::from_id_salt("interval-task")
            .width(width)
            .wrap_mode(egui::TextWrapMode::Wrap)
            .selected_text(
                self.task
                    .as_deref()
                    .map_or_else(|| t("main.no_task").to_owned(), |s| user(s).into_owned()),
            )
            .show_ui(ui, |ui| {
                ui.set_max_width(width);
                ui.selectable_value(&mut self.task, None, t("main.no_task"));
                let tasks = catalog
                    .iter()
                    .find(|p| Some(p.id) == self.project)
                    .map(|p| p.tasks.as_slice())
                    .unwrap_or_default();
                if let Some(recorded) = recorded.filter(|p| Some(p.id) == self.project)
                    && let Some(task) = &recorded.task
                    && !tasks.iter().any(|t| t.title == *task)
                {
                    ui.selectable_value(&mut self.task, Some(task.clone()), user(task).as_ref());
                }
                for task in tasks {
                    let label = if task.done {
                        tf("reports.interval_finished_task", &[("title", &task.title)])
                    } else {
                        user(&task.title).into_owned()
                    };
                    ui.selectable_value(&mut self.task, Some(task.title.clone()), label);
                }
            });
    }
    pub fn show(&mut self, ctx: &egui::Context, catalog: &[CatalogProject], busy: bool) -> Outcome {
        let mut outcome = Outcome::None;
        let response = egui::Modal::new(egui::Id::unique("interval-dialog")).show(ctx, |ui| {
            modal_width(ui);
            crate::i18n::mirrored(ui, |ui| {
                ui.heading(if self.original.is_some() {
                    t("reports.interval_edit_title")
                } else {
                    t("reports.interval_add_title")
                });
                egui::ScrollArea::vertical()
                    .id_salt("interval-draft")
                    .max_height((ctx.content_rect().height() - 160.0).max(80.0))
                    .show(ui, |ui| {
                        ui.add_enabled_ui(!self.saving, |ui| {
                            ui.label(t("reports.interval_start"));
                            self.start.show(ui, "interval-start", false, &self.timezone);
                            ui.label(t("reports.interval_end"));
                            self.end.show(ui, "interval-end", true, &self.timezone);
                            paragraph(
                                ui,
                                &message(
                                    "reports.interval_time_hint",
                                    &[("timezone", &self.timezone)],
                                ),
                            );
                            self.project_picker(ui, catalog);
                            ui.label(t("reports.interval_note"));
                            ui.add(
                                egui::TextEdit::multiline(&mut self.note)
                                    .desired_width(ui.available_width())
                                    .desired_rows(2),
                            );
                        });
                        if let Some(record) = &self.original {
                            paragraph(
                                ui,
                                &message(
                                    "reports.interval_duration_before",
                                    &[("duration", &duration(record.elapsed_ms))],
                                ),
                            );
                        }
                        let resolved = self.resolve(catalog);
                        match &resolved {
                            Ok(edit) => {
                                let elapsed = self
                                    .original
                                    .as_ref()
                                    .filter(|r| {
                                        r.start_ms == edit.start_ms && r.end_ms == edit.end_ms
                                    })
                                    .map_or_else(
                                        || edit.end_ms.saturating_sub(edit.start_ms) as u64,
                                        |r| r.elapsed_ms,
                                    );
                                paragraph(
                                    ui,
                                    &message(
                                        "reports.interval_duration_after",
                                        &[("duration", &duration(elapsed))],
                                    ),
                                );
                            }
                            Err(error) => {
                                paragraph_colored(
                                    ui,
                                    ui.visuals().error_fg_color,
                                    &error.to_string(),
                                );
                            }
                        };
                        if let Some(error) = &self.error {
                            paragraph_colored(ui, ui.visuals().error_fg_color, error);
                        }
                    });
                ui.horizontal_wrapped(|ui| {
                    let resolved = self.resolve(catalog);
                    if ui
                        .add_enabled(
                            !busy && !self.saving && resolved.is_ok(),
                            egui::Button::new(t("reports.interval_save")),
                        )
                        .clicked()
                    {
                        self.saving = true;
                        self.error = None;
                        outcome = Outcome::Send(Command::SaveInterval(resolved.unwrap()));
                    }
                    if ui
                        .add_enabled(
                            !self.saving,
                            egui::Button::new(t("reports.interval_cancel")),
                        )
                        .clicked()
                    {
                        outcome = Outcome::Cancel;
                    }
                });
            });
        });
        if !self.saving && escaped(&response) {
            outcome = Outcome::Cancel;
        }
        outcome
    }
}
fn modal_width(ui: &mut egui::Ui) {
    let width = (ui.ctx().content_rect().width() - 48.0).clamp(180.0, 450.0);
    ui.set_width(width);
}
fn escaped<T>(response: &egui::ModalResponse<T>) -> bool {
    response.is_top_modal
        && !response.any_popup_open
        && response
            .response
            .ctx
            .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
}

pub struct DeleteConfirmation {
    record: Record,
    timezone: String,
    pub(crate) error: Option<String>,
    pub(crate) saving: bool,
}
impl DeleteConfirmation {
    pub fn new(record: Record, timezone: &str) -> Self {
        Self {
            record,
            timezone: timezone.into(),
            error: None,
            saving: false,
        }
    }
    pub fn show(&mut self, ctx: &egui::Context, busy: bool) -> Outcome {
        let mut outcome = Outcome::None;
        let response = egui::Modal::new(egui::Id::unique("interval-delete")).show(ctx, |ui| {
            modal_width(ui);
            crate::i18n::mirrored(ui, |ui| {
                ui.heading(t("reports.interval_delete_title"));
                egui::ScrollArea::vertical()
                    .id_salt("interval-delete-details")
                    .max_height((ctx.content_rect().height() - 160.0).max(80.0))
                    .show(ui, |ui| {
                        ui.label(self.record.activity.painted_label().as_ref());
                        if let Some(task) = self
                            .record
                            .activity
                            .project
                            .as_ref()
                            .and_then(|p| p.task.as_ref())
                        {
                            ui.label(user(task).as_ref());
                        }
                        let zone =
                            TimeZone::get(&self.timezone).unwrap_or_else(|_| TimeZone::system());
                        let format = |ms| {
                            Timestamp::from_millisecond(ms)
                                .expect("Report timestamp is valid")
                                .to_zoned(zone.clone())
                                .strftime("%Y-%m-%d %H:%M:%S %:z")
                                .to_string()
                        };
                        paragraph(
                            ui,
                            &message("reports.start", &[("time", &format(self.record.start_ms))]),
                        );
                        paragraph(
                            ui,
                            &message(
                                "reports.interval_ended_at",
                                &[("time", &format(self.record.end_ms))],
                            ),
                        );
                        paragraph(
                            ui,
                            &message(
                                "reports.interval_duration",
                                &[("duration", &duration(self.record.elapsed_ms))],
                            ),
                        );
                        paragraph(ui, &message("reports.interval_delete_note", &[]));
                        if let Some(error) = &self.error {
                            paragraph_colored(ui, ui.visuals().error_fg_color, error);
                        }
                    });
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(
                            !busy && !self.saving,
                            egui::Button::new(t("reports.interval_delete_confirm")),
                        )
                        .clicked()
                    {
                        self.saving = true;
                        self.error = None;
                        outcome = Outcome::Send(Command::DeleteInterval {
                            id: self.record.id,
                            recorded: (self.record.start_ms, self.record.end_ms),
                        });
                    }
                    if ui
                        .add_enabled(
                            !self.saving,
                            egui::Button::new(t("reports.interval_cancel")),
                        )
                        .clicked()
                    {
                        outcome = Outcome::Cancel;
                    }
                });
            });
        });
        if !self.saving && escaped(&response) {
            outcome = Outcome::Cancel;
        }
        outcome
    }
}

#[cfg(test)]
pub(crate) mod test_ui {
    use super::*;
    pub struct Harness {
        pub context: egui::Context,
        pub width: f32,
        pub height: f32,
        time: f64,
    }
    impl Default for Harness {
        fn default() -> Self {
            Self {
                context: egui::Context::default(),
                width: 700.0,
                height: 900.0,
                time: 0.0,
            }
        }
    }
    impl Harness {
        pub fn frame(
            &mut self,
            events: Vec<egui::Event>,
            mut draw: impl FnMut(&mut egui::Ui),
        ) -> egui::FullOutput {
            self.time += 0.2;
            let mut output = self.context.run_ui(
                egui::RawInput {
                    time: Some(self.time),
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(self.width, self.height),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| draw(ui),
            );
            output.textures_delta.clear();
            output
        }
    }
    pub fn texts(output: &egui::FullOutput) -> Vec<(String, egui::Rect)> {
        output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(text) if !text.galley.job.text.is_empty() => {
                    Some((text.galley.job.text.clone(), text.visual_bounding_rect()))
                }
                _ => None,
            })
            .collect()
    }
    pub fn position(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
        texts(output)
            .into_iter()
            .find(|(text, _)| text == label)
            .unwrap_or_else(|| panic!("Missing {label}: {:?}", texts(output)))
            .1
            .center()
    }
    pub fn pointer(
        pos: egui::Pos2,
        pressed: bool,
        button: egui::PointerButton,
    ) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: Default::default(),
            },
        ]
    }
    pub fn escape() -> Vec<egui::Event> {
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }]
    }
    pub fn record() -> Record {
        Record {
            id: 17,
            start_ms: 1_793_511_000_123,
            end_ms: 1_793_514_600_456,
            elapsed_ms: 3_500_000,
            state: State::Saved,
            activity: Activity::default(),
            reason: "stop".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_ui::{Harness, escape, pointer, position, record, texts};

    struct Editor {
        ui: Harness,
        dialog: Dialog,
        catalog: Vec<CatalogProject>,
        busy: bool,
        outcomes: Vec<Outcome>,
    }
    impl Editor {
        fn new() -> Self {
            Self {
                ui: Harness::default(),
                dialog: Dialog::edit(record(), "UTC"),
                catalog: Vec::new(),
                busy: false,
                outcomes: Vec::new(),
            }
        }
        fn frame(&mut self, events: Vec<egui::Event>) -> egui::FullOutput {
            self.ui.frame(events, |ui| {
                let outcome = self.dialog.show(ui.ctx(), &self.catalog, self.busy);
                if !matches!(outcome, Outcome::None) {
                    self.outcomes.push(outcome);
                }
            })
        }
        fn click(&mut self, label: &str) {
            self.frame(vec![]);
            let output = self.frame(vec![]);
            let pos = position(&output, label);
            self.frame(pointer(pos, true, egui::PointerButton::Primary));
            self.frame(pointer(pos, false, egui::PointerButton::Primary));
        }
    }
    #[test]
    fn untouched_boundaries_preserve_milliseconds_even_in_the_repeated_hour() {
        let mut original = record();
        original.start_ms = "2026-11-01T05:30:23.123Z"
            .parse::<Timestamp>()
            .unwrap()
            .as_millisecond();
        original.end_ms = "2026-11-01T06:30:23.456Z"
            .parse::<Timestamp>()
            .unwrap()
            .as_millisecond();
        let mut dialog = Dialog::edit(original.clone(), "America/New_York");
        assert_eq!(dialog.start.time, "01:30:23");
        let edit = dialog.resolve(&[]).unwrap();
        assert_eq!(
            (edit.start_ms, edit.end_ms),
            (original.start_ms, original.end_ms)
        );
        dialog.start.time = "01:30:24".into();
        assert!(dialog.resolve(&[]).is_err());
        dialog.start.time = "00:30:24".into();
        let edit = dialog.resolve(&[]).unwrap();
        assert_eq!(edit.start_ms % 1000, 0);
        assert_eq!(edit.end_ms, original.end_ms);
        let mut boundary = Boundary::new(original.start_ms / 60000 * 60000, "UTC");
        assert_eq!(boundary.time.len(), 5);
        boundary.time.push_str(":01");
        assert_eq!(boundary.resolve(false, "UTC").unwrap() % 60000, 1000);
    }
    #[test]
    fn new_drafts_reject_repeated_local_times_inline_instead_of_choosing_an_offset() {
        let mut editor = Editor::new();
        editor.dialog = Dialog::add_local("2026-11-01", "01:30", "02:00", "America/New_York");
        assert!(editor.dialog.resolve(&[]).is_err());
        editor.click("Save");
        assert!(editor.outcomes.is_empty());
        let output = editor.frame(vec![]);
        let error = message("errors.time_ambiguous", &[]);
        assert!(texts(&output).iter().any(|(text, _)| text == &error));
        editor.dialog.start.time = "00:30".into();
        assert!(editor.dialog.resolve(&[]).is_ok());
    }
    #[test]
    fn drafts_keep_recorded_activity_after_catalog_removal() {
        let mut original = record();
        original.activity = Activity {
            project: Some(Project {
                id: 5,
                title: "Recorded".into(),
                task: Some("Recorded task".into()),
            }),
            note: "Recorded note".into(),
        };
        let dialog = Dialog::edit(original.clone(), "UTC");
        assert_eq!(dialog.resolve(&[]).unwrap().activity, original.activity);
    }
    #[test]
    fn project_and_task_pickers_offer_archived_and_finished_catalog_entries() {
        let mut editor = Editor::new();
        editor.catalog.push(CatalogProject {
            id: 5,
            title: "Archived".into(),
            archived: true,
            used: false,
            tasks: vec![crate::tracking::CatalogTask {
                title: "Finished".into(),
                done: true,
                used: false,
            }],
        });
        editor.click("Unassigned focus");
        editor.click("Archived (archived)");
        for _ in 0..3 {
            editor.frame(vec![]);
        }
        editor.click("No task");
        editor.click("Finished (finished)");
        for _ in 0..3 {
            editor.frame(vec![]);
        }
        editor.click("Save");
        assert_eq!(editor.outcomes.len(), 1);
        assert!(
            matches!(&editor.outcomes[0], Outcome::Send(Command::SaveInterval(edit))
            if edit.activity.project.as_ref().is_some_and(|p| p.id == 5 && p.task.as_deref() == Some("Finished")))
        );
    }
    #[test]
    fn save_emits_once_cancel_escape_and_outside_clicks_do_not_send() {
        let mut editor = Editor::new();
        editor.click("Save");
        editor.click("Save");
        assert!(editor.dialog.saving);
        assert_eq!(editor.outcomes.len(), 1);
        match &editor.outcomes[0] {
            Outcome::Send(Command::SaveInterval(edit)) => {
                assert_eq!(edit.id, Some(17));
                assert_eq!(edit.start_ms, record().start_ms);
            }
            _ => panic!("Expected one save"),
        }
        for label in ["Cancel", "Escape"] {
            let mut editor = Editor::new();
            if label == "Cancel" {
                editor.click(label);
            } else {
                editor.frame(vec![]);
                editor.frame(escape());
            }
            assert_eq!(editor.outcomes.len(), 1);
            assert!(matches!(editor.outcomes[0], Outcome::Cancel));
        }
        let mut editor = Editor::new();
        editor.frame(vec![]);
        editor.frame(vec![]);
        editor.frame(pointer(
            egui::pos2(2.0, 2.0),
            true,
            egui::PointerButton::Primary,
        ));
        editor.frame(pointer(
            egui::pos2(2.0, 2.0),
            false,
            egui::PointerButton::Primary,
        ));
        assert!(editor.outcomes.is_empty(), "outside clicks keep the draft");
        editor.busy = true;
        editor.click("Save");
        assert!(editor.outcomes.is_empty(), "busy outbox prevents a save");
        editor.busy = false;
        editor.dialog.end.time = "invalid".into();
        editor.click("Save");
        assert!(editor.outcomes.is_empty(), "invalid text prevents a save");
        let output = editor.frame(vec![]);
        assert!(
            texts(&output)
                .iter()
                .any(|(text, _)| text.contains("HH:MM")),
            "inline validation is visible"
        );
    }
    #[test]
    fn time_popup_applies_inside_the_modal_and_its_escape_keeps_the_draft() {
        let mut editor = Editor::new();
        let original = editor.dialog.start.time.clone();
        editor.click("Time");
        let output = editor.frame(vec![]);
        assert!(texts(&output).iter().any(|(text, _)| text == "Hour"));
        assert_eq!(
            editor.dialog.start.picker.draft_for_test(),
            Some((
                editor.dialog.start.time[..2].parse().unwrap(),
                editor.dialog.start.time[3..5].parse().unwrap()
            ))
        );
        editor.frame(escape());
        for _ in 0..3 {
            editor.frame(vec![]);
        }
        assert!(!egui::Popup::is_any_open(&editor.ui.context));
        assert_eq!(editor.dialog.start.time, original);
        assert!(
            editor.outcomes.is_empty(),
            "popup Escape must not discard the modal"
        );
        editor.click("Time");
        editor.click("Apply");
        for _ in 0..3 {
            editor.frame(vec![]);
        }
        assert_eq!(
            editor.dialog.start.time.len(),
            5,
            "applying deliberately edits away seconds"
        );
        assert!(editor.outcomes.is_empty());
    }
    #[test]
    fn calendar_popup_applies_inside_the_modal_and_its_escape_keeps_the_draft() {
        let mut editor = Editor::new();
        let original = editor.dialog.start.date.clone();
        editor.click("Calendar");
        let output = editor.frame(vec![]);
        assert!(texts(&output).iter().any(|(text, _)| text == "Mo"));
        editor.frame(escape());
        for _ in 0..3 {
            editor.frame(vec![]);
        }
        assert_eq!(editor.dialog.start.date, original);
        assert!(editor.outcomes.is_empty());
        editor.click("Calendar");
        editor.click(" 1");
        for _ in 0..3 {
            editor.frame(vec![]);
        }
        assert!(editor.dialog.start.date.ends_with("-01"));
        assert!(editor.outcomes.is_empty());
    }
    #[test]
    fn delete_requires_confirmation_and_cancel_sends_nothing() {
        for confirm in [false, true] {
            let mut harness = Harness::default();
            let mut dialog = DeleteConfirmation::new(record(), "UTC");
            let mut outcomes = Vec::new();
            let mut frame = |harness: &mut Harness, events| {
                harness.frame(events, |ui| {
                    let outcome = dialog.show(ui.ctx(), false);
                    if !matches!(outcome, Outcome::None) {
                        outcomes.push(outcome);
                    }
                })
            };
            frame(&mut harness, vec![]);
            let output = frame(&mut harness, vec![]);
            let label = if confirm { "Delete" } else { "Cancel" };
            let pos = position(&output, label);
            frame(
                &mut harness,
                pointer(pos, true, egui::PointerButton::Primary),
            );
            frame(
                &mut harness,
                pointer(pos, false, egui::PointerButton::Primary),
            );
            assert_eq!(outcomes.len(), 1);
            assert!(if confirm {
                matches!(
                    outcomes[0],
                    Outcome::Send(Command::DeleteInterval { id: 17, .. })
                )
            } else {
                matches!(outcomes[0], Outcome::Cancel)
            });
        }
    }
    #[test]
    fn both_modals_fit_the_narrowest_window_in_every_language() {
        for language in crate::i18n::Language::ALL {
            crate::i18n::with_language(language, || {
                for deleting in [false, true] {
                    let mut harness = Harness::default();
                    harness.width = 380.0;
                    harness.height = 600.0;
                    crate::fonts::Fallbacks::new(language).apply(&harness.context);
                    let mut original = record();
                    original.activity.project=Some(Project {id:1,title:"A synthetic project name that is deliberately long enough to wrap onto another line".into(),task:Some("A synthetic task name that is deliberately long enough to wrap onto another line".into())});
                    let mut dialog =
                        Dialog::edit(original.clone(), "America/Argentina/Buenos_Aires");
                    dialog.error = Some(message(
                        "errors.interval_overlap",
                        &[
                            ("start", "2026-10-26 23:59:59 -03:00"),
                            ("end", "2026-10-27 00:30:00 -03:00"),
                        ],
                    ));
                    let mut deletion =
                        DeleteConfirmation::new(original, "America/Argentina/Buenos_Aires");
                    let mut output = None;
                    for _ in 0..4 {
                        output = Some(harness.frame(vec![], |ui| {
                            if deleting {
                                deletion.show(ui.ctx(), false);
                            } else {
                                dialog.show(ui.ctx(), &[], false);
                            }
                        }));
                    }
                    let output = output.unwrap();
                    for (text, rect) in texts(&output) {
                        // Scroll content may extend vertically, but its glyphs cannot widen the modal.
                        assert!(
                            rect.left() >= -0.5 && rect.right() <= 380.5,
                            "{language:?} delete={deleting} {rect:?}: {text}"
                        );
                    }
                    let cancel = position(&output, t("reports.interval_cancel"));
                    assert!(
                        cancel.y >= 0.0 && cancel.y <= 600.0,
                        "{language:?}: actions stay visible"
                    );
                }
            });
        }
    }
}
