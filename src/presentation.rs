//! Short desktop labels with explicit state and Windows tooltip limits.
use crate::{i18n::message, profiles::Profile, tracking::Snapshot};

pub fn duration(ms: u64) -> String {
    let seconds = ms / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}
/// Compact form for chart labels, where seconds would only add noise. The text is logical.
pub fn hours_minutes(ms: u64) -> String {
    let minutes = (ms + 30_000) / 60_000;
    match (minutes / 60, minutes % 60) {
        (0, 0) if ms > 0 => message("units.under_minute", &[]),
        (0, minutes) => message("units.minutes", &[("minutes", &minutes.to_string())]),
        (hours, 0) => message("units.hours", &[("hours", &hours.to_string())]),
        (hours, minutes) => message(
            "units.hours_minutes",
            &[
                ("hours", &hours.to_string()),
                ("minutes", &format!("{minutes:02}")),
            ],
        ),
    }
}
fn shorten(text: &str, limit: usize) -> String {
    if text.encode_utf16().count() <= limit {
        return text.to_owned();
    }
    let mut result = String::new();
    let mut size = 0;
    for c in text.chars() {
        if size + c.len_utf16() > limit.saturating_sub(3) {
            break;
        }
        size += c.len_utf16();
        result.push(c);
    }
    result.push_str(&".".repeat(limit.min(3)));
    result
}
/// The tray tooltip, in logical order because Windows draws it.
pub fn tooltip(profile: Profile, state: &Snapshot) -> String {
    let title = message(
        "tray.tooltip_title",
        &[
            (
                "product",
                if profile == Profile::Development {
                    "Attention Focus [Dev]"
                } else {
                    "Attention Focus"
                },
            ),
            (
                "state",
                &if state.running {
                    message("tray.tooltip_tracking", &[])
                } else {
                    message("tray.tooltip_stopped", &[])
                },
            ),
        ],
    );
    let unavailable = || message("tray.tooltip_unavailable", &[]);
    let times = format!(
        "\n{}\n{}\n{}",
        message(
            "tray.tooltip_session",
            &[("duration", &duration(state.elapsed_ms))]
        ),
        message(
            "tray.tooltip_today_project",
            &[(
                "duration",
                &state
                    .daily_project_ms
                    .map(duration)
                    .unwrap_or_else(unavailable)
            )]
        ),
        message(
            "tray.tooltip_today_all",
            &[(
                "duration",
                &state.daily_all_ms.map(duration).unwrap_or_else(unavailable)
            )]
        ),
    );
    // Windows cuts a tooltip at 127 units. The times come first, then this notice, then the name.
    const LIMIT: usize = 127;
    const SHORTEST_NAME: usize = 8;
    let units = |text: &str| text.encode_utf16().count();
    let mut room = LIMIT.saturating_sub(units(&title) + units(&times));
    let review = if state.needs_review {
        let line = format!("\n{}", message("tray.tooltip_review", &[]));
        if units(&line) <= room {
            room -= units(&line);
            line
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    let project = {
        let prefix = message("tray.tooltip_project", &[("name", "")]);
        let budget = room.saturating_sub(1 + units(&prefix));
        let name: String = state
            .activity
            .logical_label()
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect();
        if budget < SHORTEST_NAME {
            String::new()
        } else {
            format!(
                "\n{}",
                message("tray.tooltip_project", &[("name", &shorten(&name, budget))])
            )
        }
    };
    shorten(&format!("{title}{project}{times}{review}"), LIMIT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracking::{Activity, Project};
    #[test]
    fn chart_durations_round_to_minutes_without_hiding_small_amounts() {
        assert_eq!(hours_minutes(0), "0m");
        assert_eq!(hours_minutes(29_000), "<1m");
        assert_eq!(hours_minutes(45 * 60_000), "45m");
        assert_eq!(hours_minutes(3_600_000), "1h");
        assert_eq!(hours_minutes(6 * 3_600_000 + 19 * 60_000), "6h 19m");
        assert_eq!(hours_minutes(3_599_900), "1h");
    }
    #[test]
    fn tooltip_keeps_within_the_windows_limit_and_shows_the_time_in_every_language() {
        let state = Snapshot {
            running: true,
            elapsed_ms: 12_000,
            daily_project_ms: Some(42_000),
            daily_all_ms: Some(55_000),
            needs_review: true,
            activity: Activity {
                project: Some(Project {
                    id: 1,
                    title: "Example project with quite a long name".to_owned(),
                    task: None,
                }),
                note: String::new(),
            },
            ..Snapshot::default()
        };
        for language in crate::i18n::Language::ALL {
            crate::i18n::with_language(language, || {
                let text = tooltip(Profile::Development, &state);
                assert!(text.encode_utf16().count() <= 127, "{language:?}: {text}");
                assert!(text.contains("00:00:12"), "{language:?}: {text}");
                assert!(text.contains("00:00:42"), "{language:?}: {text}");
                assert!(text.contains("00:00:55"), "{language:?}: {text}");
            });
        }
    }
    #[test]
    fn tooltip_names_the_selected_project_and_preserves_daily_time_with_long_unicode() {
        let mut state = Snapshot {
            running: true,
            elapsed_ms: 12_000,
            total_ms: 60_000,
            daily_project_ms: Some(42_000),
            daily_all_ms: Some(55_000),
            activity: Activity {
                project: Some(Project {
                    id: 1,
                    title: "Example project ".to_owned() + &"\u{1f331}".repeat(150),
                    task: None,
                }),
                note: String::new(),
            },
            ..Snapshot::default()
        };
        let text = tooltip(Profile::Development, &state);
        assert!(text.encode_utf16().count() <= 127);
        assert!(text.contains("Project: Example project"));
        assert!(text.contains("Session: 00:00:12"));
        assert!(text.contains("Today/project: 00:00:42"));
        assert!(text.contains("Today/all: 00:00:55"));
        assert!(text.contains("\nSession:"));
        state.activity.project.as_mut().unwrap().title = "Line\nNext".into();
        assert!(tooltip(Profile::Production, &state).contains("Project: Line Next\n"));
        state.running = false;
        assert!(tooltip(Profile::Production, &state).contains("Project:"));
        state.needs_review = true;
        assert!(tooltip(Profile::Production, &state).contains("Interrupted time to accept"));
        assert_eq!(shorten("\u{1f331}\u{1f331}", 4), "\u{1f331}\u{1f331}");
    }
}
