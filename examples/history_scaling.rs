//! How the cost of everyday work grows with the history. It builds synthetic histories in a
//! temporary directory, never in a profile, and times the real store, engine and reports.
//!
//! cargo run --release --example history_scaling [-- YEARS:SESSIONS_PER_DAY ...]
//!
//! Timings depend on the machine and its load; compare runs on one machine, not numbers.
use attention_focus::{reports, tracking};
use jiff::{Timestamp, tz::TimeZone};
use rusqlite::{Connection, params};
use std::{path::Path, time::Instant};

const DAY_MS: i64 = 86_400_000;
/// 2020-01-01T00:00:00Z.
const FIRST_DAY_MS: i64 = 1_577_836_800_000;
const PROJECTS: i64 = 30;
const NOTE: &str = "Synthetic note text of varying length, used only for measuring the history.";

/// Deterministic generator, so every run measures the same history.
struct Sequence(u64);
impl Sequence {
    fn next(&mut self, bound: i64) -> i64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as i64 % bound
    }
}

/// Saved sessions filling about ten hours of each day. Returns when the last one ends.
fn populate(path: &Path, days: i64, per_day: i64) -> i64 {
    drop(tracking::Store::open(path).unwrap());
    let mut connection = Connection::open(path).unwrap();
    let tx = connection.transaction().unwrap();
    for project in 1..=PROJECTS {
        tx.execute(
            "INSERT INTO focus_projects(title) VALUES(?1)",
            [format!("Project {project}")],
        )
        .unwrap();
        for task in 1..=8 {
            tx.execute(
                "INSERT INTO focus_tasks(project_id,title) VALUES(?1,?2)",
                params![project, format!("Task {task}")],
            )
            .unwrap();
        }
    }
    let mut last = FIRST_DAY_MS;
    {
        let mut insert = tx.prepare("INSERT INTO focus_sessions(started_ms,checkpoint_ms,ended_ms,elapsed_ms,state,activity,end_reason) VALUES(?1,?2,?2,?3,'stopped',?4,'switch')").unwrap();
        let mut rng = Sequence(42);
        let minutes = (600 / per_day).max(2);
        for day in 0..days {
            let mut at = FIRST_DAY_MS + day * DAY_MS + 8 * 3_600_000;
            for _ in 0..per_day {
                let start = at + rng.next(3) * 60_000;
                let end = start + (minutes / 2 + rng.next(minutes)).max(1) * 60_000;
                let id = 1 + rng.next(PROJECTS);
                let task = (rng.next(10) < 7).then(|| format!("Task {}", 1 + rng.next(8)));
                let activity = tracking::Activity {
                    project: Some(tracking::Project {
                        id,
                        title: format!("Project {id}"),
                        task,
                    }),
                    note: NOTE[..rng.next(NOTE.len() as i64) as usize].into(),
                };
                insert
                    .execute(params![
                        start,
                        end,
                        end - start,
                        serde_json::to_string(&activity).unwrap()
                    ])
                    .unwrap();
                at = end;
                last = end;
            }
        }
    }
    tx.commit().unwrap();
    last
}

/// The median of several runs, in milliseconds.
fn time<T>(runs: usize, mut work: impl FnMut() -> T) -> f64 {
    let mut times: Vec<f64> = (0..runs)
        .map(|_| {
            let started = Instant::now();
            std::hint::black_box(work());
            started.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    times.sort_by(f64::total_cmp);
    times[times.len() / 2]
}

/// Whole UTC days ending with the day of `last`, as the report window requests them.
fn period(last: i64, days: i64) -> reports::Request {
    let date = |ms: i64| {
        Timestamp::from_millisecond(ms)
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .date()
            .to_string()
    };
    reports::Request {
        from: date(last - (days - 1) * DAY_MS),
        through: date(last),
        timezone: "UTC".into(),
        from_time: "00:00".into(),
        through_time: "24:00".into(),
    }
}

fn measure(years: i64, per_day: i64) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("focus.sqlite3");
    let days = years * 365;
    let last = populate(&path, days, per_day);
    let megabytes = std::fs::metadata(&path).unwrap().len() as f64 / 1e6;
    let open = time(5, || tracking::Store::open(&path).unwrap());

    // A command as the worker handles it: apply it, then publish the new state.
    let start_wall = last + 3_600_000;
    let at = |mono_ms: u64| tracking::Moment {
        wall_ms: start_wall + mono_ms as i64,
        mono_ms,
    };
    let mut engine = tracking::Engine::new(tracking::Store::open(&path).unwrap(), at(0));
    engine.snapshot(at(0)).unwrap();
    let activity = tracking::Activity {
        project: Some(tracking::Project {
            id: 3,
            title: "Project 3".into(),
            task: None,
        }),
        note: String::new(),
    };
    let (mut starts, mut stops) = (Vec::new(), Vec::new());
    let mut mono = 0;
    for _ in 0..5 {
        for (command, times) in [
            (tracking::Command::Start(activity.clone()), &mut starts),
            (tracking::Command::Stop, &mut stops),
        ] {
            mono += 1000;
            let started = Instant::now();
            engine.apply(Some(command), at(mono)).unwrap();
            engine.snapshot(at(mono)).unwrap();
            times.push(started.elapsed().as_secs_f64() * 1000.0);
        }
    }
    drop(engine);
    let median = |mut times: Vec<f64>| {
        times.sort_by(f64::total_cmp);
        times[times.len() / 2]
    };

    // Reading, decoding and totalling a period, as an export does.
    let report = |period_days: i64| {
        time(3, || {
            tracking::export_snapshot(&path, &period(last, period_days.min(3660))).unwrap()
        })
    };
    println!(
        "{years:>3} {per_day:>5} {:>7} {megabytes:>6.1} | {open:>6.0} {:>6.0} {:>6.0} | {:>6.0} {:>6.0} {:>6.0} {:>7.0}",
        days * per_day,
        median(starts),
        median(stops),
        report(1),
        report(30),
        report(365),
        report(days),
    );
}

fn main() {
    let mut scenarios: Vec<(i64, i64)> = std::env::args()
        .skip(1)
        .map(|argument| {
            let (years, per_day) = argument
                .split_once(':')
                .expect("scenarios look like YEARS:SESSIONS_PER_DAY");
            (years.parse().unwrap(), per_day.parse().unwrap())
        })
        .collect();
    if scenarios.is_empty() {
        scenarios = vec![(1, 20), (3, 20), (5, 20), (10, 20), (10, 50)];
    }
    println!("Milliseconds, median of several runs. Reports cover whole days ending today.");
    println!(
        "{:>3} {:>5} {:>7} {:>6} | {:>6} {:>6} {:>6} | {:>6} {:>6} {:>6} {:>7}",
        "yrs", "/day", "rows", "MB", "open", "start", "stop", "today", "30d", "365d", "all"
    );
    for (years, per_day) in scenarios {
        measure(years, per_day);
    }
}
