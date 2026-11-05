//! Interface languages. `locales/en.json` is the source text and the fallback for anything missing.
//!
//! Text comes in two forms. Logical text is in reading order: it is what gets stored, and what
//! Windows draws itself, such as the tray menu. Painted text is what egui needs, which differs only
//! in right-to-left languages; see `bidi`. `t` and `tf` return painted single lines. Use `message`
//! for logical text, and `paragraph` for anything that can wrap.
use crate::bidi;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{
        OnceLock,
        atomic::{AtomicU8, Ordering},
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Language {
    English,
    Russian,
    German,
    French,
    Spanish,
    PortugueseBrazil,
    Italian,
    Polish,
    Turkish,
    ChineseSimplified,
    Japanese,
    Korean,
    Arabic,
}
impl Language {
    /// In discriminant order, which also indexes the tables below.
    pub const ALL: [Self; 13] = [
        Self::English,
        Self::Russian,
        Self::German,
        Self::French,
        Self::Spanish,
        Self::PortugueseBrazil,
        Self::Italian,
        Self::Polish,
        Self::Turkish,
        Self::ChineseSimplified,
        Self::Japanese,
        Self::Korean,
        Self::Arabic,
    ];
    pub fn is_rtl(self) -> bool {
        self == Self::Arabic
    }
    /// In character table order, by the name each language has in its own script.
    pub fn in_character_order() -> [Self; 13] {
        let mut languages = Self::ALL;
        languages.sort_by_key(|language| native_name(*language));
        languages
    }
    fn source(self) -> &'static str {
        match self {
            Self::English => include_str!("../locales/en.json"),
            Self::Russian => include_str!("../locales/ru.json"),
            Self::German => include_str!("../locales/de.json"),
            Self::French => include_str!("../locales/fr.json"),
            Self::Spanish => include_str!("../locales/es.json"),
            Self::PortugueseBrazil => include_str!("../locales/pt-BR.json"),
            Self::Italian => include_str!("../locales/it.json"),
            Self::Polish => include_str!("../locales/pl.json"),
            Self::Turkish => include_str!("../locales/tr.json"),
            Self::ChineseSimplified => include_str!("../locales/zh-Hans.json"),
            Self::Japanese => include_str!("../locales/ja.json"),
            Self::Korean => include_str!("../locales/ko.json"),
            Self::Arabic => include_str!("../locales/ar.json"),
        }
    }
    /// Maps a Windows language name such as `ru-RU`. Traditional Chinese is not supported, and
    /// showing it Simplified would be wrong for its readers, so it counts as unsupported.
    pub fn from_locale_name(name: &str) -> Option<Self> {
        let name = name.to_ascii_lowercase();
        let mut parts = name.split(['-', '_']);
        Some(match parts.next()? {
            "en" => Self::English,
            "ru" => Self::Russian,
            "de" => Self::German,
            "fr" => Self::French,
            "es" => Self::Spanish,
            "pt" => Self::PortugueseBrazil,
            "it" => Self::Italian,
            "pl" => Self::Polish,
            "tr" => Self::Turkish,
            "ja" => Self::Japanese,
            "ko" => Self::Korean,
            "ar" => Self::Arabic,
            "zh" if !parts.any(|part| matches!(part, "hant" | "tw" | "hk" | "mo")) => {
                Self::ChineseSimplified
            }
            _ => return None,
        })
    }
    /// The plural category of a count, following the Unicode CLDR rules for whole numbers.
    fn plural(self, count: u64) -> &'static str {
        let (tens, hundreds) = (count % 10, count % 100);
        let few = (2..=4).contains(&tens) && !(12..=14).contains(&hundreds);
        match self {
            Self::ChineseSimplified | Self::Japanese | Self::Korean => "other",
            Self::French | Self::PortugueseBrazil if count <= 1 => "one",
            Self::French | Self::PortugueseBrazil => "other",
            Self::Russian if tens == 1 && hundreds != 11 => "one",
            Self::Russian if few => "few",
            Self::Russian => "many",
            Self::Polish if count == 1 => "one",
            Self::Polish if few => "few",
            Self::Polish => "many",
            Self::Arabic => match (count, hundreds) {
                (0, _) => "zero",
                (1, _) => "one",
                (2, _) => "two",
                (_, 3..=10) => "few",
                (_, 11..=99) => "many",
                _ => "other",
            },
            _ if count == 1 => "one",
            _ => "other",
        }
    }
}

/// What the user chose. Same as Windows is the default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Choice {
    #[default]
    System,
    Fixed(Language),
}
/// The first supported language in the Windows preference order, or else English.
pub fn resolve(choice: Choice, preferred: &[String]) -> Language {
    match choice {
        Choice::Fixed(language) => language,
        Choice::System => preferred
            .iter()
            .find_map(|name| Language::from_locale_name(name))
            .unwrap_or(Language::English),
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);
#[cfg(test)]
thread_local! {
    static TEST_LANGUAGE: std::cell::Cell<Option<Language>> = const { std::cell::Cell::new(None) };
}

pub fn set_language(language: Language) {
    CURRENT.store(language as u8, Ordering::Relaxed);
}
pub fn current() -> Language {
    #[cfg(test)]
    if let Some(language) = TEST_LANGUAGE.get() {
        return language;
    }
    Language::ALL[usize::from(CURRENT.load(Ordering::Relaxed))]
}
/// Tests run in parallel, so they choose a language for their own thread only.
#[cfg(test)]
pub(crate) fn with_language<R>(language: Language, run: impl FnOnce() -> R) -> R {
    TEST_LANGUAGE.set(Some(language));
    let result = run();
    TEST_LANGUAGE.set(None);
    result
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Entry {
    Text(String),
    Plural(HashMap<String, String>),
}
#[derive(Default)]
struct Table {
    entries: HashMap<String, Entry>,
    /// Plain texts as egui must receive them, for right-to-left languages only.
    painted: HashMap<String, String>,
}
fn table(language: Language) -> &'static Table {
    static TABLES: [OnceLock<Table>; Language::ALL.len()] =
        [const { OnceLock::new() }; Language::ALL.len()];
    TABLES[language as usize].get_or_init(|| {
        // A test checks that every file parses, so a release cannot reach the empty default.
        let mut table = Table {
            entries: serde_json::from_str(language.source()).unwrap_or_default(),
            painted: HashMap::new(),
        };
        if language.is_rtl() {
            table.painted = table
                .entries
                .iter()
                .filter_map(|(key, entry)| match entry {
                    Entry::Text(text) => Some((key.clone(), bidi::right_to_left(text))),
                    Entry::Plural(_) => None,
                })
                .collect();
        }
        table
    })
}
fn entry(language: Language, key: &str) -> Option<&'static Entry> {
    table(language)
        .entries
        .get(key)
        .or_else(|| table(Language::English).entries.get(key))
}
/// Logical text of a key, or the key itself, which a test keeps out of releases.
fn logical(language: Language, key: &'static str) -> &'static str {
    match entry(language, key) {
        Some(Entry::Text(text)) => text,
        Some(Entry::Plural(forms)) => forms.get("other").map_or(key, String::as_str),
        None => key,
    }
}
fn fill(template: &str, arguments: &[(&str, &str)]) -> String {
    let rtl = current().is_rtl();
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let value = after.find('}').and_then(|end| {
            let name = &after[..end];
            let value = arguments.iter().find(|(key, _)| *key == name)?.1;
            Some((value, end))
        });
        if let Some((value, end)) = value {
            if rtl {
                out.push_str(&bidi::isolated(value));
            } else {
                out.push_str(value);
            }
            rest = &after[end + 1..];
        } else {
            out.push('{');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// Painted text of a key without placeholders, for one line of the interface.
pub fn t(key: &'static str) -> &'static str {
    let language = current();
    if language.is_rtl()
        && let Some(painted) = table(language).painted.get(key)
    {
        return painted;
    }
    logical(language, key)
}
/// Logical text with placeholders filled in: for storing, for Windows to draw, and for `paragraph`.
pub fn message(key: &'static str, arguments: &[(&str, &str)]) -> String {
    fill(logical(current(), key), arguments)
}
/// Logical text of a form chosen by `count`, which also fills `{count}`.
pub fn message_plural(key: &'static str, count: u64, arguments: &[(&str, &str)]) -> String {
    let language = current();
    let template = match entry(language, key) {
        Some(Entry::Plural(forms)) => forms
            .get(language.plural(count))
            .or_else(|| forms.get("other"))
            .map_or(key, String::as_str),
        Some(Entry::Text(text)) => text,
        None => key,
    };
    let count = count.to_string();
    let mut all = vec![("count", count.as_str())];
    all.extend_from_slice(arguments);
    fill(template, &all)
}
/// Painted text with placeholders filled in, for one line of the interface.
pub fn tf(key: &'static str, arguments: &[(&str, &str)]) -> String {
    paint(&message(key, arguments)).into_owned()
}
/// Painted text of a form chosen by `count`, for one line of the interface.
pub fn tp(key: &'static str, count: u64, arguments: &[(&str, &str)]) -> String {
    paint(&message_plural(key, count, arguments)).into_owned()
}
/// Logical interface text, such as a stored message, as egui must receive one line of it.
pub fn paint(text: &str) -> Cow<'_, str> {
    if current().is_rtl() {
        Cow::Owned(bidi::right_to_left(text))
    } else {
        bidi::left_to_right(text)
    }
}
/// A project name, a task or a note, in any language and any interface language.
pub fn user(text: &str) -> Cow<'_, str> {
    bidi::user_text(text)
}
/// The language's name in that language, in logical order.
pub fn native_name(language: Language) -> &'static str {
    logical(language, "language.native")
}

/// The contents of the language list, returning whether the choice changed. Following Windows is
/// not a language, so it stands apart from them, styled like the application's other special entry.
pub fn language_menu(ui: &mut egui::Ui, choice: &mut Choice, accent: egui::Color32) -> bool {
    let same_as_windows = egui::RichText::new(t("language.same_as_windows"))
        .strong()
        .italics()
        .color(accent);
    let mut changed = ui
        .selectable_value(choice, Choice::System, same_as_windows)
        .changed();
    ui.separator();
    for language in Language::in_character_order() {
        changed |= ui
            .selectable_value(
                choice,
                Choice::Fixed(language),
                user(native_name(language)).into_owned(),
            )
            .changed();
    }
    changed
}

/// Logical text as a label that wraps. egui's own wrapping works for left-to-right languages, and
/// otherwise lines must be chosen before their words are reordered.
pub fn paragraph(ui: &mut egui::Ui, text: &str) -> egui::Response {
    paragraph_styled(ui, text, |text| text)
}
pub fn paragraph_colored(ui: &mut egui::Ui, color: egui::Color32, text: &str) -> egui::Response {
    paragraph_styled(ui, text, |text| text.color(color))
}
fn paragraph_styled(
    ui: &mut egui::Ui,
    text: &str,
    style: impl FnOnce(egui::RichText) -> egui::RichText,
) -> egui::Response {
    if !current().is_rtl() {
        return ui.label(style(egui::RichText::new(text)));
    }
    let painted = painted_lines(ui, text, ui.available_width());
    ui.add(egui::Label::new(style(egui::RichText::new(painted))).extend())
}
/// Right-to-left text broken into lines of at most this width, ready to paint.
fn painted_lines(ui: &egui::Ui, text: &str, width: f32) -> String {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let lines = bidi::wrap(text, width, |line| {
        ui.painter()
            .layout_no_wrap(
                bidi::right_to_left(line),
                font.clone(),
                egui::Color32::PLACEHOLDER,
            )
            .size()
            .x
    });
    let painted: Vec<String> = lines.iter().map(|line| bidi::right_to_left(line)).collect();
    painted.join("\n")
}
/// The label of a checkbox that can need more than one line. egui wraps it in other languages.
pub fn checkbox_label(ui: &egui::Ui, key: &'static str) -> String {
    let language = current();
    if !language.is_rtl() {
        return t(key).to_owned();
    }
    let spacing = ui.spacing();
    let width = ui.available_width() - spacing.icon_width - spacing.icon_spacing;
    painted_lines(ui, logical(language, key), width)
}
/// egui resets collapsing sections to left-aligned rows, so their contents ask for mirroring again.
pub fn mirrored<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    if current().is_rtl() {
        ui.with_layout(egui::Layout::top_down(egui::Align::Max), add)
            .inner
    } else {
        add(ui)
    }
}
/// A grid works in unlimited width, which a mirrored layout turns into an invalid position.
pub fn unmirrored<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    if current().is_rtl() {
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), add)
            .inner
    } else {
        add(ui)
    }
}
/// A balloon with logical text, while the pointer rests on the widget or the keyboard focus is
/// on it. A click does not give a button focus, so a balloon does not stay after one.
pub fn tooltip(response: egui::Response, text: &str) -> egui::Response {
    if response.has_focus() {
        egui::containers::Tooltip::for_widget(&response)
            .show(|ui| crate::ui_support::balloon(ui, text));
        response
    } else {
        response.on_hover_ui(|ui| crate::ui_support::balloon(ui, text))
    }
}

/// Windows counts weekdays from Monday here, matching `reports`.
const WEEKDAYS: [&str; 7] = [
    "date.weekday.mon",
    "date.weekday.tue",
    "date.weekday.wed",
    "date.weekday.thu",
    "date.weekday.fri",
    "date.weekday.sat",
    "date.weekday.sun",
];
const MONTHS: [&str; 12] = [
    "date.month.jan",
    "date.month.feb",
    "date.month.mar",
    "date.month.apr",
    "date.month.may",
    "date.month.jun",
    "date.month.jul",
    "date.month.aug",
    "date.month.sep",
    "date.month.oct",
    "date.month.nov",
    "date.month.dec",
];
/// A short weekday name, Monday first, in logical order.
pub fn weekday_name(monday_based: usize) -> &'static str {
    logical(current(), WEEKDAYS[monday_based % 7])
}
pub fn painted_weekday_name(monday_based: usize) -> &'static str {
    t(WEEKDAYS[monday_based % 7])
}
/// A short month name, January first, in logical order.
pub fn month_name(zero_based: usize) -> &'static str {
    logical(current(), MONTHS[zero_based % 12])
}
/// A date such as `Mon 26 Oct`, in logical order.
pub fn long_date(weekday: usize, day: i8, month: usize) -> String {
    message(
        "date.long",
        &[
            ("weekday", weekday_name(weekday)),
            ("day", &day.to_string()),
            ("month", month_name(month)),
        ],
    )
}
/// A date such as `Mon 26`, in logical order.
pub fn short_date(weekday: usize, day: i8) -> String {
    message(
        "date.short",
        &[
            ("weekday", weekday_name(weekday)),
            ("day", &day.to_string()),
        ],
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn placeholders(text: &str) -> Vec<&str> {
        let mut names: Vec<&str> = text
            .split('{')
            .skip(1)
            .filter_map(|part| part.split_once('}').map(|(name, _)| name))
            .collect();
        names.sort_unstable();
        names
    }
    pub(crate) fn all_texts(language: Language) -> Vec<&'static str> {
        table(language)
            .entries
            .values()
            .flat_map(|entry| match entry {
                Entry::Text(text) => vec![text.as_str()],
                Entry::Plural(forms) => forms.values().map(String::as_str).collect(),
            })
            .collect()
    }

    #[test]
    fn every_language_has_every_key_its_placeholders_and_its_plural_forms() {
        for language in Language::ALL {
            let parsed: HashMap<String, Entry> = serde_json::from_str(language.source())
                .unwrap_or_else(|error| panic!("{language:?}: {error}"));
            assert_eq!(
                parsed.len(),
                table(Language::English).entries.len(),
                "{language:?} has missing or unknown keys"
            );
            for (key, source) in &table(Language::English).entries {
                let target = parsed
                    .get(key)
                    .unwrap_or_else(|| panic!("{language:?} lacks {key}"));
                match (source, target) {
                    (Entry::Text(source), Entry::Text(target)) => {
                        assert_eq!(
                            placeholders(source),
                            placeholders(target),
                            "{language:?} {key}"
                        );
                        assert!(!target.trim().is_empty(), "{language:?} {key}");
                    }
                    (Entry::Plural(source), Entry::Plural(target)) => {
                        fn without_count(text: &str) -> Vec<&str> {
                            let mut names = placeholders(text);
                            names.retain(|name| *name != "count");
                            names.dedup();
                            names
                        }
                        let wanted = without_count(&source["other"]);
                        for count in 0..=1200 {
                            let form = language.plural(count);
                            let text = target
                                .get(form)
                                .unwrap_or_else(|| panic!("{language:?} {key} lacks {form}"));
                            assert_eq!(without_count(text), wanted, "{language:?} {key} {form}");
                        }
                    }
                    _ => panic!("{language:?} {key} has the wrong shape"),
                }
            }
        }
    }

    #[test]
    fn keys_in_the_code_and_in_the_english_file_match() {
        fn read_sources(directory: &std::path::Path, out: &mut String) {
            for entry in std::fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    read_sources(&path, out);
                } else if path.extension().is_some_and(|extension| extension == "rs")
                    && path.file_name().is_some_and(|name| name != "tests.rs")
                {
                    out.push_str(&std::fs::read_to_string(path).unwrap());
                }
            }
        }
        let mut source = String::new();
        read_sources(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut source,
        );
        let english = &table(Language::English).entries;
        for key in english.keys() {
            assert!(source.contains(&format!("\"{key}\"")), "{key} is not used");
        }
        const AREAS: [&str; 16] = [
            "language", "main", "activity", "settings", "startup", "tray", "counters", "notes",
            "projects", "reports", "charts", "calendar", "date", "units", "export", "errors",
        ];
        for literal in source.split('"').skip(1).step_by(2) {
            let is_key = literal.split_once('.').is_some_and(|(area, rest)| {
                AREAS.contains(&area)
                    && !rest.is_empty()
                    && literal.chars().all(|c| {
                        c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_'
                    })
            });
            if is_key {
                assert!(english.contains_key(literal), "{literal} is not in en.json");
            }
        }
    }

    #[test]
    fn bundled_fonts_cover_the_languages_that_rely_on_them() {
        let context = egui::Context::default();
        let mut output = context.run_ui(egui::RawInput::default(), |_| {});
        output.textures_delta.clear();
        let font = egui::FontId::proportional(14.0);
        for language in [
            Language::English,
            Language::Russian,
            Language::German,
            Language::French,
            Language::Spanish,
            Language::PortugueseBrazil,
            Language::Italian,
            Language::Polish,
            Language::Turkish,
        ] {
            for text in all_texts(language) {
                for c in text.chars() {
                    let found = context.fonts_mut(|fonts| fonts.has_glyph(&font, c));
                    assert!(found, "{language:?} {c:?} in {text}");
                }
            }
        }
    }

    #[test]
    fn the_language_list_sets_windows_apart_and_follows_the_character_table() {
        assert_eq!(
            Language::in_character_order(),
            [
                Language::German,
                Language::English,
                Language::Spanish,
                Language::French,
                Language::Italian,
                Language::Polish,
                Language::PortugueseBrazil,
                Language::Turkish,
                Language::Russian,
                Language::Arabic,
                Language::Japanese,
                Language::ChineseSimplified,
                Language::Korean,
            ]
        );
        let context = egui::Context::default();
        let accent = egui::Color32::from_rgb(1, 2, 3);
        let mut choice = Choice::Fixed(Language::German);
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            assert!(!language_menu(ui, &mut choice, accent));
        });
        output.textures_delta.clear();
        let mut rows = Vec::new();
        let mut line = None;
        for shape in &output.shapes {
            match &shape.shape {
                egui::Shape::Text(text) => rows.push((
                    text.galley.job.text.clone(),
                    text.galley.job.sections[0].format.color,
                    text.visual_bounding_rect(),
                )),
                egui::Shape::LineSegment { points, .. } => line = Some(points[0].y),
                _ => {}
            }
        }
        assert_eq!(rows.len(), 14, "Windows and every language");
        assert_eq!(rows[0].0, "Same as Windows");
        assert_eq!(rows[0].1, accent, "it looks different from the languages");
        assert!(rows[1..].iter().all(|row| row.1 != accent));
        assert_eq!(rows[1].0, "Deutsch");
        assert_eq!(rows[2].0, "English");
        let line = line.expect("a line separates it from the languages");
        assert!(rows[0].2.bottom() <= line && line <= rows[1].2.top());
        assert!(
            rows.windows(2)
                .all(|pair| pair[0].2.top() < pair[1].2.top())
        );
    }
    #[test]
    fn windows_language_names_map_to_supported_languages() {
        for (name, expected) in [
            ("en-US", Some(Language::English)),
            ("ru-RU", Some(Language::Russian)),
            ("DE-at", Some(Language::German)),
            ("pt-PT", Some(Language::PortugueseBrazil)),
            ("pt-BR", Some(Language::PortugueseBrazil)),
            ("zh-CN", Some(Language::ChineseSimplified)),
            ("zh-Hans-SG", Some(Language::ChineseSimplified)),
            ("zh-TW", None),
            ("zh-Hant-HK", None),
            ("ar-SA", Some(Language::Arabic)),
            ("uk-UA", None),
            ("", None),
        ] {
            assert_eq!(Language::from_locale_name(name), expected, "{name}");
        }
    }
    #[test]
    fn the_system_language_falls_back_along_windows_preferences_to_english() {
        let names = |list: &[&str]| list.iter().map(|n| (*n).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            resolve(Choice::System, &names(&["ru-RU", "en-US"])),
            Language::Russian
        );
        assert_eq!(
            resolve(Choice::System, &names(&["uk-UA", "de-DE"])),
            Language::German
        );
        assert_eq!(
            resolve(Choice::System, &names(&["uk-UA"])),
            Language::English
        );
        assert_eq!(resolve(Choice::System, &[]), Language::English);
        assert_eq!(
            resolve(Choice::Fixed(Language::Korean), &names(&["ru-RU"])),
            Language::Korean
        );
    }
    #[test]
    fn plural_categories_follow_the_rules_of_each_language() {
        let forms = |language: Language, counts: &[u64]| -> Vec<&str> {
            counts.iter().map(|&n| language.plural(n)).collect()
        };
        assert_eq!(
            forms(Language::English, &[0, 1, 2]),
            ["other", "one", "other"]
        );
        assert_eq!(forms(Language::French, &[0, 1, 2]), ["one", "one", "other"]);
        assert_eq!(
            forms(Language::PortugueseBrazil, &[0, 1, 2]),
            ["one", "one", "other"]
        );
        assert_eq!(
            forms(Language::Russian, &[1, 2, 5, 11, 12, 21, 22, 25, 101, 111]),
            [
                "one", "few", "many", "many", "many", "one", "few", "many", "one", "many"
            ]
        );
        assert_eq!(
            forms(Language::Polish, &[1, 2, 5, 12, 21, 22, 25]),
            ["one", "few", "many", "many", "many", "few", "many"]
        );
        assert_eq!(
            forms(Language::Japanese, &[0, 1, 2]),
            ["other", "other", "other"]
        );
        assert_eq!(
            forms(Language::Arabic, &[0, 1, 2, 3, 10, 11, 99, 100, 103, 111]),
            [
                "zero", "one", "two", "few", "few", "many", "many", "other", "few", "many"
            ]
        );
    }
    #[test]
    fn placeholders_fill_in_one_pass_and_leave_unknown_names_alone() {
        assert_eq!(fill("{a} and {b}", &[("a", "1"), ("b", "2")]), "1 and 2");
        assert_eq!(fill("{a}", &[("a", "{b}"), ("b", "2")]), "{b}");
        assert_eq!(fill("{unknown} {a}", &[("a", "1")]), "{unknown} 1");
        assert_eq!(fill("open { only", &[]), "open { only");
    }
    #[test]
    fn right_to_left_interface_text_is_painted_and_stored_text_stays_logical() {
        with_language(Language::Arabic, || {
            let logical_text = message("language.native", &[]);
            assert!(!logical_text.is_empty());
            assert_eq!(paint(&logical_text), bidi::right_to_left(&logical_text));
        });
        assert_eq!(paint("Plain text"), "Plain text");
    }
    #[test]
    fn a_missing_key_shows_itself_rather_than_nothing() {
        assert_eq!(t("no.such.key"), "no.such.key");
    }
    #[test]
    fn a_test_language_stays_on_its_own_thread() {
        with_language(Language::Korean, || {
            assert_eq!(current(), Language::Korean);
            std::thread::spawn(|| assert_eq!(current(), Language::English))
                .join()
                .unwrap();
        });
        assert_eq!(current(), Language::English);
    }
}
