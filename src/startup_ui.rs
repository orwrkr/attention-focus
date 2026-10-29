//! Startup preferences backed by actual per-user registration, never an assumed flag.
use crate::{profiles::Profile, startup};
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
            .checkbox(&mut enabled, "Start at Windows sign-in")
            .changed()
        {
            self.apply(enabled);
        }
        let previous = self.tray;
        if ui
            .checkbox(&mut self.tray, "Start in tray when launched at sign-in")
            .changed()
            && self.entry.is_some()
            && !self.apply(true)
        {
            self.tray = previous;
        }
        ui.weak("Manual launches show the window. Tracking always starts stopped.");
        if self.entry.is_some() && self.expected().ok().as_ref() != self.entry.as_ref() {
            ui.label("Startup points to another copy or different options.");
            if ui.button("Use this EXE for startup").clicked() {
                self.apply(true);
            }
        }
        if ui.button("Refresh startup status").clicked() {
            self.refresh();
        }
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
    }
}
