//! Startup preferences backed by actual per-user registration, never an assumed flag.
use crate::{
    i18n::{checkbox_label, message, paragraph, paragraph_colored, t},
    profiles::Profile,
    startup,
};
use eframe::egui;

pub struct Panel {
    profile: Profile,
    entry: Option<String>,
    pub tray: bool,
    error: Option<String>,
}
impl Panel {
    pub fn new(profile: Profile, tray: bool) -> Self {
        let mut panel = Self {
            profile,
            entry: None,
            tray,
            error: None,
        };
        panel.refresh();
        panel
    }
    fn refresh(&mut self) {
        match startup::read(self.profile) {
            Ok(entry) => {
                if let Some(value) = &entry {
                    self.tray = value.ends_with(" --start-in-tray");
                }
                self.entry = entry;
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }
    fn expected(&self) -> Result<String, String> {
        std::env::current_exe()
            .and_then(|exe| startup::command(&exe, self.profile, self.tray))
            .map_err(|e| e.to_string())
    }
    fn apply(&mut self, enabled: bool) -> bool {
        let result = if enabled {
            self.expected().and_then(|value| {
                startup::set(self.profile, Some(&value)).map_err(|e| e.to_string())
            })
        } else {
            startup::set(self.profile, None).map_err(|e| e.to_string())
        };
        match result {
            Ok(()) => {
                self.refresh();
                self.error.is_none()
            }
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let mut enabled = self.entry.is_some();
        if ui
            .checkbox(&mut enabled, checkbox_label(ui, "startup.at_sign_in"))
            .changed()
        {
            self.apply(enabled);
        }
        let previous = self.tray;
        let tray = &mut self.tray;
        if crate::ui_support::with_info(ui, &message("startup.manual_launch_hint", &[]), |ui| {
            ui.checkbox(tray, checkbox_label(ui, "startup.in_tray"))
        })
        .changed()
            && self.entry.is_some()
            && !self.apply(true)
        {
            self.tray = previous;
        }
        if self.entry.is_some() && self.expected().ok().as_ref() != self.entry.as_ref() {
            paragraph(ui, &message("startup.points_elsewhere", &[]));
            if ui.button(t("startup.use_this_exe")).clicked() {
                self.apply(true);
            }
        }
        if ui.button(t("startup.refresh")).clicked() {
            self.refresh();
        }
        if let Some(error) = &self.error {
            paragraph_colored(ui, ui.visuals().error_fg_color, error);
        }
    }
}
