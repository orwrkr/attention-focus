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
        profile.title(),
        if state.running { "tracking" } else { "stopped" }
    );
    let times = format!(
        "\nSession: {}\nTotal: {}{}",
        duration(state.elapsed_ms),
        duration(state.total_ms.saturating_add(state.elapsed_ms)),
        if state.needs_review {
            "\nInterrupted time needs review"
        } else {
            ""
        }
    );
    let project = if state.running {
        let budget = 127usize
            .saturating_sub(title.encode_utf16().count() + times.encode_utf16().count() + 10);
        let name: String = state
            .activity
            .label()
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect();
        format!("\nProject: {}", shorten(&name, budget))
    } else {
        String::new()
    };
    shorten(&format!("{title}{project}{times}"), 127)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracking::{Activity, Project};
    #[test]
    fn tooltip_names_only_the_active_project_and_preserves_times_with_long_unicode() {
        let mut state = Snapshot {
            running: true,
            elapsed_ms: 12_000,
            total_ms: 60_000,
            activity: Activity {
                project: Some(Project {
                    source_id: 1,
                    source_path: "synthetic".into(),
                    slug: "example".into(),
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
        assert!(text.contains("Total: 00:01:12"));
        assert!(text.contains("\nSession:"));
        state.activity.project.as_mut().unwrap().title = "Line\nNext".into();
        assert!(tooltip(Profile::Production, &state).contains("Project: Line Next\n"));
        state.running = false;
        assert!(!tooltip(Profile::Production, &state).contains("Project:"));
        state.needs_review = true;
        assert!(tooltip(Profile::Production, &state).contains("needs review"));
        assert_eq!(shorten("\u{1f331}\u{1f331}", 4), "\u{1f331}\u{1f331}");
    }
}
