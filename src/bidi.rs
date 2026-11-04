//! Right-to-left text for egui, which has no bidirectional layout.
//!
//! egui shapes one run of a single font correctly, including Arabic joining and letter order, but
//! places runs from left to right. The bundled font has spaces, digits and punctuation but no
//! Arabic, so each Arabic word already forms its own run. Giving egui the runs in the order they
//! appear on screen, each word still in its logical order, therefore paints correct text.
//!
//! Reordering must happen line by line, because egui would otherwise wrap the end of a sentence
//! onto the first row. Revisit this once egui supports bidirectional text; see ADR-0035.
use std::borrow::Cow;
use unicode_bidi::{BidiInfo, Level};

fn is_rtl_letter(c: char) -> bool {
    matches!(c as u32, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF)
}
fn mirrored(c: char) -> char {
    match c {
        '(' => ')',
        ')' => '(',
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        '<' => '>',
        '>' => '<',
        '\u{AB}' => '\u{BB}',
        '\u{BB}' => '\u{AB}',
        other => other,
    }
}

/// A right-to-left run, with words kept whole for the shaper and everything else reversed.
fn push_reversed(run: &str, out: &mut String) {
    let mut pieces = Vec::new();
    let mut rest = run;
    while let Some(first) = rest.chars().next() {
        let end = if is_rtl_letter(first) {
            rest.find(|c| !is_rtl_letter(c)).unwrap_or(rest.len())
        } else {
            first.len_utf8()
        };
        pieces.push(&rest[..end]);
        rest = &rest[end..];
    }
    for piece in pieces.into_iter().rev() {
        if piece.chars().next().is_some_and(is_rtl_letter) {
            out.push_str(piece);
        } else {
            out.extend(piece.chars().map(mirrored));
        }
    }
}

fn push_line(line: &str, level: Option<Level>, out: &mut String) {
    let info = BidiInfo::new(line, level);
    for paragraph in &info.paragraphs {
        let (levels, runs) = info.visual_runs(paragraph, paragraph.range.clone());
        for run in runs {
            let piece = &line[run.clone()];
            if levels[run.start].is_rtl() {
                push_reversed(piece, out);
            } else {
                out.push_str(piece);
            }
        }
    }
}

fn reordered(text: &str, level: Option<Level>) -> String {
    let mut out = String::with_capacity(text.len());
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        push_line(line, level, &mut out);
    }
    // These have done their work, and egui does not need to see them.
    out.retain(|c| ![ISOLATE_START, ISOLATE_END].contains(&c));
    out
}

const ISOLATE_START: char = '\u{2068}';
const ISOLATE_END: char = '\u{2069}';
/// Keeps an inserted value whole in right-to-left text. Without this, Unicode rules reverse the
/// groups of a date such as 2026-11-04 next to Arabic letters. Windows honors these as well.
pub fn isolated(value: &str) -> String {
    format!("{ISOLATE_START}{value}{ISOLATE_END}")
}

/// Interface text in a right-to-left language, whatever it starts with.
pub fn right_to_left(text: &str) -> String {
    reordered(text, Some(Level::rtl()))
}

/// Left-to-right interface text that may contain right-to-left user data.
pub fn left_to_right(text: &str) -> Cow<'_, str> {
    if text.chars().any(is_rtl_letter) {
        Cow::Owned(reordered(text, Some(Level::ltr())))
    } else {
        Cow::Borrowed(text)
    }
}

/// User data keeps its own direction, so a Latin project name is left alone in any language.
pub fn user_text(text: &str) -> Cow<'_, str> {
    if text.chars().any(is_rtl_letter) {
        Cow::Owned(reordered(text, None))
    } else {
        Cow::Borrowed(text)
    }
}

/// Fills lines in reading order, so the start of the text stays on the first line. A word wider
/// than the limit keeps a line of its own, and line breaks in the text are kept.
pub fn wrap(text: &str, width: f32, mut measure: impl FnMut(&str) -> f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split(' ') {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && measure(&candidate) > width {
                lines.push(std::mem::take(&mut line));
                line = word.to_owned();
            } else {
                line = candidate;
            }
        }
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    // Two Arabic words, meaning "hello" and "world", kept as escapes in this English source.
    const HELLO: &str = "\u{645}\u{631}\u{62d}\u{628}\u{627}";
    const WORLD: &str = "\u{639}\u{627}\u{644}\u{645}";

    #[test]
    fn words_appear_in_screen_order_and_each_stays_whole_for_the_shaper() {
        assert_eq!(
            right_to_left(&format!("{HELLO} {WORLD}")),
            format!("{WORLD} {HELLO}")
        );
    }
    #[test]
    fn numbers_and_latin_text_keep_their_direction_inside_right_to_left_text() {
        let date = isolated("2026-11-04");
        assert_eq!(
            right_to_left(&format!("{HELLO} {date} {WORLD}")),
            format!("{WORLD} 2026-11-04 {HELLO}")
        );
        assert_eq!(
            right_to_left(&format!("{HELLO} {}", isolated("C:\\Some path\\file.csv"))),
            format!("C:\\Some path\\file.csv {HELLO}")
        );
        assert_eq!(
            right_to_left(&format!("{HELLO} Attention Focus")),
            format!("Attention Focus {HELLO}")
        );
        assert_eq!(
            right_to_left(&format!("{HELLO}: 12:30")),
            format!("12:30 :{HELLO}")
        );
    }
    #[test]
    fn brackets_are_mirrored_around_right_to_left_text() {
        assert_eq!(
            right_to_left(&format!("{WORLD} ({HELLO})")),
            format!("({HELLO}) {WORLD}")
        );
    }
    #[test]
    fn each_line_is_ordered_on_its_own() {
        assert_eq!(
            right_to_left(&format!("{HELLO} {WORLD}\n{WORLD} {HELLO}")),
            format!("{WORLD} {HELLO}\n{HELLO} {WORLD}")
        );
    }
    #[test]
    fn user_text_only_changes_when_it_contains_right_to_left_letters() {
        assert!(matches!(user_text("Website redesign"), Cow::Borrowed(_)));
        assert_eq!(user_text("Website redesign"), "Website redesign");
        assert_eq!(
            user_text(&format!("{HELLO} {WORLD}")),
            format!("{WORLD} {HELLO}")
        );
    }
    #[test]
    fn a_name_inside_left_to_right_text_is_ordered_without_moving_that_text() {
        assert_eq!(
            left_to_right(&format!("{HELLO} {WORLD} (Archived)")),
            format!("{WORLD} {HELLO} (Archived)")
        );
        assert!(matches!(left_to_right("Plain (text)"), Cow::Borrowed(_)));
    }
    #[test]
    fn wrapping_keeps_the_start_of_the_text_on_the_first_line() {
        let by_length = |line: &str| line.chars().count() as f32;
        assert_eq!(
            wrap("one two three four", 9.0, by_length),
            ["one two", "three", "four"]
        );
        assert_eq!(wrap("short", 100.0, by_length), ["short"]);
        assert_eq!(
            wrap("averyveryverylongword x", 5.0, by_length),
            ["averyveryverylongword", "x"]
        );
        assert_eq!(wrap("a\nb", 100.0, by_length), ["a", "b"]);
    }
}
