//! Fonts for scripts the bundled ones do not cover, read from Windows only when something needs
//! them. Together they are tens of megabytes, so the application does not ship or preload them.
use crate::i18n::Language;
use eframe::egui;
use std::{collections::BTreeSet, path::PathBuf, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Script {
    Arabic,
    Chinese,
    Japanese,
    Korean,
}
impl Script {
    /// Fonts that Windows includes, best first. Arabic comes from a font that also has spaces, but
    /// it is only ever a fallback, so `bidi` can rely on spaces forming separate runs.
    ///
    /// Each entry also names the face within the file. Japanese uses the UI faces of Yu Gothic:
    /// its plain faces have a line gap of half an em, which lifts them off the shared baseline.
    fn files(self) -> &'static [(&'static str, u32)] {
        match self {
            Self::Arabic => &[("segoeui.ttf", 0), ("tahoma.ttf", 0), ("arial.ttf", 0)],
            Self::Chinese => &[("msyh.ttc", 0), ("simsun.ttc", 0)],
            Self::Japanese => &[
                ("YuGothM.ttc", 1),
                ("YuGothR.ttc", 1),
                ("meiryo.ttc", 0),
                ("msgothic.ttc", 2),
            ],
            Self::Korean => &[("malgun.ttf", 0), ("gulim.ttc", 0)],
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Arabic => "system-arabic",
            Self::Chinese => "system-chinese",
            Self::Japanese => "system-japanese",
            Self::Korean => "system-korean",
        }
    }
    fn of_language(language: Language) -> Option<Self> {
        match language {
            Language::Arabic => Some(Self::Arabic),
            Language::ChineseSimplified => Some(Self::Chinese),
            Language::Japanese => Some(Self::Japanese),
            Language::Korean => Some(Self::Korean),
            _ => None,
        }
    }
    /// The script that should draw a character. Chinese characters follow the interface language,
    /// because the two languages draw some of them differently.
    fn of_char(c: char, language: Language) -> Option<Self> {
        match c as u32 {
            0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF => Some(Self::Arabic),
            0x3040..=0x30FF | 0x31F0..=0x31FF | 0xFF66..=0xFF9F => Some(Self::Japanese),
            0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xAC00..=0xD7FF => {
                Some(Self::Korean)
            }
            0x2E80..=0x2FDF
            | 0x3000..=0x303F
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0xFF00..=0xFF65
            | 0x20000..=0x3FFFF => Some(if language == Language::Japanese {
                Self::Japanese
            } else {
                Self::Chinese
            }),
            _ => None,
        }
    }
}

/// Tracks which scripts are needed, and adds their fonts once.
pub struct Fallbacks {
    directory: PathBuf,
    language: Language,
    wanted: BTreeSet<Script>,
    loaded: Vec<(Script, Arc<egui::FontData>)>,
    /// Scripts already tried, so a missing font is not read from disk on every frame.
    tried: BTreeSet<Script>,
    applied: Vec<Script>,
}
impl Fallbacks {
    pub fn new(language: Language) -> Self {
        let directory = std::env::var_os("WINDIR")
            .map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from)
            .join("Fonts");
        Self::in_directory(directory, language)
    }
    fn in_directory(directory: PathBuf, language: Language) -> Self {
        let mut fallbacks = Self {
            directory,
            language,
            wanted: BTreeSet::new(),
            loaded: Vec::new(),
            tried: BTreeSet::new(),
            applied: Vec::new(),
        };
        fallbacks.set_language(language);
        fallbacks
    }
    pub fn set_language(&mut self, language: Language) {
        self.language = language;
        self.wanted.extend(Script::of_language(language));
    }
    /// User data can be in any script, whatever the interface language.
    pub fn require_text(&mut self, text: &str) {
        for c in text.chars().filter(|c| !c.is_ascii()) {
            if let Some(script) = Script::of_char(c, self.language) {
                self.wanted.insert(script);
            }
        }
    }
    /// The language list names each language in its own script.
    pub fn require_language_names(&mut self) {
        self.wanted.extend(
            Language::ALL
                .iter()
                .copied()
                .filter_map(Script::of_language),
        );
    }
    fn read(&self, script: Script) -> Option<Arc<egui::FontData>> {
        script.files().iter().find_map(|(file, face)| {
            let bytes = std::fs::read(self.directory.join(file)).ok()?;
            let mut data = egui::FontData::from_owned(bytes);
            data.index = *face;
            Some(Arc::new(data))
        })
    }
    /// The interface language's own script comes first among the additions.
    fn order(&self) -> Vec<Script> {
        let own = Script::of_language(self.language);
        let mut order: Vec<Script> = self
            .loaded
            .iter()
            .map(|(script, _)| *script)
            .filter(|script| Some(*script) != own)
            .collect();
        if let Some(own) = own
            && self.loaded.iter().any(|(script, _)| *script == own)
        {
            order.insert(0, own);
        }
        order
    }
    /// Reads what is newly needed and rebuilds egui's fonts only when something changed. Bundled
    /// fonts stay first, which `bidi` relies on.
    pub fn apply(&mut self, ctx: &egui::Context) {
        let missing: Vec<Script> = self.wanted.difference(&self.tried).copied().collect();
        for script in missing {
            self.tried.insert(script);
            if let Some(data) = self.read(script) {
                self.loaded.push((script, data));
            }
        }
        let order = self.order();
        if order == self.applied {
            return;
        }
        let mut definitions = egui::FontDefinitions::default();
        for script in &order {
            if let Some((_, data)) = self.loaded.iter().find(|(loaded, _)| loaded == script) {
                definitions
                    .font_data
                    .insert(script.name().to_owned(), data.clone());
                for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                    definitions
                        .families
                        .entry(family)
                        .or_default()
                        .push(script.name().to_owned());
                }
            }
        }
        ctx.set_fonts(definitions);
        self.applied = order;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidi;

    #[test]
    fn scripts_are_recognized_and_chinese_characters_follow_the_interface_language() {
        let of = |c, language| Script::of_char(c, language);
        assert_eq!(of('\u{645}', Language::English), Some(Script::Arabic));
        assert_eq!(of('\u{3042}', Language::English), Some(Script::Japanese));
        assert_eq!(of('\u{D55C}', Language::English), Some(Script::Korean));
        assert_eq!(of('\u{4E2D}', Language::English), Some(Script::Chinese));
        assert_eq!(of('\u{4E2D}', Language::Japanese), Some(Script::Japanese));
        assert_eq!(of('a', Language::Japanese), None);
        assert_eq!(
            of('\u{416}', Language::English),
            None,
            "Cyrillic is bundled"
        );
    }
    #[test]
    fn a_missing_font_is_tried_once_and_changes_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let mut fallbacks = Fallbacks::in_directory(directory.path().to_owned(), Language::Korean);
        let context = egui::Context::default();
        fallbacks.apply(&context);
        assert!(fallbacks.loaded.is_empty());
        assert!(fallbacks.tried.contains(&Script::Korean));
        assert!(fallbacks.applied.is_empty());
    }
    #[test]
    fn nothing_is_read_until_something_needs_it() {
        let mut fallbacks = Fallbacks::new(Language::German);
        fallbacks.require_text("Projekt f\u{FC}r M\u{E4}rz");
        assert!(fallbacks.wanted.is_empty());
        fallbacks.require_text("\u{D55C}");
        assert_eq!(fallbacks.wanted.len(), 1);
    }
    #[test]
    fn windows_fonts_cover_the_languages_that_rely_on_them() {
        for language in [
            Language::ChineseSimplified,
            Language::Japanese,
            Language::Korean,
            Language::Arabic,
        ] {
            let mut fallbacks = Fallbacks::new(language);
            if fallbacks
                .read(Script::of_language(language).unwrap())
                .is_none()
            {
                continue;
            }
            let context = egui::Context::default();
            fallbacks.apply(&context);
            let mut output = context.run_ui(egui::RawInput::default(), |_| {});
            output.textures_delta.clear();
            let font = egui::FontId::proportional(14.0);
            for text in crate::i18n::tests::all_texts(language) {
                for c in text.chars() {
                    let found = context.fonts_mut(|fonts| fonts.has_glyph(&font, c));
                    assert!(found, "{language:?} {c:?} in {text}");
                }
            }
        }
    }
    /// This checks egui itself, so that an upgrade which changes its layout fails here first.
    #[test]
    fn arabic_words_appear_from_right_to_left_on_screen() {
        const FIRST: &str = "\u{645}\u{631}\u{62d}\u{628}\u{627}";
        const SECOND: &str = "\u{639}\u{627}\u{644}\u{645}";
        let mut fallbacks = Fallbacks::new(Language::Arabic);
        if fallbacks.read(Script::Arabic).is_none() {
            return;
        }
        let context = egui::Context::default();
        fallbacks.apply(&context);
        let mut positions = None;
        // Fonts set on a context take effect from the next pass.
        for pass in 0..2 {
            let mut output = context.run_ui(
                egui::RawInput {
                    time: Some(f64::from(pass)),
                    ..Default::default()
                },
                |ui| {
                    let galley = ui.painter().layout_no_wrap(
                        bidi::right_to_left(&format!("{FIRST} {SECOND}")),
                        egui::FontId::proportional(14.0),
                        egui::Color32::WHITE,
                    );
                    let x_of = |letter: char| {
                        galley.rows[0]
                            .glyphs
                            .iter()
                            .find(|glyph| glyph.chr == letter)
                            .map(|glyph| glyph.pos.x)
                    };
                    positions = x_of('\u{62d}').zip(x_of('\u{639}'));
                },
            );
            output.textures_delta.clear();
        }
        let (in_first_word, in_second_word) = positions.unwrap();
        assert!(
            in_first_word > in_second_word,
            "the first word belongs on the right: {in_first_word} {in_second_word}"
        );
    }
}
