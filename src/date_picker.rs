//! Calendar selection backed by the same civil dates as report accounting.
use eframe::egui;
use jiff::{Span, Timestamp, civil::Date, tz::TimeZone};

#[derive(Default)]
pub struct Calendar {
    month: Option<Date>,
    selected_text: String,
}
fn first(date: Date) -> Date {
    Date::new(date.year(), date.month(), 1).expect("Existing civil month is valid")
}
fn shift(date: Date, months: i32) -> Option<Date> {
    date.checked_add(Span::new().months(months))
        .ok()
        .filter(|d| d.year() >= 1)
}
fn cells(month: Date) -> Vec<Option<Date>> {
    let offset = month.weekday().to_monday_zero_offset() as usize;
    let count = month.days_in_month() as usize;
    (0..42)
        .map(|index| {
            if index >= offset && index < offset + count {
                Date::new(month.year(), month.month(), (index - offset + 1) as i8).ok()
            } else {
                None
            }
        })
        .collect()
}
impl Calendar {
    pub fn show(&mut self, ui: &mut egui::Ui, text: &mut String, id: &str) -> bool {
        if self.selected_text != *text {
            self.month = None;
            self.selected_text = text.clone();
        }
        let mut changed = false;
        ui.push_id(id, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add(egui::TextEdit::singleline(text).desired_width(110.0))
                    .changed()
                {
                    self.month = None;
                    changed = true;
                }
                ui.menu_button("Calendar", |ui| {
                    let selected = text.parse::<Date>().ok();
                    let month = self.month.get_or_insert_with(|| {
                        first(
                            selected
                                .unwrap_or_else(|| Timestamp::now().to_zoned(TimeZone::UTC).date()),
                        )
                    });
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(shift(*month, -1).is_some(), egui::Button::new("<"))
                            .clicked()
                        {
                            *month = shift(*month, -1).unwrap();
                        }
                        const MONTHS: [&str; 12] = [
                            "January",
                            "February",
                            "March",
                            "April",
                            "May",
                            "June",
                            "July",
                            "August",
                            "September",
                            "October",
                            "November",
                            "December",
                        ];
                        egui::ComboBox::from_id_salt("month")
                            .selected_text(MONTHS[(month.month() - 1) as usize])
                            .show_ui(ui, |ui| {
                                for (index, label) in MONTHS.iter().enumerate() {
                                    if ui
                                        .selectable_label(
                                            month.month() as usize == index + 1,
                                            *label,
                                        )
                                        .clicked()
                                    {
                                        *month =
                                            Date::new(month.year(), index as i8 + 1, 1).unwrap();
                                    }
                                }
                            });
                        let mut year = i32::from(month.year());
                        if ui
                            .add(egui::DragValue::new(&mut year).range(1..=9999))
                            .changed()
                        {
                            *month = Date::new(year as i16, month.month(), 1).unwrap();
                        }
                        if ui
                            .add_enabled(shift(*month, 1).is_some(), egui::Button::new(">"))
                            .clicked()
                        {
                            *month = shift(*month, 1).unwrap();
                        }
                    });
                    egui::Grid::new("days").spacing([6.0, 4.0]).show(ui, |ui| {
                        for day in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"] {
                            ui.strong(day);
                        }
                        ui.end_row();
                        for (index, date) in cells(*month).into_iter().enumerate() {
                            if let Some(date) = date {
                                if ui
                                    .selectable_label(
                                        selected == Some(date),
                                        format!("{:2}", date.day()),
                                    )
                                    .clicked()
                                {
                                    *text = date.to_string();
                                    changed = true;
                                    ui.close();
                                }
                            } else {
                                ui.label(" ");
                            }
                            if index % 7 == 6 {
                                ui.end_row();
                            }
                        }
                    });
                });
            });
        });
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calendar_preserves_valid_days_leap_rules_and_year_boundaries() {
        for (year, days) in [(2024, 29), (2000, 29), (2100, 28)] {
            let month = Date::new(year, 2, 1).unwrap();
            let dates: Vec<_> = cells(month).into_iter().flatten().collect();
            assert_eq!(dates.len(), days);
            assert_eq!(dates.first().unwrap().day(), 1);
            assert_eq!(dates.last().unwrap().day(), days as i8);
            assert_eq!(
                cells(month).iter().position(Option::is_some).unwrap(),
                month.weekday().to_monday_zero_offset() as usize
            );
        }
        assert_eq!(
            shift(Date::new(2026, 12, 1).unwrap(), 1),
            Some(Date::new(2027, 1, 1).unwrap())
        );
        assert_eq!(
            shift(Date::new(2026, 1, 1).unwrap(), -1),
            Some(Date::new(2025, 12, 1).unwrap())
        );
        assert!(shift(Date::new(9999, 12, 1).unwrap(), 1).is_none());
        assert!(shift(Date::new(1, 1, 1).unwrap(), -1).is_none());
    }
}
