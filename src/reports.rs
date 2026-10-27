//! Pure calendar boundaries and exact allocation of recorded elapsed time.
use crate::tracking::Activity;
use jiff::{Timestamp, civil::Date, tz::TimeZone};
use std::collections::BTreeMap;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub from: String,
    pub through: String,
    pub timezone: String,
}
impl Default for Request {
    fn default() -> Self {
        let zone = TimeZone::system();
        let name = zone.iana_name().unwrap_or("UTC");
        Self::recent(1, name).unwrap_or_else(|_| Self {
            from: "1970-01-01".into(),
            through: "1970-01-01".into(),
            timezone: "UTC".into(),
        })
    }
}
impl Request {
    pub fn recent(days: usize, timezone: &str) -> Result<Self> {
        let zone = TimeZone::get(timezone)?;
        let today = Timestamp::now().to_zoned(zone).date();
        let mut first = today;
        for _ in 1..days {
            first = first.yesterday()?;
        }
        Ok(Self {
            from: first.to_string(),
            through: today.to_string(),
            timezone: timezone.into(),
        })
    }
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
        let last: Date = request.through.parse()?;
        if last < date {
            return Err(std::io::Error::other("Through date must not precede From date.").into());
        }
        let zone = TimeZone::get(&request.timezone)?;
        let mut days = Vec::new();
        while date <= last {
            if days.len() == 3660 {
                return Err(std::io::Error::other("Choose at most 3,660 days per report.").into());
            }
            let next = date.tomorrow()?;
            let start_ms = date
                .to_zoned(zone.clone())?
                .start_of_day()?
                .timestamp()
                .as_millisecond();
            let end_ms = next
                .to_zoned(zone.clone())?
                .start_of_day()?
                .timestamp()
                .as_millisecond();
            days.push(Day {
                date: date.to_string(),
                start_ms,
                end_ms,
                saved_ms: 0,
            });
            date = next;
        }
        Ok(Self {
            request: request.clone(),
            start_ms: days[0].start_ms,
            end_ms: days.last().unwrap().end_ms,
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
    pub task: Option<String>,
    pub source: Option<String>,
    pub saved_ms: u64,
    newest_id: i64,
}
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct GroupKey {
    source: Option<(i64, String, String)>,
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

/// Cumulative rounding makes adjacent clips add to the original elapsed duration.
fn allocated(record: &Record, start: i64, end: i64) -> u64 {
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

pub fn compute(mut range: Range, records: Vec<Record>, generated_ms: i64) -> Report {
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
                let key = GroupKey {
                    source: project.map(|p| (p.source_id, p.source_path.clone(), p.slug.clone())),
                    task: task.clone(),
                };
                let group = groups.entry(key).or_insert_with(|| Group {
                    project: record.activity.label().into(),
                    task,
                    source: project.map(|p| p.source_path.clone()),
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
    history.truncate(200);
    let mut projects = BTreeMap::<Option<(i64, String, String)>, Group>::new();
    let mut task_groups = Vec::new();
    for (key, group) in groups {
        let project = projects.entry(key.source).or_insert_with(|| {
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
    groups.sort_by(|a, b| {
        b.saved_ms
            .cmp(&a.saved_ms)
            .then_with(|| a.project.cmp(&b.project))
    });
    let mut projects: Vec<_> = projects.into_values().collect();
    projects.sort_by(|a, b| {
        b.saved_ms
            .cmp(&a.saved_ms)
            .then_with(|| a.project.cmp(&b.project))
    });
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
            source_id: 1,
            source_path: "synthetic-source".into(),
            slug: "sample".into(),
            title: "Archived \u{1f331}".into(),
            task: Some("Old task".into()),
        });
        let mut renamed = saved.clone();
        renamed.id = 2;
        renamed.start_ms += 1000;
        renamed.end_ms += 1000;
        renamed.activity.project.as_mut().unwrap().title = "New title".into();
        let mut other_source = renamed.clone();
        other_source.id = 3;
        other_source.activity.project.as_mut().unwrap().source_id = 2;
        let report = compute(
            range,
            vec![
                saved,
                renamed,
                other_source,
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
    #[test]
    fn invalid_request_is_rejected() {
        for request in [
            Request {
                from: "invalid".into(),
                through: "2026-01-01".into(),
                timezone: "UTC".into(),
            },
            Request {
                from: "2026-02-01".into(),
                through: "2026-01-01".into(),
                timezone: "UTC".into(),
            },
            Request {
                from: "2026-01-01".into(),
                through: "2026-01-01".into(),
                timezone: "No/Such_Zone".into(),
            },
        ] {
            assert!(Range::new(&request).is_err());
        }
    }
}
