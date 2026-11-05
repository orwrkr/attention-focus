//! The license of the application and of every component it includes. The text travels inside
//! the EXE and can be read in Settings, so the program can be distributed as one file; see ADR-0036.
use crate::i18n::{message, paragraph, t, unmirrored};
use eframe::egui;
use std::ops::Range;

const COMPRESSED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/notices.deflate"));
const NARROWEST_ROW: usize = 20;

/// A test confirms that the embedded data decompresses, so a release cannot reach the default.
pub fn text() -> String {
    miniz_oxide::inflate::decompress_to_vec(COMPRESSED)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default()
}

/// Splits text into rows of at most `columns` characters, at a space where a line has one.
fn rows(text: &str, columns: usize) -> Vec<Range<usize>> {
    let columns = columns.max(1);
    let mut rows = Vec::new();
    let mut offset = 0;
    for line in text.split('\n') {
        let end = offset + line.len();
        let mut start = offset;
        while let Some((limit, _)) = text[start..end].char_indices().nth(columns) {
            let rest = &text[start..end];
            let space = if rest[limit..].starts_with(' ') {
                Some(limit)
            } else {
                rest[..limit].rfind(' ')
            };
            // Breaking at the indentation of a line would only add an empty row.
            match space.filter(|&space| !rest[..space].trim().is_empty()) {
                Some(space) => {
                    rows.push(start..start + space);
                    start += space + 1;
                }
                None => {
                    rows.push(start..start + limit);
                    start += limit;
                }
            }
        }
        rows.push(start..end);
        offset = end + 1;
    }
    rows
}

struct Contents {
    text: String,
    columns: usize,
    rows: Vec<Range<usize>>,
}
impl Contents {
    fn new() -> Self {
        Self {
            // The rows count characters, so every character must take one column.
            text: text().replace('\t', "    "),
            columns: 0,
            rows: Vec::new(),
        }
    }
    fn fit(&mut self, columns: usize) {
        if self.columns != columns {
            self.columns = columns;
            self.rows = rows(&self.text, columns);
        }
    }
}

/// The viewer holds the text only while it is open.
#[derive(Default)]
pub struct Panel {
    contents: Option<Contents>,
}
impl Panel {
    pub fn show(&mut self, ui: &mut egui::Ui) {
        paragraph(ui, &message("settings.license_summary", &[]));
        let open = self.contents.is_some();
        if ui
            .add(egui::Button::new(t("settings.licenses")).selected(open))
            .clicked()
        {
            self.contents = (!open).then(Contents::new);
        }
        if let Some(contents) = &mut self.contents {
            // License texts are in English and tens of thousands of lines long, so they always
            // run from left to right, and only the visible rows are laid out. Those rows must fit
            // the width, since a wrapped label would not have the height the area counts on.
            unmirrored(ui, |ui| {
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let style = egui::TextStyle::Monospace;
                    let font = style.resolve(ui.style());
                    let column = ui.ctx().fonts_mut(|fonts| fonts.glyph_width(&font, 'M'));
                    let width = ui.available_width() - ui.spacing().scroll.floating_allocated_width;
                    // One column spare for a character that a wider fallback font draws.
                    contents.fit(
                        ((width / column) as usize)
                            .saturating_sub(1)
                            .max(NARROWEST_ROW),
                    );
                    crate::ui_support::inner_area("licenses", 360.0).show_rows(
                        ui,
                        ui.text_style_height(&style),
                        contents.rows.len(),
                        |ui, visible| {
                            for row in &contents.rows[visible] {
                                let text = &contents.text[row.clone()];
                                ui.add(
                                    egui::Label::new(egui::RichText::new(text).monospace())
                                        .extend(),
                                );
                            }
                        },
                    );
                });
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_text_covers_the_application_and_its_components() {
        let text = text();
        assert!(text.starts_with("Attention Focus - License and Third-Party Notices"));
        for expected in [
            "MIT License",
            "Apache License",
            "SIL OPEN FONT LICENSE",
            "UBUNTU FONT LICENCE",
            "Rust standard library",
            "unicode-bidi",
            "rusqlite",
            "egui",
        ] {
            assert!(text.contains(expected), "{expected}");
        }
    }

    #[test]
    fn rows_fit_the_width_and_lose_no_text() {
        let text = Contents::new().text;
        let visible =
            |text: &str| -> String { text.chars().filter(|c| !c.is_whitespace()).collect() };
        let whole = visible(&text);
        for columns in [NARROWEST_ROW, 47, 80] {
            let rows = rows(&text, columns);
            assert!(rows.len() >= text.split('\n').count());
            let mut rebuilt = String::with_capacity(whole.len());
            for row in rows {
                let row = &text[row];
                assert!(row.chars().count() <= columns, "{columns}: {row}");
                rebuilt.push_str(&visible(row));
            }
            assert!(rebuilt == whole, "{columns} columns changed the text");
        }
    }

    #[test]
    fn rows_break_at_spaces_and_keep_empty_lines() {
        let split = |text: &'static str, columns| -> Vec<&'static str> {
            rows(text, columns)
                .into_iter()
                .map(|row| &text[row])
                .collect()
        };
        assert_eq!(split("one two three", 7), ["one two", "three"]);
        assert_eq!(split("one two three", 8), ["one two", "three"]);
        assert_eq!(split("a\n\nb", 8), ["a", "", "b"]);
        assert_eq!(split("abcdefghij", 4), ["abcd", "efgh", "ij"]);
        // Indentation stays with its text instead of taking a row for itself.
        assert_eq!(split("    abcdefghij", 8), ["    abcd", "efghij"]);
        // Three Cyrillic letters, two bytes each: a limit in bytes would split one of them.
        assert_eq!(
            split("\u{430}\u{431}\u{432}\u{433}", 3),
            ["\u{430}\u{431}\u{432}", "\u{433}"]
        );
    }

    #[test]
    fn the_viewer_lays_out_only_visible_rows_within_the_width_in_every_language() {
        const WIDTH: f32 = 380.0;
        for language in [
            crate::i18n::Language::English,
            crate::i18n::Language::Arabic,
        ] {
            crate::i18n::with_language(language, || {
                let context = egui::Context::default();
                let mut panel = Panel {
                    contents: Some(Contents::new()),
                };
                let mut output = None;
                for pass in 0..2 {
                    let mut result = context.run_ui(
                        egui::RawInput {
                            time: Some(f64::from(pass)),
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(WIDTH, 800.0),
                            )),
                            ..Default::default()
                        },
                        // The application mirrors its root layout in right-to-left languages.
                        |ui| crate::i18n::mirrored(ui, |ui| panel.show(ui)),
                    );
                    result.textures_delta.clear();
                    output = Some(result);
                }
                let total = panel.contents.as_ref().unwrap().rows.len();
                assert!(total > 10_000, "{total}");
                let mut texts = Vec::new();
                for shape in &output.unwrap().shapes {
                    if let egui::Shape::Text(text) = &shape.shape {
                        // Measured from the galley, as right-aligned text extends to the left.
                        let (left, right) = (
                            text.pos.x + text.galley.rect.min.x,
                            text.pos.x + text.galley.rect.max.x,
                        );
                        assert!(
                            left >= -0.5 && right <= WIDTH + 0.5,
                            "{language:?} {left}..{right}: {}",
                            text.galley.job.text
                        );
                        texts.push(text.galley.job.text.clone());
                    }
                }
                assert!(texts.len() < 60, "{language:?} laid out {}", texts.len());
                assert!(
                    texts
                        .iter()
                        .any(|text| text.starts_with("Attention Focus - License")),
                    "{language:?} shows the start of the text"
                );
            });
        }
    }
}
