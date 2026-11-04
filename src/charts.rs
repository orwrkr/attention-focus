//! Graphical report views, drawn with the egui painter from the report's own numbers.
use crate::{
    presentation::hours_minutes,
    reports::{Chart, ChartDay, ChartSegment, ChartSeries, State},
    tracking::CatalogProject,
};
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, vec2};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Categorical colors in a fixed order chosen so neighbors stay distinct with color vision
/// differences. Each theme has its own validated steps of the same eight hues.
const LIGHT_SERIES: [Color32; 8] = [
    Color32::from_rgb(0x2a, 0x78, 0xd6),
    Color32::from_rgb(0xeb, 0x68, 0x34),
    Color32::from_rgb(0x1b, 0xaf, 0x7a),
    Color32::from_rgb(0xed, 0xa1, 0x00),
    Color32::from_rgb(0xe8, 0x7b, 0xa4),
    Color32::from_rgb(0x00, 0x83, 0x00),
    Color32::from_rgb(0x62, 0x50, 0xd6),
    Color32::from_rgb(0xe3, 0x49, 0x48),
];
const DARK_SERIES: [Color32; 8] = [
    Color32::from_rgb(0x39, 0x87, 0xe5),
    Color32::from_rgb(0xd9, 0x59, 0x26),
    Color32::from_rgb(0x19, 0x9e, 0x70),
    Color32::from_rgb(0xc9, 0x85, 0x00),
    Color32::from_rgb(0xd5, 0x51, 0x81),
    Color32::from_rgb(0x00, 0x83, 0x00),
    Color32::from_rgb(0x90, 0x85, 0xe9),
    Color32::from_rgb(0xe6, 0x67, 0x67),
];
/// One hue from near zero to the most time, with empty cells apart from the scale.
const LIGHT_RAMP: [Color32; 7] = [
    Color32::from_rgb(0xf0, 0xef, 0xec),
    Color32::from_rgb(0xcd, 0xe2, 0xfb),
    Color32::from_rgb(0x9e, 0xc5, 0xf4),
    Color32::from_rgb(0x6d, 0xa7, 0xec),
    Color32::from_rgb(0x39, 0x87, 0xe5),
    Color32::from_rgb(0x25, 0x6a, 0xbf),
    Color32::from_rgb(0x10, 0x42, 0x81),
];
const DARK_RAMP: [Color32; 7] = [
    Color32::from_rgb(0x24, 0x24, 0x22),
    Color32::from_rgb(0x18, 0x4f, 0x95),
    Color32::from_rgb(0x1c, 0x5c, 0xab),
    Color32::from_rgb(0x2a, 0x78, 0xd6),
    Color32::from_rgb(0x55, 0x98, 0xe7),
    Color32::from_rgb(0x86, 0xb6, 0xef),
    Color32::from_rgb(0xcd, 0xe2, 0xfb),
];
const UNASSIGNED: Color32 = Color32::from_rgb(0xa8, 0xa7, 0x9f);
const OTHER: Color32 = Color32::from_rgb(0x6f, 0x6e, 0x69);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum View {
    #[default]
    Timeline,
    Days,
    Rhythm,
}

/// Each project keeps its color as others come and go, so a period or filter never repaints
/// one. These are interface preferences; losing them only assigns the colors again.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectColors {
    slots: BTreeMap<i64, u8>,
}
impl ProjectColors {
    /// Give new projects the color used least by active ones, then by archived ones, so a
    /// restored project rarely collides. Returns whether anything changed.
    pub fn update(&mut self, projects: &[CatalogProject]) -> bool {
        let before = self.slots.len();
        self.slots
            .retain(|id, _| projects.iter().any(|project| project.id == *id));
        let mut changed = before != self.slots.len();
        let mut ordered: Vec<_> = projects.iter().collect();
        ordered.sort_by_key(|project| project.id);
        for project in ordered {
            if self.slots.contains_key(&project.id) {
                continue;
            }
            let (mut active, mut all) = ([0_u32; 8], [0_u32; 8]);
            for other in projects {
                if let Some(slot) = self.slot(other.id) {
                    all[slot] += 1;
                    active[slot] += u32::from(!other.archived);
                }
            }
            let slot = (0..8)
                .min_by_key(|&slot| (active[slot], all[slot]))
                .unwrap_or(0);
            self.slots.insert(project.id, slot as u8);
            changed = true;
        }
        changed
    }
    fn slot(&self, id: i64) -> Option<usize> {
        self.slots.get(&id).map(|&slot| usize::from(slot) % 8)
    }
}

/// The colors for one chart. Two projects can share a color only past eight active ones. The
/// larger keeps it and the smaller joins a neutral Other, so one color never names two.
struct Styles {
    colors: HashMap<Option<i64>, Color32>,
    titles: HashMap<Option<i64>, String>,
    legend: Vec<(Color32, String)>,
    other: usize,
}
impl Styles {
    fn new<'a>(
        series: impl IntoIterator<Item = &'a ChartSeries>,
        colors: &ProjectColors,
        dark: bool,
    ) -> Self {
        let palette = if dark { &DARK_SERIES } else { &LIGHT_SERIES };
        let mut used = [false; 8];
        let mut styles = Self {
            colors: HashMap::new(),
            titles: HashMap::new(),
            legend: Vec::new(),
            other: 0,
        };
        for series in series {
            styles.titles.insert(series.project, series.title.clone());
            let color = match series.project {
                None => Some(UNASSIGNED),
                Some(id) => colors.slot(id).filter(|&slot| !used[slot]).map(|slot| {
                    used[slot] = true;
                    palette[slot]
                }),
            };
            match color {
                Some(color) => {
                    styles.colors.insert(series.project, color);
                    styles.legend.push((color, series.title.clone()));
                }
                None => styles.other += 1,
            }
        }
        if styles.other > 0 {
            styles
                .legend
                .push((OTHER, format!("Other ({})", styles.other)));
        }
        styles
    }
    fn color(&self, project: Option<i64>) -> Color32 {
        self.colors.get(&project).copied().unwrap_or(OTHER)
    }
    fn title(&self, project: Option<i64>) -> &str {
        self.titles
            .get(&project)
            .map_or("Unassigned focus", String::as_str)
    }
}

#[derive(Clone, Copy)]
struct Ink {
    text: Color32,
    muted: Color32,
    grid: Color32,
    axis: Color32,
}
impl Ink {
    fn new(ui: &egui::Ui) -> Self {
        let visuals = ui.visuals();
        Self {
            text: visuals.strong_text_color(),
            // Axis labels stay at normal text contrast: the default dark text is already dim.
            muted: visuals.text_color(),
            grid: visuals.widgets.noninteractive.bg_stroke.color,
            axis: visuals.text_color().gamma_multiply(0.5),
        }
    }
}
fn small() -> FontId {
    FontId::proportional(12.0)
}
fn clock(minute: f32) -> String {
    let minute = minute.round().clamp(0.0, 1440.0) as u32;
    format!("{:02}:{:02}", minute / 60, minute % 60)
}
fn hour_label(hour: usize) -> String {
    format!("{hour:02}:00")
}
/// Narrower than this, egui paints an outline solid, and unfinished time would look finished.
const UNFINISHED_LEAST: f32 = 5.0;
fn least_width(state: State) -> f32 {
    if state == State::Saved {
        1.5
    } else {
        UNFINISHED_LEAST
    }
}

/// Pure geometry of the day timeline, so it can be checked without drawing.
struct Timeline {
    rect: Rect,
    labels: f32,
    totals: f32,
    top: f32,
    row: f32,
    bar: f32,
}
impl Timeline {
    const COMPACT_ROWS: usize = 14;

    fn height(rows: usize) -> f32 {
        let (bar, gap) = Self::bar_and_gap(rows);
        20.0 + (bar + gap) * rows as f32 + 2.0
    }
    fn bar_and_gap(rows: usize) -> (f32, f32) {
        match rows {
            0 | 1 => (24.0, 10.0),
            r if r <= Self::COMPACT_ROWS => (18.0, 10.0),
            _ => (11.0, 4.0),
        }
    }
    fn new(rect: Rect, rows: usize) -> Self {
        let (bar, gap) = Self::bar_and_gap(rows);
        Self {
            rect,
            labels: if rows > Self::COMPACT_ROWS {
                44.0
            } else {
                74.0
            },
            totals: 56.0,
            top: rect.top() + 20.0,
            row: bar + gap,
            bar,
        }
    }
    fn x(&self, minute: f32) -> f32 {
        let left = self.rect.left() + self.labels;
        left + minute / 1440.0 * (self.rect.width() - self.labels - self.totals).max(1.0)
    }
    fn row_top(&self, row: usize) -> f32 {
        self.top + row as f32 * self.row
    }
    /// One point of space on each side keeps touching sessions visibly separate.
    fn mark(&self, segment: &ChartSegment) -> Rect {
        let left = self.x(segment.start_minute) + 1.0;
        let right = (self.x(segment.end_minute) - 1.0).max(left + least_width(segment.state));
        let top = self.row_top(segment.day);
        Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, top + self.bar))
    }
    /// The target is larger than the mark, since a short session is only a sliver.
    fn at<'a>(&self, pointer: Pos2, segments: &'a [ChartSegment]) -> Option<&'a ChartSegment> {
        segments
            .iter()
            .filter(|segment| {
                let mark = self.mark(segment);
                pointer.y >= mark.top() - 2.0
                    && pointer.y <= mark.bottom() + 2.0
                    && pointer.x >= mark.left() - 3.0
                    && pointer.x <= mark.right() + 3.0
            })
            .min_by(|a, b| {
                let distance = |segment: &ChartSegment| {
                    let mark = self.mark(segment);
                    if pointer.x < mark.left() {
                        mark.left() - pointer.x
                    } else {
                        (pointer.x - mark.right()).max(0.0)
                    }
                };
                distance(a).total_cmp(&distance(b))
            })
    }
}

fn state_note(state: State) -> Option<&'static str> {
    match state {
        State::Saved => None,
        State::Running => Some("Running, not finished yet"),
        State::Review => Some("Needs review, not in totals"),
    }
}
/// Finished time is solid. Unfinished time keeps the project's hue in a visibly lighter form.
fn paint_mark(
    painter: &egui::Painter,
    rect: Rect,
    color: Color32,
    state: State,
    round: CornerRadius,
) {
    match state {
        State::Saved => {
            painter.rect_filled(rect, round, color);
        }
        State::Running => {
            painter.rect_filled(rect, round, color.gamma_multiply(0.45));
            painter.rect_stroke(
                rect,
                round,
                Stroke::new(1.0, color),
                egui::StrokeKind::Inside,
            );
        }
        State::Review => {
            painter.rect_stroke(
                rect,
                round,
                Stroke::new(1.0, color),
                egui::StrokeKind::Inside,
            );
        }
    }
}
fn outline_hovered(painter: &egui::Painter, rect: Rect, ink: Ink) {
    painter.rect_stroke(
        rect.expand(1.0),
        3,
        Stroke::new(1.5, ink.text),
        egui::StrokeKind::Outside,
    );
}

fn timeline(ui: &mut egui::Ui, chart: &Chart, styles: &Styles, ink: Ink) {
    let rows = chart.days.len();
    egui::ScrollArea::vertical()
        .id_salt("chart-timeline")
        .max_height(420.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let size = vec2(ui.available_width(), Timeline::height(rows));
            let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
            let layout = Timeline::new(rect, rows);
            let painter = ui.painter_at(rect);
            for hour in (0..=24).step_by(6) {
                let x = layout.x(hour as f32 * 60.0);
                painter.vline(
                    x,
                    (layout.top - 4.0)..=rect.bottom(),
                    Stroke::new(1.0, ink.grid),
                );
                let anchor = match hour {
                    0 => Align2::LEFT_TOP,
                    24 => Align2::RIGHT_TOP,
                    _ => Align2::CENTER_TOP,
                };
                painter.text(
                    egui::pos2(x, rect.top()),
                    anchor,
                    hour_label(hour),
                    small(),
                    ink.muted,
                );
            }
            let visible = ui.clip_rect();
            for (row, day) in chart.days.iter().enumerate() {
                let top = layout.row_top(row);
                if top > visible.bottom() || top + layout.row < visible.top() {
                    continue;
                }
                let middle = top + layout.bar / 2.0;
                let label = if rows > Timeline::COMPACT_ROWS {
                    &day.short_label
                } else {
                    &day.long_label
                };
                painter.text(
                    egui::pos2(rect.left(), middle),
                    Align2::LEFT_CENTER,
                    label,
                    small(),
                    ink.muted,
                );
                painter.text(
                    egui::pos2(rect.right(), middle),
                    Align2::RIGHT_CENTER,
                    hours_minutes(day.total_ms()),
                    small(),
                    ink.text,
                );
            }
            for segment in &chart.segments {
                let mark = layout.mark(segment);
                if mark.intersects(visible) {
                    let round = CornerRadius::same(if mark.width() >= 6.0 { 3 } else { 1 });
                    paint_mark(
                        &painter,
                        mark,
                        styles.color(segment.project),
                        segment.state,
                        round,
                    );
                }
            }
            if let Some(pointer) = response.hover_pos()
                && let Some(segment) = layout.at(pointer, &chart.segments)
            {
                outline_hovered(&painter, layout.mark(segment), ink);
                let day = &chart.days[segment.day];
                response.on_hover_ui_at_pointer(|ui| {
                    ui.strong(hours_minutes(segment.elapsed_ms));
                    ui.label(styles.title(segment.project));
                    if let Some(task) = &segment.task {
                        ui.label(task);
                    }
                    ui.label(format!(
                        "{}, {} to {}",
                        day.long_label,
                        clock(segment.start_minute),
                        clock(segment.end_clock)
                    ));
                    if let Some(note) = state_note(segment.state) {
                        ui.label(note);
                    }
                });
            }
        });
}

/// Pure geometry of the columns by day.
struct Columns {
    rect: Rect,
    left: f32,
    bottom: f32,
    top: f32,
    count: usize,
    scale_hours: u64,
}
impl Columns {
    const HEIGHT: f32 = 210.0;
    /// Room for the widest short date, such as Wed 30, with a small gap.
    const DATE_ROOM: f32 = 44.0;

    fn new(rect: Rect, count: usize, most_ms: u64) -> Self {
        let hours = (most_ms as f64 / 3_600_000.0 / 2.0).ceil().max(1.0) as u64 * 2;
        Self {
            rect,
            left: 36.0,
            bottom: 20.0,
            top: 16.0,
            count,
            scale_hours: hours,
        }
    }
    fn y(&self, ms: u64) -> f32 {
        let plot = self.rect.height() - self.bottom - self.top;
        self.rect.top() + self.top + plot
            - ms as f32 / (self.scale_hours as f32 * 3_600_000.0) * plot
    }
    /// Hours between grid lines, keeping a handful of lines at any scale.
    fn grid_step(&self) -> u64 {
        self.scale_hours.div_ceil(4).next_multiple_of(2)
    }
    /// Label every this many columns, so dates never collide at any width.
    fn date_stride(&self) -> usize {
        let fits = (Self::DATE_ROOM / self.band()).ceil() as usize;
        fits.max(if self.count <= 14 { 1 } else { 7 })
    }
    fn band(&self) -> f32 {
        (self.rect.width() - self.left) / self.count.max(1) as f32
    }
    fn center(&self, column: usize) -> f32 {
        self.rect.left() + self.left + self.band() * (column as f32 + 0.5)
    }
    fn width(&self) -> f32 {
        (self.band() * 0.62).clamp(2.0, 24.0)
    }
    fn column_at(&self, x: f32) -> Option<usize> {
        let offset = x - self.rect.left() - self.left;
        if offset < 0.0 {
            return None;
        }
        let column = (offset / self.band()) as usize;
        (column < self.count).then_some(column)
    }
    /// Stacked parts from the baseline up, largest project first, leaving a gap between them.
    fn stack(&self, column: usize, day: &ChartDay) -> Vec<(Option<i64>, State, Rect)> {
        let (center, half) = (self.center(column), self.width() / 2.0);
        let mut below = 0;
        let mut parts = Vec::new();
        let items = day
            .saved
            .iter()
            .map(|part| (part, State::Saved))
            .chain(day.running.iter().map(|part| (part, State::Running)));
        for (part, state) in items {
            let lower = self.y(below);
            below = below.saturating_add(part.ms);
            let upper = self.y(below);
            let gap = if parts.is_empty() { 0.0 } else { 2.0 };
            let least = if state == State::Saved {
                0.5
            } else {
                UNFINISHED_LEAST
            };
            let top = upper.min(lower - gap - least);
            parts.push((
                part.project,
                state,
                Rect::from_min_max(
                    egui::pos2(center - half, top),
                    egui::pos2(center + half, lower - gap),
                ),
            ));
        }
        parts
    }
}

fn day_details(ui: &mut egui::Ui, day: &ChartDay, styles: &Styles) {
    ui.strong(hours_minutes(day.total_ms()));
    ui.label(&day.long_label);
    for part in &day.saved {
        ui.label(format!(
            "{}: {}",
            styles.title(part.project),
            hours_minutes(part.ms)
        ));
    }
    if let Some(part) = &day.running {
        ui.label(format!(
            "{} (running): {}",
            styles.title(part.project),
            hours_minutes(part.ms)
        ));
    }
    if day.review_ms > 0 {
        ui.label(format!(
            "Needs review, not in totals: {}",
            hours_minutes(day.review_ms)
        ));
    }
}

fn columns(ui: &mut egui::Ui, chart: &Chart, styles: &Styles, ink: Ink) {
    if let [day] = chart.days.as_slice() {
        split_bar(ui, day, styles, ink);
        return;
    }
    let size = vec2(ui.available_width(), Columns::HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let most = chart.days.iter().map(ChartDay::total_ms).max().unwrap_or(0);
    let layout = Columns::new(rect, chart.days.len(), most);
    let painter = ui.painter_at(rect);
    for hour in (0..=layout.scale_hours).step_by(layout.grid_step() as usize) {
        let y = layout.y(hour.saturating_mul(3_600_000));
        let color = if hour == 0 { ink.axis } else { ink.grid };
        painter.hline(
            (rect.left() + layout.left)..=rect.right(),
            y,
            Stroke::new(1.0, color),
        );
        painter.text(
            egui::pos2(rect.left() + layout.left - 8.0, y),
            Align2::RIGHT_CENTER,
            format!("{hour}h"),
            small(),
            ink.muted,
        );
    }
    let band = layout.band();
    let stride = layout.date_stride();
    let first_most = chart.days.iter().position(|day| day.total_ms() == most);
    for (column, day) in chart.days.iter().enumerate() {
        let parts = layout.stack(column, day);
        let last = parts.len().saturating_sub(1);
        for (index, (project, state, mark)) in parts.iter().enumerate() {
            let round = if index == last {
                CornerRadius {
                    nw: 4,
                    ne: 4,
                    sw: 0,
                    se: 0,
                }
            } else {
                CornerRadius::ZERO
            };
            paint_mark(&painter, *mark, styles.color(*project), *state, round);
        }
        let total = day.total_ms();
        if total > 0 {
            // A total wider than its column would collide; the largest always shows.
            let label = painter.layout_no_wrap(hours_minutes(total), small(), ink.text);
            let size = label.size();
            if size.x + 2.0 <= band || Some(column) == first_most {
                let center = layout.center(column).min(rect.right() - size.x / 2.0);
                let stack_top = parts.last().map_or(layout.y(total), |part| part.2.top());
                let at = egui::pos2(center - size.x / 2.0, stack_top - 3.0 - size.y);
                painter.galley(at, label, ink.text);
            }
        }
        if column % stride == 0 {
            let label = painter.layout_no_wrap(day.short_label.clone(), small(), ink.muted);
            let size = label.size();
            let center = layout.center(column);
            if center + size.x / 2.0 <= rect.right() {
                let at = egui::pos2(center - size.x / 2.0, rect.bottom() - size.y);
                painter.galley(at, label, ink.muted);
            }
        }
    }
    if let Some(pointer) = response.hover_pos()
        && let Some(column) = layout.column_at(pointer.x)
    {
        let day = &chart.days[column];
        let parts = layout.stack(column, day);
        if let (Some(first), Some(last)) = (parts.first(), parts.last()) {
            let whole = Rect::from_min_max(
                egui::pos2(first.2.left(), last.2.top()),
                egui::pos2(first.2.right(), first.2.bottom()),
            );
            outline_hovered(&painter, whole, ink);
        }
        response.on_hover_ui_at_pointer(|ui| day_details(ui, day, styles));
    }
}

/// One part of the split bar, with a point of space on each side. A very small share still
/// gets a visible mark, kept inside the bar.
fn split_mark(outer: Rect, state: State, bar: Rect) -> Rect {
    let least = least_width(state);
    if outer.width() - 2.0 >= least {
        return outer.shrink2(vec2(1.0, 0.0));
    }
    let left = outer.left().min(bar.right() - least).max(bar.left());
    Rect::from_min_size(egui::pos2(left, bar.top()), vec2(least, bar.height()))
}

/// A single day has no other day to compare with, so it shows how the day was split.
fn split_bar(ui: &mut egui::Ui, day: &ChartDay, styles: &Styles, ink: Ink) {
    let total = day.total_ms();
    ui.label(format!("{}: {}", day.long_label, hours_minutes(total)));
    if day.review_ms > 0 {
        ui.small(format!(
            "Needs review, not in totals: {}",
            hours_minutes(day.review_ms)
        ));
    }
    if total == 0 {
        return;
    }
    let size = vec2(ui.available_width(), 28.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter_at(rect);
    let mut left = rect.left();
    let mut hovered = None;
    let items = day
        .saved
        .iter()
        .map(|part| (part, State::Saved))
        .chain(day.running.iter().map(|part| (part, State::Running)));
    for (part, state) in items {
        let width = part.ms as f32 / total as f32 * rect.width();
        let outer = Rect::from_min_size(egui::pos2(left, rect.top()), vec2(width, rect.height()));
        let mark = split_mark(outer, state, rect);
        paint_mark(
            &painter,
            mark,
            styles.color(part.project),
            state,
            CornerRadius::same(4),
        );
        let share = format!("{}%", (part.ms as f32 / total as f32 * 100.0).round());
        let label = painter.layout_no_wrap(share, small(), contrast_on(styles.color(part.project)));
        if state == State::Saved && label.size().x + 12.0 <= mark.width() {
            let at = egui::pos2(mark.left() + 6.0, mark.center().y - label.size().y / 2.0);
            painter.galley(at, label, ink.text);
        }
        if response
            .hover_pos()
            .is_some_and(|p| outer.union(mark).contains(p))
        {
            hovered = Some((part, mark));
        }
        left += width;
    }
    if let Some((part, mark)) = hovered {
        outline_hovered(&painter, mark, ink);
        response.on_hover_ui_at_pointer(|ui| {
            ui.strong(hours_minutes(part.ms));
            ui.label(styles.title(part.project));
            ui.label(format!(
                "{:.0}% of {}",
                part.ms as f32 / total as f32 * 100.0,
                day.long_label
            ));
        });
    }
}
/// Labels inside a fill are white, except on the light yellow of the palette.
fn contrast_on(fill: Color32) -> Color32 {
    if fill == LIGHT_SERIES[3] || fill == UNASSIGNED {
        Color32::from_rgb(0x0b, 0x0b, 0x0b)
    } else {
        Color32::WHITE
    }
}

/// The step of the single-hue scale for an amount of time in one cell.
fn rhythm_step(ms: u64, most: u64) -> usize {
    if ms == 0 || most == 0 {
        return 0;
    }
    ((ms as f64 / most as f64 * 6.0).ceil() as usize).clamp(1, 6)
}
const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// Pure geometry of the weekly rhythm.
struct Rhythm {
    rect: Rect,
    left: f32,
    top: f32,
    cell: Rect,
}
impl Rhythm {
    fn cell_height(width: f32) -> f32 {
        ((width - 36.0) / 24.0).clamp(16.0, 26.0)
    }
    fn height(width: f32, rows: usize) -> f32 {
        20.0 + Self::cell_height(width) * rows as f32 + 2.0
    }
    fn new(rect: Rect) -> Self {
        let left = 36.0;
        let height = Self::cell_height(rect.width());
        let width = (rect.width() - left) / 24.0;
        Self {
            rect,
            left,
            top: 20.0,
            cell: Rect::from_min_size(Pos2::ZERO, vec2(width, height)),
        }
    }
    fn cell_at(&self, row: usize, hour: usize) -> Rect {
        let origin = self.rect.min
            + vec2(
                self.left + hour as f32 * self.cell.width(),
                self.top + row as f32 * self.cell.height(),
            );
        Rect::from_min_size(origin, self.cell.size())
    }
    fn locate(&self, pointer: Pos2, rows: usize) -> Option<(usize, usize)> {
        let offset = pointer - self.rect.min - vec2(self.left, self.top);
        if offset.x < 0.0 || offset.y < 0.0 {
            return None;
        }
        let (row, hour) = (
            (offset.y / self.cell.height()) as usize,
            (offset.x / self.cell.width()) as usize,
        );
        (row < rows && hour < 24).then_some((row, hour))
    }
}

fn rhythm(ui: &mut egui::Ui, chart: &Chart, ink: Ink) {
    let dark = ui.visuals().dark_mode;
    let ramp = if dark { &DARK_RAMP } else { &LIGHT_RAMP };
    let rows: Vec<usize> = (0..7).filter(|&d| chart.weekday_counts[d] > 0).collect();
    let most = chart.rhythm.iter().flatten().copied().max().unwrap_or(0);
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(
        vec2(width, Rhythm::height(width, rows.len())),
        Sense::hover(),
    );
    let layout = Rhythm::new(rect);
    let painter = ui.painter_at(rect);
    for hour in (0..=24).step_by(6) {
        let x = rect.left() + layout.left + hour as f32 * layout.cell.width();
        let anchor = match hour {
            0 => Align2::LEFT_TOP,
            24 => Align2::RIGHT_TOP,
            _ => Align2::CENTER_TOP,
        };
        painter.text(
            egui::pos2(x, rect.top()),
            anchor,
            format!("{hour:02}"),
            small(),
            ink.muted,
        );
    }
    for (row, &weekday) in rows.iter().enumerate() {
        let first = layout.cell_at(row, 0);
        painter.text(
            egui::pos2(rect.left(), first.center().y),
            Align2::LEFT_CENTER,
            WEEKDAYS[weekday],
            small(),
            ink.muted,
        );
        for hour in 0..24 {
            let fill = ramp[rhythm_step(chart.rhythm[weekday][hour], most)];
            painter.rect_filled(layout.cell_at(row, hour).shrink(1.0), 2, fill);
        }
    }
    if let Some(pointer) = response.hover_pos()
        && let Some((row, hour)) = layout.locate(pointer, rows.len())
    {
        let weekday = rows[row];
        outline_hovered(&painter, layout.cell_at(row, hour).shrink(1.0), ink);
        response.on_hover_ui_at_pointer(|ui| {
            ui.strong(hours_minutes(chart.rhythm[weekday][hour]));
            ui.label(format!(
                "{}, {:02}:00 to {:02}:00",
                WEEKDAYS[weekday],
                hour,
                hour + 1
            ));
            let count = chart.weekday_counts[weekday];
            ui.label(format!(
                "Across {count} {}",
                if count == 1 { "day" } else { "days" }
            ));
        });
    }
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        ui.small("Less");
        for color in ramp {
            let (swatch, _) = ui.allocate_exact_size(vec2(16.0, 10.0), Sense::hover());
            ui.painter().rect_filled(swatch, 2, *color);
        }
        ui.small(format!("More (up to {} in one cell)", hours_minutes(most)));
    });
}

fn legend(ui: &mut egui::Ui, chart: &Chart, styles: &Styles, ink: Ink, view: View) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        for (color, title) in &styles.legend {
            let (swatch, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
            ui.painter().rect_filled(swatch, 2, *color);
            ui.small(title);
            ui.add_space(7.0);
        }
        let states = [(State::Running, "Running"), (State::Review, "Needs review")];
        for (state, label) in states {
            // By day leaves out review time, so it has no such mark to explain.
            let drawn = state != State::Review || view != View::Days;
            if drawn && chart.segments.iter().any(|segment| segment.state == state) {
                let (swatch, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                paint_mark(
                    ui.painter(),
                    swatch,
                    ink.muted,
                    state,
                    CornerRadius::same(2),
                );
                ui.small(label);
                ui.add_space(7.0);
            }
        }
    });
}

/// The selectable views, their shared legend and the chart itself.
pub fn show(ui: &mut egui::Ui, chart: &Chart, view: &mut View, colors: &ProjectColors) {
    ui.horizontal_wrapped(|ui| {
        for (value, label) in [
            (View::Timeline, "Timeline"),
            (View::Days, "By day"),
            (View::Rhythm, "Weekly rhythm"),
        ] {
            if ui
                .add(egui::Button::new(label).selected(*view == value))
                .clicked()
            {
                *view = value;
            }
        }
    });
    if chart.series.is_empty() {
        ui.label("No tracked time in this period.");
        return;
    }
    if chart.too_long && *view != View::Rhythm {
        ui.label(format!(
            "This view covers up to {} days. Choose a shorter period, or use Weekly rhythm.",
            crate::reports::CHART_MAX_DAYS
        ));
        return;
    }
    let ink = Ink::new(ui);
    let current = *view;
    match current {
        View::Timeline | View::Days => {
            // By day leaves out review time, so it does not name projects with only that.
            let in_columns: HashSet<Option<i64>> = chart
                .days
                .iter()
                .flat_map(|day| day.saved.iter().chain(&day.running))
                .map(|part| part.project)
                .collect();
            let series = chart
                .series
                .iter()
                .filter(|series| current == View::Timeline || in_columns.contains(&series.project));
            let styles = Styles::new(series, colors, ui.visuals().dark_mode);
            legend(ui, chart, &styles, ink, current);
            ui.add_space(4.0);
            if current == View::Timeline {
                timeline(ui, chart, &styles, ink);
            } else {
                columns(ui, chart, &styles, ink);
            }
        }
        View::Rhythm => rhythm(ui, chart, ink),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reports::ChartPart;

    fn catalog(ids: &[(i64, bool)]) -> Vec<CatalogProject> {
        ids.iter()
            .map(|&(id, archived)| CatalogProject {
                id,
                title: format!("Project {id}"),
                archived,
                used: true,
                tasks: Vec::new(),
            })
            .collect()
    }
    fn segment(day: usize, start_minute: f32, end_minute: f32) -> ChartSegment {
        ChartSegment {
            day,
            start_minute,
            end_minute,
            end_clock: end_minute,
            elapsed_ms: ((end_minute - start_minute) * 60_000.0) as u64,
            project: Some(1),
            task: None,
            state: State::Saved,
        }
    }
    fn sample_chart() -> Chart {
        let day = |long: &str, short: &str, alpha: u64, beta: u64| ChartDay {
            long_label: long.into(),
            short_label: short.into(),
            saved: vec![
                ChartPart {
                    project: Some(1),
                    ms: alpha,
                },
                ChartPart {
                    project: Some(2),
                    ms: beta,
                },
            ],
            running: None,
            review_ms: 0,
        };
        let mut rhythm = [[0; 24]; 7];
        rhythm[0][9] = 3_600_000;
        rhythm[1][14] = 1_800_000;
        Chart {
            days: vec![
                day("Mon 26 Oct", "Mon 26", 3_600_000, 1_800_000),
                day("Tue 27 Oct", "Tue 27", 7_200_000, 3_600_000),
            ],
            segments: vec![segment(0, 540.0, 600.0), segment(1, 840.0, 870.0)],
            series: vec![
                ChartSeries {
                    project: Some(1),
                    title: "Alpha".into(),
                    total_ms: 10_800_000,
                },
                ChartSeries {
                    project: Some(2),
                    title: "Beta".into(),
                    total_ms: 5_400_000,
                },
            ],
            rhythm,
            weekday_counts: [1, 1, 0, 0, 0, 0, 0],
            too_long: false,
        }
    }

    #[test]
    fn project_colors_stay_with_their_project_as_others_come_and_go() {
        let mut colors = ProjectColors::default();
        assert!(colors.update(&catalog(&[(3, false), (9, false)])));
        assert_eq!((colors.slot(3), colors.slot(9)), (Some(0), Some(1)));
        assert!(!colors.update(&catalog(&[(3, false), (9, false)])));
        // A new project does not disturb existing ones, and takes an unused color.
        colors.update(&catalog(&[(3, false), (9, false), (12, false)]));
        assert_eq!(
            (colors.slot(3), colors.slot(9), colors.slot(12)),
            (Some(0), Some(1), Some(2))
        );
        // An archived project keeps its color, and a new project avoids it while it can.
        colors.update(&catalog(&[(3, true), (9, false), (12, false), (20, false)]));
        assert_eq!(colors.slot(3), Some(0));
        assert_eq!(colors.slot(20), Some(3));
        // A deleted project is forgotten, and the rest keep their colors.
        assert!(colors.update(&catalog(&[(9, false), (12, false), (20, false)])));
        assert_eq!(colors.slot(3), None);
        assert_eq!(colors.slot(9), Some(1));
        let saved = serde_json::to_string(&colors).unwrap();
        assert_eq!(
            serde_json::from_str::<ProjectColors>(&saved).unwrap(),
            colors
        );
    }

    #[test]
    fn a_restored_color_out_of_range_cannot_crash_the_assignment() {
        let mut colors: ProjectColors = serde_json::from_str(r#"{"slots":{"3":9}}"#).unwrap();
        colors.update(&catalog(&[(3, false), (4, false)]));
        assert_eq!((colors.slot(3), colors.slot(4)), (Some(1), Some(0)));
    }

    #[test]
    fn archived_projects_spread_over_the_colors_and_do_not_crowd_active_ones() {
        // A first launch with archived projects shares nothing.
        let mut colors = ProjectColors::default();
        colors.update(&catalog(&[
            (1, true),
            (2, true),
            (3, true),
            (4, false),
            (5, false),
        ]));
        let slots: Vec<usize> = (1..=5).map(|id| colors.slot(id).unwrap()).collect();
        assert_eq!(slots, [0, 1, 2, 3, 4]);
        // Restoring a project cannot collide with one added while it was archived.
        let mut colors = ProjectColors::default();
        colors.update(&catalog(&[(1, false), (2, false)]));
        colors.update(&catalog(&[(1, true), (2, false), (3, false)]));
        colors.update(&catalog(&[(1, false), (2, false), (3, false)]));
        let slots: Vec<usize> = (1..=3).map(|id| colors.slot(id).unwrap()).collect();
        assert_eq!(slots, [0, 1, 2]);
        // With every color held, an archived project's is reused before an active one's.
        let mut colors = ProjectColors::default();
        let eight: Vec<(i64, bool)> = (1..=8).map(|id| (id, false)).collect();
        colors.update(&catalog(&eight));
        let mut next: Vec<(i64, bool)> = (1..=8).map(|id| (id, id == 5)).collect();
        next.push((9, false));
        colors.update(&catalog(&next));
        assert_eq!(colors.slot(9), colors.slot(5));
    }

    #[test]
    fn only_past_eight_active_projects_do_colors_repeat_and_then_one_joins_other() {
        let ids: Vec<(i64, bool)> = (1..=9).map(|id| (id, false)).collect();
        let mut colors = ProjectColors::default();
        colors.update(&catalog(&ids));
        let slots: Vec<usize> = (1..=8).map(|id| colors.slot(id).unwrap()).collect();
        assert_eq!(slots, [0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(colors.slot(9), Some(0), "the ninth has to share");
        // In a chart with both, the larger keeps the color and the smaller is Other.
        let series = |id: i64, total_ms| ChartSeries {
            project: Some(id),
            title: format!("Project {id}"),
            total_ms,
        };
        let chart = Chart {
            series: vec![
                series(9, 500),
                series(1, 100),
                ChartSeries {
                    project: None,
                    title: "Unassigned focus".into(),
                    total_ms: 50,
                },
            ],
            ..Chart::default()
        };
        let styles = Styles::new(&chart.series, &colors, false);
        assert_eq!(styles.color(Some(9)), LIGHT_SERIES[0]);
        assert_eq!(styles.color(Some(1)), OTHER);
        assert_eq!(styles.color(None), UNASSIGNED);
        assert_eq!(styles.other, 1);
        assert_eq!(styles.title(Some(1)), "Project 1", "hover still names it");
        let legend: Vec<&str> = styles.legend.iter().map(|(_, t)| t.as_str()).collect();
        assert_eq!(legend, ["Project 9", "Unassigned focus", "Other (1)"]);
    }

    #[test]
    fn dark_theme_uses_its_own_steps_of_the_same_order() {
        let mut colors = ProjectColors::default();
        colors.update(&catalog(&[(1, false)]));
        let chart = Chart {
            series: vec![ChartSeries {
                project: Some(1),
                title: "Alpha".into(),
                total_ms: 1,
            }],
            ..Chart::default()
        };
        assert_eq!(
            Styles::new(&chart.series, &colors, true).color(Some(1)),
            DARK_SERIES[0]
        );
    }

    #[test]
    fn timeline_places_the_day_on_a_midnight_to_midnight_axis() {
        let rect = Rect::from_min_size(egui::pos2(10.0, 20.0), vec2(700.0, 200.0));
        let layout = Timeline::new(rect, 7);
        assert_eq!(layout.x(0.0), 84.0);
        assert_eq!(layout.x(1440.0), 654.0);
        let morning = layout.mark(&segment(2, 720.0, 780.0));
        assert!((morning.left() - (layout.x(720.0) + 1.0)).abs() < 0.01);
        assert!((morning.right() - (layout.x(780.0) - 1.0)).abs() < 0.01);
        assert_eq!(morning.top(), layout.row_top(2));
        assert_eq!(morning.height(), 18.0);
        // Touching sessions stay separate, and a tiny one stays visible.
        let next = layout.mark(&segment(2, 780.0, 800.0));
        assert!(next.left() - morning.right() >= 1.9);
        assert!(layout.mark(&segment(0, 600.0, 600.1)).width() >= 1.4);
        // Rows shrink and labels shorten for long periods, and a single day stays roomy.
        assert_eq!(Timeline::new(rect, 30).bar, 11.0);
        assert_eq!(Timeline::new(rect, 30).labels, 44.0);
        assert_eq!(Timeline::new(rect, 1).bar, 24.0);
        assert!(Timeline::height(92) > Timeline::height(7));
    }

    #[test]
    fn unfinished_marks_stay_wide_enough_to_look_unfinished() {
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(344.0, 200.0));
        let layout = Timeline::new(rect, 7);
        for state in [State::Running, State::Review] {
            let mut short = segment(0, 600.0, 600.5);
            short.state = state;
            assert!(layout.mark(&short).width() >= UNFINISHED_LEAST, "{state:?}");
        }
        let part = |ms| ChartPart {
            project: Some(1),
            ms,
        };
        let day = ChartDay {
            long_label: String::new(),
            short_label: String::new(),
            saved: vec![part(14_400_000)],
            running: Some(part(1000)),
            review_ms: 0,
        };
        let columns = Columns::new(
            Rect::from_min_size(Pos2::ZERO, vec2(344.0, 210.0)),
            2,
            28_800_000,
        );
        let parts = columns.stack(0, &day);
        assert!(parts[1].2.height() >= UNFINISHED_LEAST);
    }

    #[test]
    fn a_tiny_share_of_the_split_bar_stays_visible_and_inside_it() {
        let bar = Rect::from_min_size(Pos2::ZERO, vec2(600.0, 28.0));
        let sliver = Rect::from_min_size(egui::pos2(599.5, 0.0), vec2(0.5, 28.0));
        for (state, least) in [(State::Saved, 1.5), (State::Running, UNFINISHED_LEAST)] {
            let mark = split_mark(sliver, state, bar);
            assert!(
                mark.width() >= least && mark.right() <= bar.right(),
                "{state:?}"
            );
        }
        let wide = Rect::from_min_size(egui::pos2(100.0, 0.0), vec2(50.0, 28.0));
        assert_eq!(split_mark(wide, State::Saved, bar).width(), 48.0);
    }

    #[test]
    fn the_grid_keeps_a_handful_of_lines_at_any_scale() {
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(700.0, Columns::HEIGHT));
        let hour: u64 = 3_600_000;
        let step = |most| Columns::new(rect, 2, most).grid_step();
        assert_eq!(step(0), 2);
        assert_eq!(step(8 * hour), 2);
        assert_eq!(step(12 * hour), 4);
        assert_eq!(step(16 * hour), 4);
        assert_eq!(step(24 * hour), 6);
        let damaged = Columns::new(rect, 2, 100_000 * hour);
        assert!(damaged.scale_hours / damaged.grid_step() <= 5);
    }

    #[test]
    fn timeline_hover_finds_the_nearest_session_on_the_right_row_only() {
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(700.0, 200.0));
        let layout = Timeline::new(rect, 3);
        let segments = [
            segment(0, 600.0, 660.0),
            segment(1, 600.0, 660.0),
            segment(1, 661.0, 700.0),
        ];
        let inside = layout.mark(&segments[1]).center();
        assert_eq!(layout.at(inside, &segments).unwrap().day, 1);
        // A sliver is easier to hit than its painted width.
        let sliver = [segment(0, 600.0, 600.5)];
        let mark = layout.mark(&sliver[0]);
        assert!(layout.at(mark.center() + vec2(2.5, 0.0), &sliver).is_some());
        // The gap between neighbors goes to the nearer one.
        let between = layout.mark(&segments[2]).left_center() - vec2(0.2, 0.0);
        assert_eq!(layout.at(between, &segments).unwrap().start_minute, 661.0);
        assert!(layout.at(egui::pos2(5.0, inside.y), &segments).is_none());
        let third_row = egui::pos2(inside.x, layout.row_top(2) + 4.0);
        assert!(layout.at(third_row, &segments).is_none());
    }

    #[test]
    fn columns_stack_to_their_total_and_share_one_scale() {
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(700.0, Columns::HEIGHT));
        let chart = sample_chart();
        let most = chart.days[1].total_ms();
        let layout = Columns::new(rect, 2, most);
        assert_eq!(
            layout.scale_hours, 4,
            "three hours rounds up to a clean axis"
        );
        let parts = layout.stack(1, &chart.days[1]);
        assert_eq!(parts.len(), 2);
        assert_eq!(
            parts[0].0,
            Some(1),
            "the larger project sits on the baseline"
        );
        assert_eq!(parts[0].2.bottom(), layout.y(0));
        assert!((parts[1].2.top() - layout.y(most)).abs() < 0.01);
        assert!(
            parts[0].2.top() - parts[1].2.bottom() >= 1.9,
            "stacked parts stay separate"
        );
        assert!(layout.width() <= 24.0);
        assert_eq!(layout.column_at(layout.center(1)), Some(1));
        assert_eq!(layout.column_at(0.0), None);
        // Over a quarter year, columns stay visible at a width of one per day.
        let many = Columns::new(rect, 92, most);
        assert!(many.width() >= 2.0);
        assert_eq!(many.column_at(rect.right() - 1.0), Some(91));
    }

    #[test]
    fn rhythm_scale_marks_empty_cells_apart_from_any_recorded_time() {
        assert_eq!(rhythm_step(0, 1000), 0);
        assert_eq!(rhythm_step(1, 1000), 1, "a little time still shows");
        assert_eq!(rhythm_step(1000, 1000), 6);
        assert_eq!(rhythm_step(500, 1000), 3);
        assert_eq!(rhythm_step(0, 0), 0);
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(700.0, 200.0));
        let layout = Rhythm::new(rect);
        let cell = layout.cell_at(1, 14);
        assert_eq!(layout.locate(cell.center(), 2), Some((1, 14)));
        assert_eq!(layout.locate(cell.center(), 1), None);
        assert_eq!(layout.locate(egui::pos2(1.0, 30.0), 2), None);
    }

    struct Harness {
        context: egui::Context,
        chart: Chart,
        view: View,
        colors: ProjectColors,
        width: f32,
    }
    impl Harness {
        fn new(chart: Chart, view: View) -> Self {
            let mut colors = ProjectColors::default();
            colors.update(&catalog(&[(1, false), (2, false)]));
            Self {
                context: egui::Context::default(),
                chart,
                view,
                colors,
                width: 700.0,
            }
        }
        fn frame(&mut self, events: Vec<egui::Event>, time: f64) -> egui::FullOutput {
            let (chart, view, colors) = (&self.chart, &mut self.view, &self.colors);
            let mut output = self.context.run_ui(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(self.width, 700.0))),
                    events,
                    ..Default::default()
                },
                |ui| show(ui, chart, view, colors),
            );
            output.textures_delta.clear();
            output
        }
        fn texts(output: &egui::FullOutput) -> Vec<String> {
            output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
                    _ => None,
                })
                .collect()
        }
        /// Solid marks in the first series color, in either theme, wider than a legend swatch.
        fn first_series_marks(output: &egui::FullOutput) -> Vec<Rect> {
            output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect)
                        if (rect.fill == LIGHT_SERIES[0] || rect.fill == DARK_SERIES[0])
                            && rect.rect.width() > 12.0 =>
                    {
                        Some(rect.rect)
                    }
                    _ => None,
                })
                .collect()
        }
    }

    #[test]
    fn each_view_paints_its_labels_marks_and_shared_legend() {
        for view in [View::Timeline, View::Days] {
            let mut harness = Harness::new(sample_chart(), view);
            harness.frame(vec![], 0.0);
            let output = harness.frame(vec![], 0.1);
            let texts = Harness::texts(&output);
            for expected in ["Alpha", "Beta", "Timeline", "By day", "Weekly rhythm"] {
                assert!(texts.iter().any(|t| t == expected), "{view:?}: {expected}");
            }
            assert!(!Harness::first_series_marks(&output).is_empty(), "{view:?}");
        }
        let mut timeline = Harness::new(sample_chart(), View::Timeline);
        timeline.frame(vec![], 0.0);
        let texts = Harness::texts(&timeline.frame(vec![], 0.1));
        assert!(texts.iter().any(|t| t == "Mon 26 Oct"));
        assert!(texts.iter().any(|t| t == "00:00"));
        assert!(texts.iter().any(|t| t == "24:00"));
        assert!(
            texts.iter().any(|t| t == "3h"),
            "Tue 27 Oct has three hours"
        );
        let mut columns = Harness::new(sample_chart(), View::Days);
        columns.frame(vec![], 0.0);
        let texts = Harness::texts(&columns.frame(vec![], 0.1));
        assert!(texts.iter().any(|t| t == "Mon 26"));
        assert!(texts.iter().any(|t| t == "1h 30m"));
        let mut rhythm = Harness::new(sample_chart(), View::Rhythm);
        rhythm.frame(vec![], 0.0);
        let texts = Harness::texts(&rhythm.frame(vec![], 0.1));
        assert!(texts.iter().any(|t| t == "Mon"));
        assert!(texts.iter().any(|t| t == "More (up to 1h in one cell)"));
        assert!(
            !texts.iter().any(|t| t == "Sun"),
            "weekdays outside the period are left out"
        );
    }

    #[test]
    fn a_single_day_becomes_one_split_bar_with_shares() {
        let mut chart = sample_chart();
        chart.days.truncate(1);
        let mut harness = Harness::new(chart, View::Days);
        harness.frame(vec![], 0.0);
        let texts = Harness::texts(&harness.frame(vec![], 0.1));
        assert!(texts.iter().any(|t| t == "Mon 26 Oct: 1h 30m"));
        assert!(texts.iter().any(|t| t == "67%") && texts.iter().any(|t| t == "33%"));
        assert!(!texts.iter().any(|t| t == "0h"), "no axis for a single day");
    }

    #[test]
    fn hovering_a_mark_shows_its_details() {
        let mut chart = sample_chart();
        // The block follows the real duration and the text follows the clock.
        chart.segments[0].end_clock = 660.0;
        let mut harness = Harness::new(chart, View::Timeline);
        harness.frame(vec![], 0.0);
        let output = harness.frame(vec![], 0.1);
        let pointer = Harness::first_series_marks(&output)[0].center();
        let mut texts = Vec::new();
        for (step, time) in [(0, 0.2), (1, 1.0), (2, 2.0)] {
            let events = if step == 0 {
                vec![egui::Event::PointerMoved(pointer)]
            } else {
                vec![]
            };
            texts = Harness::texts(&harness.frame(events, time));
        }
        assert!(
            texts.iter().any(|t| t == "Mon 26 Oct, 09:00 to 11:00"),
            "{texts:?}"
        );
        assert!(texts.iter().any(|t| t == "1h"));
    }

    #[test]
    fn by_day_labels_never_collide_at_any_window_width() {
        const TOTAL: u64 = 45_000_000;
        for width in [344.0, 420.0, 544.0, 620.0, 684.0, 1000.0] {
            for count in [2, 7, 10, 12, 14, 15, 30, 92] {
                let days = (0..count).map(|_| ChartDay {
                    long_label: "Wed 30 Sep".into(),
                    short_label: "Wed 30".into(),
                    saved: vec![ChartPart {
                        project: Some(1),
                        ms: TOTAL,
                    }],
                    running: None,
                    review_ms: 0,
                });
                let chart = Chart {
                    days: days.collect(),
                    series: vec![ChartSeries {
                        project: Some(1),
                        title: "Alpha".into(),
                        total_ms: TOTAL * count,
                    }],
                    ..Chart::default()
                };
                let mut harness = Harness::new(chart, View::Days);
                harness.width = width;
                harness.frame(vec![], 0.0);
                let output = harness.frame(vec![], 0.1);
                let boxes = |wanted: &str| -> Vec<Rect> {
                    output
                        .shapes
                        .iter()
                        .filter_map(|shape| match &shape.shape {
                            egui::Shape::Text(text) if text.galley.job.text == wanted => {
                                Some(Rect::from_min_size(text.pos, text.galley.size()))
                            }
                            _ => None,
                        })
                        .collect()
                };
                for wanted in ["Wed 30", "12h 30m"] {
                    let found = boxes(wanted);
                    assert!(!found.is_empty(), "{width} wide, {count} days: {wanted}");
                    for (index, first) in found.iter().enumerate() {
                        for second in &found[index + 1..] {
                            assert!(
                                !first.intersects(*second),
                                "{width} wide, {count} days: {wanted} collide"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn by_day_names_only_what_it_draws_and_states_review_time_in_words() {
        let mut chart = sample_chart();
        chart.days.truncate(1);
        chart.days[0].saved.truncate(1);
        chart.days[0].review_ms = 1_800_000;
        chart.segments.truncate(1);
        chart.segments.push(ChartSegment {
            project: Some(2),
            state: State::Review,
            ..segment(0, 700.0, 730.0)
        });
        chart.series[1].total_ms = 0;
        let mut timeline = Harness::new(chart.clone(), View::Timeline);
        timeline.frame(vec![], 0.0);
        let texts = Harness::texts(&timeline.frame(vec![], 0.1));
        assert!(texts.iter().any(|t| t == "Beta"), "its outline has a name");
        assert!(texts.iter().any(|t| t == "Needs review"));
        let mut days = Harness::new(chart.clone(), View::Days);
        days.frame(vec![], 0.0);
        let texts = Harness::texts(&days.frame(vec![], 0.1));
        assert!(texts.iter().any(|t| t == "Alpha"));
        assert!(!texts.iter().any(|t| t == "Beta" || t == "Needs review"));
        assert!(
            texts
                .iter()
                .any(|t| t == "Needs review, not in totals: 30m")
        );
        // A day with only review time says so, and draws no empty bar.
        chart.days[0].saved.clear();
        let mut review = Harness::new(chart, View::Days);
        review.frame(vec![], 0.0);
        let texts = Harness::texts(&review.frame(vec![], 0.1));
        assert!(texts.iter().any(|t| t == "Mon 26 Oct: 0m"));
        assert!(
            texts
                .iter()
                .any(|t| t == "Needs review, not in totals: 30m")
        );
    }

    #[test]
    fn views_explain_what_they_cannot_show_instead_of_drawing_nothing() {
        let mut empty = Harness::new(Chart::default(), View::Timeline);
        empty.frame(vec![], 0.0);
        assert!(
            Harness::texts(&empty.frame(vec![], 0.1))
                .iter()
                .any(|t| t == "No tracked time in this period.")
        );
        let mut long = sample_chart();
        long.too_long = true;
        long.days.clear();
        long.segments.clear();
        let mut harness = Harness::new(long, View::Days);
        harness.frame(vec![], 0.0);
        assert!(
            Harness::texts(&harness.frame(vec![], 0.1))
                .iter()
                .any(|t| t.starts_with("This view covers up to 92 days"))
        );
        harness.view = View::Rhythm;
        harness.frame(vec![], 0.2);
        assert!(
            Harness::texts(&harness.frame(vec![], 0.3))
                .iter()
                .any(|t| t == "Mon"),
            "the weekly rhythm still works for long periods"
        );
    }

    #[test]
    fn running_and_review_time_look_different_from_finished_time() {
        let mut chart = sample_chart();
        chart.segments[0].state = State::Running;
        chart.segments[1].state = State::Review;
        let mut harness = Harness::new(chart, View::Timeline);
        harness.frame(vec![], 0.0);
        let output = harness.frame(vec![], 0.1);
        let texts = Harness::texts(&output);
        assert!(texts.iter().any(|t| t == "Running") && texts.iter().any(|t| t == "Needs review"));
        assert!(
            Harness::first_series_marks(&output).is_empty(),
            "neither is drawn as a solid fill"
        );
    }
}
