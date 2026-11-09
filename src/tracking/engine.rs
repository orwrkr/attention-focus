//! The clock-injected timer: applies commands and detects clock or heartbeat problems.
use super::{Activity, ActivityKey, ChangeMode, Command, Moment, Result, Snapshot, Store, reject};
use crate::reports;
use rusqlite::params;

/// Saved sessions that reach into a day, read through the end index.
pub(super) const DAILY_ROWS: &str = "SELECT started_ms,ended_ms,elapsed_ms,activity FROM focus_sessions WHERE state='stopped' AND started_ms<?2 AND ended_ms>?1";

pub struct Engine {
    pub(super) store: Store,
    pub(super) started: Option<Moment>,
    pub(super) observed: Moment,
    checkpoint_mono: u64,
    pub(super) daily_cache: std::cell::RefCell<Option<DailyTotals>>,
    /// Stored state reused between commands, so idle ticks do not repeat every query.
    stored: std::cell::RefCell<Option<Snapshot>>,
    /// Calendar override for tests; otherwise the current system zone is read when needed.
    pub(super) zone: Option<jiff::tz::TimeZone>,
}
pub(super) struct DailyTotals {
    begin: i64,
    end: i64,
    key: ActivityKey,
    project_ms: u64,
    all_ms: u64,
}
impl Engine {
    pub fn new(store: Store, now: Moment) -> Self {
        Self {
            store,
            started: None,
            observed: now,
            checkpoint_mono: now.mono_ms,
            daily_cache: std::cell::RefCell::new(None),
            stored: std::cell::RefCell::new(None),
            zone: None,
        }
    }
    fn zone(&self) -> jiff::tz::TimeZone {
        self.zone.clone().unwrap_or_else(jiff::tz::TimeZone::system)
    }
    /// Windows can step a clock back slightly when it synchronizes. While tracking, wall time
    /// then continues from the latest observation by monotonic time, so a saved interval
    /// always spans its elapsed time and the next one cannot overlap it.
    pub(super) fn on_timeline(&self, now: Moment) -> Moment {
        if self.started.is_none() {
            return now;
        }
        let continued = i128::from(self.observed.wall_ms) + i128::from(now.mono_ms)
            - i128::from(self.observed.mono_ms);
        Moment {
            wall_ms: i64::try_from(continued).map_or(now.wall_ms, |wall| now.wall_ms.max(wall)),
            ..now
        }
    }
    pub fn snapshot(&self, now: Moment) -> Result<Snapshot> {
        self.snapshot_in_zone(now, self.zone())
    }
    pub(super) fn snapshot_in_zone(
        &self,
        now: Moment,
        zone: jiff::tz::TimeZone,
    ) -> Result<Snapshot> {
        let now = self.on_timeline(now);
        let mut snapshot = match self.stored.borrow().as_ref() {
            Some(stored) => stored.clone(),
            None => self.store.snapshot()?,
        };
        self.stored
            .borrow_mut()
            .get_or_insert_with(|| snapshot.clone());
        if let Some(start) = self.started {
            snapshot.elapsed_ms = now.mono_ms.saturating_sub(start.mono_ms);
        }
        let daily = (|| -> Result<(u64, u64)> {
            let day = jiff::Timestamp::from_millisecond(now.wall_ms)?
                .to_zoned(zone.clone())
                .date();
            let begin = day
                .to_zoned(zone.clone())?
                .start_of_day()?
                .timestamp()
                .as_millisecond();
            let end = day
                .tomorrow()?
                .to_zoned(zone)?
                .start_of_day()?
                .timestamp()
                .as_millisecond();
            let mut key = snapshot.activity.key();
            key.task = None;
            let mut cache = self.daily_cache.borrow_mut();
            if cache
                .as_ref()
                .is_none_or(|entry| entry.begin != begin || entry.end != end || entry.key != key)
            {
                let mut total = 0u64;
                let mut all = 0u64;
                let mut query = self.store.connection.prepare(DAILY_ROWS)?;
                let rows = query.query_map(params![begin, end], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                })?;
                for row in rows {
                    let (start_ms, end_ms, elapsed, text) = row?;
                    let activity: Activity = serde_json::from_str(&text)?;
                    let matches_project = activity.key().project == key.project;
                    let allocated = reports::allocated(
                        &reports::Record {
                            id: 0,
                            start_ms,
                            end_ms,
                            elapsed_ms: elapsed.max(0) as u64,
                            state: reports::State::Saved,
                            activity,
                            reason: String::new(),
                        },
                        begin,
                        end,
                    );
                    all = all.saturating_add(allocated);
                    if matches_project {
                        total = total.saturating_add(allocated);
                    }
                }
                *cache = Some(DailyTotals {
                    begin,
                    end,
                    key,
                    project_ms: total,
                    all_ms: all,
                });
            }
            let mut total = cache.as_ref().unwrap().project_ms;
            let mut all = cache.as_ref().unwrap().all_ms;
            if let Some(start) = self.started {
                let live = reports::allocated(
                    &reports::Record {
                        id: 0,
                        start_ms: start.wall_ms,
                        end_ms: now.wall_ms,
                        elapsed_ms: snapshot.elapsed_ms,
                        state: reports::State::Running,
                        activity: snapshot.activity.clone(),
                        reason: String::new(),
                    },
                    begin,
                    end,
                );
                total = total.saturating_add(live);
                all = all.saturating_add(live);
            }
            Ok((total, all))
        })();
        // A broken historical row must not prevent publishing successful Start/Stop state.
        if let Ok((project, all)) = daily {
            snapshot.daily_project_ms = Some(project);
            snapshot.daily_all_ms = Some(all);
        }
        Ok(snapshot)
    }
    /// Returns true only after a successful explicit exit.
    pub fn apply(&mut self, command: Option<Command>, now: Moment) -> Result<bool> {
        if command
            .as_ref()
            .is_some_and(|c| !matches!(c, Command::Report { .. }))
        {
            self.daily_cache.borrow_mut().take();
            self.stored.borrow_mut().take();
        }
        // A queued lifecycle boundary can predate the last observation, moving both clocks
        // back together, so only divergence between them counts as a clock change.
        let mono_delta = i128::from(now.mono_ms) - i128::from(self.observed.mono_ms);
        let wall_delta = i128::from(now.wall_ms) - i128::from(self.observed.wall_ms);
        if self.started.is_some()
            && (mono_delta > 30_000 || (wall_delta - mono_delta).abs() > 5_000)
        {
            self.store.interrupt("clock_or_heartbeat")?;
            self.started = None;
            self.stored.borrow_mut().take();
        }
        let now = self.on_timeline(now);
        if mono_delta >= 0 {
            self.observed = now;
        }
        let split = self.split_at_day_starts(now);
        let elapsed = self
            .started
            .map(|start| now.mono_ms.saturating_sub(start.mono_ms));
        let tick = command.is_none();
        match command {
            Some(Command::SaveInterval(edit)) => self.store.save_interval(&edit, now.wall_ms)?,
            Some(Command::DeleteInterval { id, recorded }) => {
                self.store.delete_interval(id, recorded, now.wall_ms)?;
            }
            Some(Command::Start(activity)) => {
                if self.started.is_some() {
                    return Err(reject("errors.already_running").into());
                }
                self.start(now, &activity, None)?;
            }
            Some(Command::Switch(activity)) => {
                let elapsed = elapsed.ok_or_else(|| reject("errors.start_before_switch"))?;
                if self.store.selected()? != activity {
                    self.start(now, &activity, Some(elapsed))?;
                }
            }
            Some(Command::Select(activity)) => {
                if self.started.is_some() {
                    return Err(reject("errors.use_switch").into());
                }
                self.store.select(&activity)?;
            }
            Some(Command::ChangeActivity(activity)) => {
                if let Some(elapsed) = elapsed {
                    if self.store.selected()? != activity {
                        match self.store.change_mode()? {
                            ChangeMode::NewInterval => self.start(now, &activity, Some(elapsed))?,
                            ChangeMode::EditCurrent => {
                                self.store.check_startable(&activity)?;
                                self.store.reassign(&activity, now.wall_ms, elapsed)?;
                                self.checkpoint_mono = now.mono_ms;
                            }
                        }
                    }
                } else {
                    if self.store.needs_review()? {
                        return Err(reject("errors.review_before_change").into());
                    }
                    self.store.select(&activity)?;
                }
            }
            Some(Command::SetChangeMode(mode)) => {
                self.store.connection.execute(
                    "UPDATE focus_preferences SET split_on_change=?1 WHERE singleton=1",
                    [mode == ChangeMode::NewInterval],
                )?;
            }
            Some(Command::SetSplitOnDay(enabled)) => {
                self.store.connection.execute(
                    "UPDATE focus_preferences SET split_on_day=?1 WHERE singleton=1",
                    [enabled],
                )?;
            }
            Some(Command::AddProjects(titles)) => self.store.add_projects(&titles)?,
            Some(Command::RenameProject { id, title }) => self.store.rename_project(id, &title)?,
            Some(Command::SetProjectArchived { id, archived }) => {
                self.store.set_project_archived(id, archived)?;
            }
            Some(Command::DeleteProject(id)) => self.store.delete_project(id)?,
            Some(Command::AddTasks { project, titles }) => {
                self.store.add_tasks(project, &titles)?;
            }
            Some(Command::SetTaskDone {
                project,
                title,
                done,
            }) => self.store.set_task_done(project, &title, done)?,
            Some(Command::DeleteTask { project, title }) => {
                self.store.delete_task(project, &title)?;
            }
            Some(Command::Stop) => {
                if let Some(elapsed) = elapsed {
                    self.stop(now, elapsed, "stop")?;
                }
            }
            Some(Command::Toggle) => {
                if let Some(elapsed) = elapsed {
                    self.stop(now, elapsed, "stop")?;
                } else {
                    self.start(now, &self.store.selected()?, None)?;
                }
            }
            Some(Command::Interrupt { reason, .. }) => {
                if let (Some(elapsed), Some(start)) = (elapsed, self.started) {
                    // A boundary captured just before a queued start still ends that interval.
                    let at = Moment {
                        wall_ms: now.wall_ms.max(start.wall_ms),
                        ..now
                    };
                    self.stop(at, elapsed, reason)?;
                }
            }
            Some(Command::AcceptRecovery) => self.store.accept_recovery()?,
            Some(Command::Exit) => {
                if let Some(elapsed) = elapsed {
                    self.stop(now, elapsed, "exit")?;
                }
                return Ok(true);
            }
            Some(Command::Report { .. }) | None => {
                if let Some(elapsed) = elapsed
                    && now.mono_ms.saturating_sub(self.checkpoint_mono) >= 10_000
                {
                    self.store.checkpoint(now.wall_ms, elapsed)?;
                    self.checkpoint_mono = now.mono_ms;
                }
            }
        }
        // A failed split retries on each tick, where it is reported after checkpointing.
        // Commands and reports proceed regardless: reports clip unsplit intervals correctly.
        if tick {
            split?;
        }
        Ok(false)
    }
    /// While enabled, intervals never span a local day start. Each passed start closes the
    /// running interval and continues it from that boundary. Monotonic elapsed time is
    /// divided rather than recalculated, so the parts always add up to the same duration.
    /// Recovery cases such as long gaps were already interrupted before reaching this point.
    fn split_at_day_starts(&mut self, now: Moment) -> Result<()> {
        let Some(mut start) = self.started else {
            return Ok(());
        };
        let zone = self.zone();
        let passed = |start: Moment| {
            reports::next_day_start(start.wall_ms, &zone).filter(|&day| day < now.wall_ms)
        };
        if passed(start).is_none() || !self.store.split_on_day()? {
            return Ok(());
        }
        while let Some(boundary) = passed(start) {
            let behind = (now.wall_ms - boundary) as u64;
            let mono_ms = now.mono_ms.saturating_sub(behind).max(start.mono_ms);
            self.store.split_day(
                boundary,
                mono_ms - start.mono_ms,
                now.wall_ms,
                now.mono_ms.saturating_sub(mono_ms),
            )?;
            start = Moment {
                wall_ms: boundary,
                mono_ms,
            };
            self.started = Some(start);
            self.checkpoint_mono = now.mono_ms;
            self.daily_cache.borrow_mut().take();
            self.stored.borrow_mut().take();
        }
        Ok(())
    }
    fn start(&mut self, now: Moment, activity: &Activity, switching: Option<u64>) -> Result<()> {
        self.store.check_startable(activity)?;
        self.store.begin(now.wall_ms, activity, switching)?;
        self.started = Some(now);
        self.checkpoint_mono = now.mono_ms;
        Ok(())
    }
    fn stop(&mut self, now: Moment, elapsed: u64, reason: &str) -> Result<()> {
        self.store.finish(now.wall_ms, elapsed, reason)?;
        self.started = None;
        Ok(())
    }
}
