//! Pure calendar boundaries and exact allocation of recorded elapsed time.
use crate::tracking::Activity;
use jiff::{Timestamp, civil::Date, tz::TimeZone};
use std::collections::{BTreeMap, HashMap};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Request {
    pub from: String,
    pub through: String,
    pub timezone: String,
    pub from_time: String,
    pub through_time: String,
}
impl Default for Request {
    fn default() -> Self {
        let zone = TimeZone::system();
        let name = zone.iana_name().unwrap_or("UTC");
        Self::recent(1, name).unwrap_or_else(|_| Self {
            from: "1970-01-01".into(),
            through: "1970-01-01".into(),
            timezone: "UTC".into(),
            from_time: "00:00".into(),
            through_time: "24:00".into(),
        })
    }
}
impl Request {
    pub fn recent(days: usize, timezone: &str) -> Result<Self> {
        Self::recent_at(days, timezone, Timestamp::now())
    }
    /// Whole local days ending with the date of `now` in the selected time zone.
    pub fn recent_at(days: usize, timezone: &str, now: Timestamp) -> Result<Self> {
        let zone = TimeZone::get(timezone)?;
        let today = now.to_zoned(zone).date();
        let mut first = today;
        for _ in 1..days {
            first = first.yesterday()?;
        }
        Ok(Self {
            from: first.to_string(),
            through: today.to_string(),
            timezone: timezone.into(),
            from_time: "00:00".into(),
            through_time: "24:00".into(),
        })
    }
    pub fn bounds(&self) -> Result<(i64, i64)> {
        let first: Date = self.from.parse()?;
        let last: Date = self.through.parse()?;
        let begin = parse_minute(&self.from_time, false)?;
        let end = parse_minute(&self.through_time, true)?;
        if (last, end) <= (first, begin) {
            return Err(std::io::Error::other(
                "Until must be later than From. The Until boundary is excluded.",
            )
            .into());
        }
        let zone = TimeZone::get(&self.timezone)?;
        let start_ms = minute_boundary(first, begin, &zone)?;
        let end_ms = minute_boundary(last, end, &zone)?;
        if end_ms < start_ms {
            return Err(
                std::io::Error::other("Until must follow From in the selected time zone.").into(),
            );
        }
        Ok((start_ms, end_ms))
    }
}
/// First local day start strictly after `after_ms`, resolved like report day boundaries.
/// `None` means no later start can be determined, so nothing should be split.
pub(crate) fn next_day_start(after_ms: i64, zone: &TimeZone) -> Option<i64> {
    let today = Timestamp::from_millisecond(after_ms)
        .ok()?
        .to_zoned(zone.clone())
        .date();
    let next = today
        .tomorrow()
        .ok()?
        .to_zoned(zone.clone())
        .ok()?
        .start_of_day()
        .ok()?
        .timestamp()
        .as_millisecond();
    (next > after_ms).then_some(next)
}
fn parse_minute(text: &str, allow_end: bool) -> Result<u16> {
    let bytes = text.as_bytes();
    if bytes.len() != 5
        || bytes[2] != b':'
        || ![bytes[0], bytes[1], bytes[3], bytes[4]]
            .iter()
            .all(u8::is_ascii_digit)
    {
        return Err(std::io::Error::other(
            "Enter time as HH:MM (24:00 is allowed only for Until).",
        )
        .into());
    }
    let hour = (bytes[0] - b'0') as u16 * 10 + (bytes[1] - b'0') as u16;
    let minute = (bytes[3] - b'0') as u16 * 10 + (bytes[4] - b'0') as u16;
    if minute >= 60 || hour >= 24 && !(allow_end && hour == 24 && minute == 0) {
        return Err(std::io::Error::other(
            "Time must be 00:00-23:59; Until also accepts 24:00 for end of day.",
        )
        .into());
    }
    Ok(hour * 60 + minute)
}
fn minute_boundary(mut date: Date, minute: u16, zone: &TimeZone) -> Result<i64> {
    if minute == 1440 {
        date = date.tomorrow()?;
    }
    if minute == 0 || minute == 1440 {
        return Ok(date
            .to_zoned(zone.clone())?
            .start_of_day()?
            .timestamp()
            .as_millisecond());
    }
    zone.to_ambiguous_zoned(date.at((minute/60) as i8,(minute%60) as i8,0,0)).unambiguous()
        .map(|zoned|zoned.timestamp().as_millisecond())
        .map_err(|_|std::io::Error::other("This local time is repeated or skipped by a clock change. Choose an unambiguous time or use UTC.").into())
}
#[derive(Clone, Debug)]
pub struct Day {
    pub date: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub saved_ms: u64,
}
#[derive(Clone, Debug)]
pub struct Range {
    pub request: Request,
    pub start_ms: i64,
    pub end_ms: i64,
    pub days: Vec<Day>,
    zone: TimeZone,
}
impl Range {
    pub fn new(request: &Request) -> Result<Self> {
        let mut date: Date = request.from.parse()?;
        let mut last: Date = request.through.parse()?;
        let (start_ms, end_ms) = request.bounds()?;
        if request.through_time == "00:00" {
            last = last.yesterday()?;
        }
        let zone = TimeZone::get(&request.timezone)?;
        let mut days = Vec::new();
        while date <= last {
            if days.len() == 3660 {
                return Err(std::io::Error::other("Choose at most 3,660 days per report.").into());
            }
            let next = date.tomorrow()?;
            let day_start = date
                .to_zoned(zone.clone())?
                .start_of_day()?
                .timestamp()
                .as_millisecond();
            let day_end = next
                .to_zoned(zone.clone())?
                .start_of_day()?
                .timestamp()
                .as_millisecond();
            days.push(Day {
                date: date.to_string(),
                start_ms: day_start.max(start_ms),
                end_ms: day_end.min(end_ms),
                saved_ms: 0,
            });
            date = next;
        }
        Ok(Self {
            request: request.clone(),
            start_ms,
            end_ms,
            days,
            zone,
        })
    }
    pub fn format_time(&self, ms: i64) -> String {
        Timestamp::from_millisecond(ms)
            .map(|t| {
                t.to_zoned(self.zone.clone())
                    .strftime("%Y-%m-%d %H:%M:%S %:z")
                    .to_string()
            })
            .unwrap_or_else(|_| "Invalid timestamp".into())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Saved,
    Running,
    Review,
}
#[derive(Clone, Debug)]
pub struct Record {
    pub id: i64,
    pub start_ms: i64,
    pub end_ms: i64,
    pub elapsed_ms: u64,
    pub state: State,
    pub activity: Activity,
    pub reason: String,
}
#[derive(Debug)]
pub struct History {
    pub record: Record,
    pub clipped_ms: u64,
}
#[derive(Clone, Debug)]
pub struct Group {
    pub project: String,
    pub project_id: Option<i64>,
    pub task: Option<String>,
    pub saved_ms: u64,
    newest_id: i64,
}
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct GroupKey {
    project: Option<i64>,
    task: Option<String>,
}
#[derive(Debug)]
pub struct Report {
    pub range: Range,
    pub saved_ms: u64,
    pub running_ms: u64,
    pub review_ms: u64,
    pub groups: Vec<Group>,
    pub projects: Vec<Group>,
    pub history: Vec<History>,
    pub matching_sessions: usize,
    pub diagnostics: Vec<String>,
    pub generated_ms: i64,
}
impl Report {
    /// Totals show each project's current name, while session rows keep the recorded one.
    pub fn use_current_titles(&mut self, titles: &HashMap<i64, String>) {
        for groups in [&mut self.groups, &mut self.projects] {
            for group in groups.iter_mut() {
                if let Some(title) = group.project_id.and_then(|id| titles.get(&id)) {
                    group.project = title.clone();
                }
            }
            sort_groups(groups);
        }
    }
}
fn sort_groups(groups: &mut [Group]) {
    groups.sort_by(|a, b| {
        b.saved_ms
            .cmp(&a.saved_ms)
            .then_with(|| a.project.cmp(&b.project))
    });
}

/// Cumulative rounding makes adjacent clips add to the original elapsed duration.
pub(crate) fn allocated(record: &Record, start: i64, end: i64) -> u64 {
    let span = i128::from(record.end_ms) - i128::from(record.start_ms);
    if span <= 0 {
        return 0;
    }
    let prefix = |at: i64| {
        let offset = (i128::from(at) - i128::from(record.start_ms)).clamp(0, span) as u128;
        (u128::from(record.elapsed_ms) * offset / span as u128) as u64
    };
    prefix(end).saturating_sub(prefix(start))
}

pub fn compute(range: Range, records: Vec<Record>, generated_ms: i64) -> Report {
    compute_with_limit(range, records, generated_ms, Some(200))
}
pub(crate) fn compute_all(range: Range, records: Vec<Record>, generated_ms: i64) -> Report {
    compute_with_limit(range, records, generated_ms, None)
}
fn compute_with_limit(
    mut range: Range,
    records: Vec<Record>,
    generated_ms: i64,
    limit: Option<usize>,
) -> Report {
    let mut groups = BTreeMap::<GroupKey, Group>::new();
    let (mut saved_ms, mut running_ms, mut review_ms) = (0u64, 0u64, 0u64);
    let mut history = Vec::new();
    let mut diagnostics = Vec::new();
    for record in records {
        if range.start_ms == range.end_ms {
            continue;
        }
        if record.end_ms < record.start_ms
            || (record.end_ms == record.start_ms && record.elapsed_ms > 0)
        {
            diagnostics.push(format!(
                "Session {} has an invalid wall interval; its duration is excluded.",
                record.id
            ));
            continue;
        }
        if record.start_ms >= range.end_ms
            || record.end_ms < range.start_ms
            || (record.end_ms == range.start_ms && record.start_ms != record.end_ms)
        {
            continue;
        }
        let ms = allocated(&record, range.start_ms, range.end_ms);
        match record.state {
            State::Saved => {
                saved_ms = saved_ms.saturating_add(ms);
                for day in &mut range.days {
                    day.saved_ms =
                        day.saved_ms
                            .saturating_add(allocated(&record, day.start_ms, day.end_ms));
                }
                let project = record.activity.project.as_ref();
                let task = project.and_then(|p| p.task.clone());
                let project_id = project.map(|p| p.id);
                let key = GroupKey {
                    project: project_id,
                    task: task.clone(),
                };
                let group = groups.entry(key).or_insert_with(|| Group {
                    project: record.activity.label().into(),
                    project_id,
                    task,
                    saved_ms: 0,
                    newest_id: record.id,
                });
                if record.id >= group.newest_id {
                    group.project = record.activity.label().into();
                    group.newest_id = record.id;
                }
                group.saved_ms = group.saved_ms.saturating_add(ms);
            }
            State::Running => running_ms = running_ms.saturating_add(ms),
            State::Review => review_ms = review_ms.saturating_add(ms),
        }
        history.push(History {
            record,
            clipped_ms: ms,
        });
    }
    history.sort_by_key(|h| std::cmp::Reverse((h.record.start_ms, h.record.id)));
    let matching_sessions = history.len();
    if let Some(limit) = limit {
        history.truncate(limit);
    }
    let mut projects = BTreeMap::<Option<i64>, Group>::new();
    let mut task_groups = Vec::new();
    for (key, group) in groups {
        let project = projects.entry(key.project).or_insert_with(|| {
            let mut p = group.clone();
            p.task = None;
            p.saved_ms = 0;
            p
        });
        if group.newest_id >= project.newest_id {
            project.project = group.project.clone();
            project.newest_id = group.newest_id;
        }
        project.saved_ms = project.saved_ms.saturating_add(group.saved_ms);
        task_groups.push(group);
    }
    let mut groups = task_groups;
    sort_groups(&mut groups);
    let mut projects: Vec<_> = projects.into_values().collect();
    sort_groups(&mut projects);
    Report {
        range,
        saved_ms,
        running_ms,
        review_ms,
        groups,
        projects,
        history,
        matching_sessions,
        diagnostics,
        generated_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn range(from: &str, through: &str, timezone: &str) -> Range {
        Range::new(&Request {
            from: from.into(),
            through: through.into(),
            timezone: timezone.into(),
            from_time: "00:00".into(),
            through_time: "24:00".into(),
        })
        .unwrap()
    }
    fn record(start: i64, end: i64, elapsed: u64, state: State) -> Record {
        Record {
            id: 1,
            start_ms: start,
            end_ms: end,
            elapsed_ms: elapsed,
            state,
            activity: Activity::default(),
            reason: "test".into(),
        }
    }
    fn minute_range(from: &str, start: &str, until: &str, end: &str, zone: &str) -> Range {
        Range::new(&Request {
            from: from.into(),
            through: until.into(),
            timezone: zone.into(),
            from_time: start.into(),
            through_time: end.into(),
        })
        .unwrap()
    }
    #[test]
    fn minute_filters_clip_daily_totals_and_adjacent_ranges_exactly() {
        let all = minute_range("2026-01-01", "23:30", "2026-01-02", "00:30", "UTC");
        let start = all.start_ms;
        let end = all.end_ms;
        assert_eq!(end - start, 3_600_000);
        let make = || {
            record(
                start - 30 * 60_000,
                end + 30 * 60_000,
                7_200_003,
                State::Saved,
            )
        };
        let report = compute(all, vec![make()], 0);
        assert_eq!(
            report.saved_ms,
            report.range.days.iter().map(|d| d.saved_ms).sum::<u64>()
        );
        assert_eq!(report.range.days.len(), 2);
        assert_eq!(report.range.days[0].start_ms, start);
        assert_eq!(report.range.days[1].end_ms, end);
        let first = compute(
            minute_range("2026-01-01", "23:30", "2026-01-01", "24:00", "UTC"),
            vec![make()],
            0,
        );
        let second = compute(
            minute_range("2026-01-02", "00:00", "2026-01-02", "00:30", "UTC"),
            vec![make()],
            0,
        );
        assert_eq!(first.saved_ms + second.saved_ms, report.saved_ms);
        let utc = minute_range("2026-10-27", "06:15", "2026-10-27", "07:30", "UTC");
        let local = minute_range(
            "2026-10-27",
            "09:15",
            "2026-10-27",
            "10:30",
            "Europe/Istanbul",
        );
        assert_eq!((utc.start_ms, utc.end_ms), (local.start_ms, local.end_ms));
    }
    #[test]
    fn minute_boundaries_reject_invalid_and_ambiguous_times() {
        let mut request = Request {
            from: "2026-10-27".into(),
            through: "2026-10-27".into(),
            timezone: "UTC".into(),
            from_time: "09:15".into(),
            through_time: "10:30".into(),
        };
        for text in ["9:15", "24:00", "09:60", "aa:bb", ""] {
            request.from_time = text.into();
            assert!(request.bounds().is_err());
        }
        request.from_time = "09:15".into();
        for text in ["24:01", "09:15", "08:00"] {
            request.through_time = text.into();
            assert!(request.bounds().is_err());
        }
        request.timezone = "America/New_York".into();
        request.from = "2026-03-08".into();
        request.through = request.from.clone();
        request.from_time = "02:15".into();
        request.through_time = "03:30".into();
        assert!(request.bounds().is_err());
        request.from = "2026-11-01".into();
        request.through = request.from.clone();
        request.from_time = "01:30".into();
        request.through_time = "02:30".into();
        assert!(request.bounds().is_err());
        request.timezone = "UTC".into();
        assert!(request.bounds().is_ok());
    }
    #[test]
    fn until_minute_is_excluded_and_until_midnight_does_not_add_an_extra_day() {
        let range = minute_range("2026-01-01", "23:00", "2026-01-02", "00:00", "UTC");
        assert_eq!(range.days.len(), 1);
        let end = range.end_ms;
        let start = range.start_ms;
        let report = compute(
            range,
            vec![
                record(end, end + 1000, 1000, State::Running),
                record(start - 1000, start, 1000, State::Saved),
            ],
            0,
        );
        assert_eq!(report.matching_sessions, 0);
    }
    #[test]
    fn calendar_boundaries_handle_dst_midnight_gaps_and_skipped_dates() {
        for (date, hours) in [("2026-03-08", 23), ("2026-11-01", 25)] {
            let range = range(date, date, "America/New_York");
            assert_eq!(range.end_ms - range.start_ms, hours * 3_600_000);
        }
        let gap = range("2015-10-18", "2015-10-18", "America/Sao_Paulo");
        assert_eq!(gap.end_ms - gap.start_ms, 23 * 3_600_000);
        let skipped = range("2011-12-30", "2011-12-30", "Pacific/Apia");
        assert_eq!(skipped.end_ms, skipped.start_ms);
        let start = skipped.start_ms;
        assert_eq!(
            compute(
                skipped,
                vec![record(start - 1000, start + 1000, 2000, State::Saved)],
                0
            )
            .matching_sessions,
            0
        );
    }
    #[test]
    fn midnight_and_adjacent_ranges_preserve_elapsed_even_with_drift() {
        let all = range("2026-01-01", "2026-01-02", "UTC");
        let middle = all.days[1].start_ms;
        let make = || record(middle - 1000, middle + 2000, 3001, State::Saved);
        let report = compute(all, vec![make()], 0);
        assert_eq!(report.saved_ms, 3001);
        assert_eq!(
            report.range.days.iter().map(|d| d.saved_ms).sum::<u64>(),
            3001
        );
        let first = compute(range("2026-01-01", "2026-01-01", "UTC"), vec![make()], 0);
        let second = compute(range("2026-01-02", "2026-01-02", "UTC"), vec![make()], 0);
        assert_eq!(first.saved_ms + second.saved_ms, 3001);
        assert_eq!(allocated(&make(), middle - 500, middle + 500), 1000);
    }
    #[test]
    fn uncertain_and_live_time_are_separate_and_history_keeps_old_titles() {
        let range = range("2026-01-01", "2026-01-01", "UTC");
        let start = range.start_ms;
        let mut saved = record(start, start + 1000, 1000, State::Saved);
        saved.activity.project = Some(crate::tracking::Project {
            id: 1,
            title: "Archived \u{1f331}".into(),
            task: Some("Old task".into()),
        });
        let mut renamed = saved.clone();
        renamed.id = 2;
        renamed.start_ms += 1000;
        renamed.end_ms += 1000;
        renamed.activity.project.as_mut().unwrap().title = "New title".into();
        let mut other_project = renamed.clone();
        other_project.id = 3;
        other_project.activity.project.as_mut().unwrap().id = 2;
        let report = compute(
            range,
            vec![
                saved,
                renamed,
                other_project,
                record(start + 3000, start + 4000, 1000, State::Review),
                record(start + 4000, start + 5000, 1000, State::Running),
            ],
            0,
        );
        assert_eq!(report.saved_ms, 3000);
        assert_eq!(report.review_ms, 1000);
        assert_eq!(report.running_ms, 1000);
        assert_eq!(report.groups.len(), 2);
        assert_eq!(report.groups[0].project, "New title");
        assert!(
            report
                .history
                .iter()
                .any(|h| h.record.activity.label() == "Archived \u{1f331}")
        );
        assert_eq!(report.groups.iter().map(|g| g.saved_ms).sum::<u64>(), 3000);
    }
    #[test]
    fn history_limit_never_truncates_totals_and_invalid_intervals_are_explicit() {
        let range = range("2026-01-01", "2026-01-01", "UTC");
        let start = range.start_ms;
        let mut records: Vec<_> = (0..250)
            .map(|i| {
                let mut r = record(start + i * 1000, start + (i + 1) * 1000, 1000, State::Saved);
                r.id = i;
                r
            })
            .collect();
        records.push(record(start, start, 10, State::Saved));
        let report = compute(range, records, 0);
        assert_eq!(report.saved_ms, 250_000);
        assert_eq!(report.history.len(), 200);
        assert_eq!(report.matching_sessions, 250);
        assert_eq!(report.diagnostics.len(), 1);
    }
    /// Deterministic generator: failures reproduce without adding a test dependency.
    struct Sequence(u64);
    impl Sequence {
        fn next(&mut self, bound: u64) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (self.0 >> 33) % bound
        }
    }
    fn project(id: i64, title: &str, task: Option<&str>) -> crate::tracking::Project {
        crate::tracking::Project {
            id,
            title: title.into(),
            task: task.map(Into::into),
        }
    }
    /// Non-overlapping sessions, with elapsed time deviating slightly from the wall span.
    fn timeline(rng: &mut Sequence, mut at: i64, end: i64) -> Vec<Record> {
        let mut records = Vec::new();
        let mut id = 0;
        while at < end {
            at += rng.next(7_200_000) as i64;
            let span = 1 + rng.next(14_400_000);
            let elapsed = span + rng.next(2_001) - rng.next(2_001).min(span);
            id += 1;
            let mut record = record(at, at + span as i64, elapsed, State::Saved);
            record.id = id;
            record.activity.project = match rng.next(4) {
                0 => None,
                1 => Some(project(1, "Alpha", None)),
                2 => Some(project(1, "Alpha", Some("Task \u{1f331}"))),
                _ => Some(project(2, "Beta", None)),
            };
            at = record.end_ms;
            records.push(record);
        }
        records
    }
    #[test]
    fn totals_agree_for_every_grouping_and_partition_across_calendar_boundaries() {
        // 23-, 25-hour, gap-at-midnight and skipped-date days are all inside these ranges.
        for (zone, from, through) in [
            ("UTC", "2026-01-01", "2026-01-10"),
            ("Europe/Istanbul", "2026-10-25", "2026-11-04"),
            ("America/New_York", "2026-03-06", "2026-03-10"),
            ("America/New_York", "2026-10-30", "2026-11-03"),
            ("America/Sao_Paulo", "2015-10-16", "2015-10-20"),
            ("Pacific/Apia", "2011-12-28", "2012-01-02"),
            ("Asia/Kathmandu", "2026-05-01", "2026-05-05"),
            ("Australia/Lord_Howe", "2026-04-03", "2026-04-07"),
        ] {
            let mut rng = Sequence(zone.len() as u64 ^ 0x5eed);
            let whole = range(from, through, zone);
            let (begin, end) = (whole.start_ms, whole.end_ms);
            let dates: Vec<String> = whole.days.iter().map(|d| d.date.clone()).collect();
            // Sessions begin before the range and end after it, so clipping is exercised.
            let records = timeline(&mut rng, begin - 18_000_000, end + 18_000_000);
            let report = compute_all(whole, records.clone(), 0);
            let context = format!("{zone} {from}..{through}");
            assert!(report.diagnostics.is_empty(), "{context}");
            assert_eq!(
                report.range.days.iter().map(|d| d.saved_ms).sum::<u64>(),
                report.saved_ms,
                "days must add up: {context}"
            );
            assert_eq!(
                report.groups.iter().map(|g| g.saved_ms).sum::<u64>(),
                report.saved_ms,
                "tasks must add up: {context}"
            );
            assert_eq!(
                report.projects.iter().map(|g| g.saved_ms).sum::<u64>(),
                report.saved_ms,
                "projects must add up: {context}"
            );
            assert_eq!(
                report.history.iter().map(|h| h.clipped_ms).sum::<u64>(),
                report.saved_ms,
                "sessions must add up: {context}"
            );
            // Every calendar day queried on its own equals that day's row in the long report.
            for (index, date) in dates.iter().enumerate() {
                let single = compute_all(range(date, date, zone), records.clone(), 0);
                assert_eq!(
                    single.saved_ms, report.range.days[index].saved_ms,
                    "single-day report for {date}: {context}"
                );
            }
            // Arbitrary adjacent minute windows partition the range without loss or overlap.
            let mut cuts = vec![begin];
            while *cuts.last().unwrap() < end {
                let step = (60_000 + rng.next(20_000_000) as i64).min(end - cuts.last().unwrap());
                cuts.push(cuts.last().unwrap() + step);
            }
            let mut sum = 0;
            for pair in cuts.windows(2) {
                sum += records
                    .iter()
                    .map(|r| allocated(r, pair[0], pair[1]))
                    .sum::<u64>();
            }
            assert_eq!(sum, report.saved_ms, "adjacent windows: {context}");
            // A session wholly inside the range contributes exactly its recorded duration.
            let inside: Vec<_> = records
                .iter()
                .filter(|r| r.start_ms >= begin && r.end_ms <= end)
                .cloned()
                .collect();
            assert!(!inside.is_empty(), "{context}");
            let expected = inside.iter().map(|r| r.elapsed_ms).sum::<u64>();
            let contained = compute_all(range(from, through, zone), inside, 0);
            assert_eq!(contained.saved_ms, expected, "full durations: {context}");
        }
    }
    #[test]
    fn half_open_boundaries_count_a_session_touching_midnight_on_one_side_only() {
        let first = range("2026-01-01", "2026-01-01", "UTC");
        let midnight = first.end_ms;
        let ends_at = || record(midnight - 1000, midnight, 1000, State::Saved);
        let starts_at = || record(midnight, midnight + 1000, 1000, State::Saved);
        let next = || range("2026-01-02", "2026-01-02", "UTC");
        assert_eq!(compute(first.clone(), vec![ends_at()], 0).saved_ms, 1000);
        assert_eq!(compute(next(), vec![ends_at()], 0).matching_sessions, 0);
        assert_eq!(compute(first, vec![starts_at()], 0).matching_sessions, 0);
        assert_eq!(compute(next(), vec![starts_at()], 0).saved_ms, 1000);
        let both = range("2026-01-01", "2026-01-02", "UTC");
        let report = compute(both, vec![ends_at(), starts_at()], 0);
        let days: Vec<_> = report.range.days.iter().map(|d| d.saved_ms).collect();
        assert_eq!((report.saved_ms, days), (2000, vec![1000, 1000]));
    }
    #[test]
    fn day_rows_cover_only_the_selected_minutes_on_partial_first_and_last_days() {
        let all = minute_range("2026-01-01", "22:00", "2026-01-03", "02:00", "UTC");
        let (begin, end) = (all.start_ms, all.end_ms);
        let span = (end - begin) as u64;
        let report = compute(
            all,
            vec![record(
                begin - 3_600_000,
                end + 3_600_000,
                span + 7_200_000,
                State::Saved,
            )],
            0,
        );
        let days: Vec<_> = report.range.days.iter().map(|d| d.saved_ms).collect();
        assert_eq!(days, vec![7_200_000, 86_400_000, 7_200_000]);
        assert_eq!(report.saved_ms, span);
    }
    #[test]
    fn invalid_request_is_rejected() {
        for request in [
            Request {
                from: "invalid".into(),
                through: "2026-01-01".into(),
                timezone: "UTC".into(),
                from_time: "00:00".into(),
                through_time: "24:00".into(),
            },
            Request {
                from: "2026-02-01".into(),
                through: "2026-01-01".into(),
                timezone: "UTC".into(),
                from_time: "00:00".into(),
                through_time: "24:00".into(),
            },
            Request {
                from: "2026-01-01".into(),
                through: "2026-01-01".into(),
                timezone: "No/Such_Zone".into(),
                from_time: "00:00".into(),
                through_time: "24:00".into(),
            },
        ] {
            assert!(Range::new(&request).is_err());
        }
    }
}
