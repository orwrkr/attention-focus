#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use std::sync::mpsc::{self, Receiver};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem},
};

enum Action {
    Show,
    Exit,
    Storage(Result<String, String>),
}

struct Desktop {
    tray: Option<TrayIcon>,
    tray_error: Option<String>,
    actions: Receiver<Action>,
    storage: String,
    exiting: bool,
}

impl Desktop {
    fn new(context: &eframe::CreationContext<'_>) -> Self {
        let (sender, actions) = mpsc::channel();
        let menu = Menu::new();
        let show = MenuItem::new("Show Focus", true, None);
        let exit = MenuItem::new("Exit", true, None);
        let tray_result = (|| -> Result<TrayIcon, Box<dyn std::error::Error>> {
            menu.append_items(&[&show, &exit])?;
            let pixels = [40, 160, 140, 255].repeat(32 * 32);
            Ok(TrayIconBuilder::new()
                .with_tooltip("Attention Focus")
                .with_icon(Icon::from_rgba(pixels, 32, 32)?)
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .build()?)
        })();
        let (tray, tray_error) = match tray_result {
            Ok(tray) => (Some(tray), None),
            Err(error) => (None, Some(error.to_string())),
        };

        let menu_sender = sender.clone();
        let ctx = context.egui_ctx.clone();
        let show_id = show.id().clone();
        let exit_id = exit.id().clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let action = if event.id == show_id {
                Some(Action::Show)
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
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                let _ = tray_sender.send(Action::Show);
                ctx.request_repaint();
            }
        }));
        let ctx = context.egui_ctx.clone();
        std::thread::spawn(move || {
            let result = attention_focus::sqlite_probe().map_err(|error| error.to_string());
            let _ = sender.send(Action::Storage(result));
            ctx.request_repaint();
        });
        Self {
            tray,
            tray_error,
            actions,
            storage: "Checking temporary SQLite storage...".into(),
            exiting: false,
        }
    }
}

impl eframe::App for Desktop {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(action) = self.actions.try_recv() {
            match action {
                Action::Show => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                Action::Exit => {
                    self.exiting = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Action::Storage(result) => {
                    self.storage = match result {
                        Ok(value) => format!("{value}: saved, reopened, and removed successfully."),
                        Err(error) => format!("Storage check failed: {error}"),
                    };
                }
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Hidden-window logic retains the last input snapshot. Handle close only
        // in a fresh visible UI pass so a stale close cannot hide a restored window.
        let ctx = ui.ctx();
        if ctx.input(|input| input.viewport().close_requested())
            && !self.exiting
            && self.tray.is_some()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            ui.heading("Attention Focus");
            ui.label("Desktop integration preview");
            ui.separator();
            ui.label("Your focus tracker is taking shape.");
            ui.label("Project selection and time tracking are not available yet.");
            ui.add_space(16.0);
            ui.label(&self.storage);
            ui.label("This preview does not read or modify MetaPlan.");
            ui.add_space(16.0);
            if let Some(error) = &self.tray_error {
                ui.colored_label(egui::Color32::RED, format!("Tray unavailable: {error}"));
                ui.label("The window stays accessible. Closing it exits the preview.");
            } else {
                ui.label("Close this window to hide it. Click the tray icon to return.");
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(self.tray.is_some(), egui::Button::new("Hide to tray"))
                    .clicked()
                {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                }
                if ui.button("Exit").clicked() {
                    self.exiting = true;
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "Attention Focus",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([540.0, 300.0]),
            ..Default::default()
        },
        Box::new(|context| Ok(Box::new(Desktop::new(context)))),
    )
}
