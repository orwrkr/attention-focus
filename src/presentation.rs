//! Short desktop labels with explicit state and Windows tooltip limits.
use crate::{profiles::Profile, tracking::Snapshot};

pub fn duration(ms: u64) -> String {
    let seconds = ms / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}
/// Compact form for chart labels, where seconds would only add noise.
pub fn hours_minutes(ms: u64) -> String {
    let minutes = (ms + 30_000) / 60_000;
    match (minutes / 60, minutes % 60) {
        (0, 0) if ms > 0 => "<1m".into(),
        (0, minutes) => format!("{minutes}m"),
        (hours, 0) => format!("{hours}h"),
        (hours, minutes) => format!("{hours}h {minutes:02}m"),
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
pub fn tooltip(profile: Profile, state: &Snapshot) -> String {
    let title = format!(
        "{} - {}",
        if profile == Profile::Development {
            "Attention Focus [Dev]"
        } else {
            "Attention Focus"
        },
        if state.running { "tracking" } else { "stopped" }
    );
    let times = format!(
        "\nSession: {}\nToday/project: {}\nToday/all: {}{}",
        duration(state.elapsed_ms),
        state
            .daily_project_ms
            .map(duration)
            .unwrap_or_else(|| "unavailable".into()),
        state
            .daily_all_ms
            .map(duration)
            .unwrap_or_else(|| "unavailable".into()),
        if state.needs_review {
            "\nInterrupted time to accept"
        } else {
            ""
        }
    );
    let project = {
        let budget = 127usize
            .saturating_sub(title.encode_utf16().count() + times.encode_utf16().count() + 10);
        let name: String = state
            .activity
            .label()
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect();
        format!("\nProject: {}", shorten(&name, budget))
    };
    shorten(&format!("{title}{project}{times}"), 127)
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
