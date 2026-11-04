//! Durable history: sessions, selection, preferences and recent use.
use super::{
    Activity, ChangeMode, Moment, RecentUse, Result, Snapshot, bounded, invalid, migrate, now_ms,
};
use crate::reports;
use rusqlite::{Connection, OptionalExtension, params};
use std::{path::Path, time::Duration};

pub struct Store {
    pub(super) connection: Connection,
}
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(2))?;
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")?;
        migrate::upgrade(&mut connection, path)?;
        connection.execute("UPDATE focus_sessions SET state='interrupted', ended_ms=checkpoint_ms, end_reason='crash' WHERE state='running'", [])?;
        let store = Self { connection };
        store.snapshot()?;
        Ok(store)
    }
    pub(super) fn selected(&self) -> Result<Activity> {
        let text: String = self.connection.query_row(
            "SELECT activity FROM focus_selection WHERE singleton=1",
            [],
            |r| r.get(0),
        )?;
        let activity: Activity = serde_json::from_str(&text)?;
        activity.validate()?;
        Ok(activity)
    }
    pub(super) fn select(&self, activity: &Activity) -> Result<()> {
        activity.validate()?;
        self.connection.execute(
            "UPDATE focus_selection SET activity=?1 WHERE singleton=1",
            [serde_json::to_string(activity)?],
        )?;
        Ok(())
    }
    pub(super) fn change_mode(&self) -> Result<ChangeMode> {
        let split: bool = self.connection.query_row(
            "SELECT split_on_change FROM focus_preferences WHERE singleton=1",
            [],
            |r| r.get(0),
        )?;
        Ok(if split {
            ChangeMode::NewInterval
        } else {
            ChangeMode::EditCurrent
        })
    }
    pub(super) fn split_on_day(&self) -> Result<bool> {
        Ok(self.connection.query_row(
            "SELECT split_on_day FROM focus_preferences WHERE singleton=1",
            [],
            |r| r.get(0),
        )?)
    }
    /// Close the running interval at a day start and continue its activity from there.
    /// The continuation stays durable up to `now`, like any other checkpoint.
    pub(super) fn split_day(
        &mut self,
        boundary: i64,
        closed: u64,
        now: i64,
        running: u64,
    ) -> Result<()> {
        let tx = self.connection.transaction()?;
        let activity: String = tx
            .query_row(
                "SELECT activity FROM focus_sessions WHERE state='running' AND started_ms<?1",
                [boundary],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| invalid("No active interval crosses this day start."))?;
        tx.execute("UPDATE focus_sessions SET state='stopped', ended_ms=?1, checkpoint_ms=?1, elapsed_ms=?2, end_reason='day_boundary' WHERE state='running'", params![boundary, bounded(closed)])?;
        tx.execute("INSERT INTO focus_sessions(started_ms,checkpoint_ms,elapsed_ms,state,activity) VALUES(?1,?2,?3,'running',?4)", params![boundary, now, bounded(running), activity])?;
        tx.commit()?;
        Ok(())
    }
    pub(super) fn remember(connection: &Connection, activity: &Activity) -> Result<()> {
        activity.validate()?;
        connection.execute("INSERT INTO focus_recent(activity_key,used_order) VALUES(?1,(SELECT COALESCE(MAX(used_order),0)+1 FROM focus_recent)) ON CONFLICT(activity_key) DO UPDATE SET used_order=excluded.used_order", [serde_json::to_string(&activity.key())?])?;
        Ok(())
    }
    pub(super) fn reassign(&mut self, activity: &Activity, now: i64, elapsed: u64) -> Result<()> {
        activity.validate()?;
        let text = serde_json::to_string(activity)?;
        let tx = self.connection.transaction()?;
        let changed = tx.execute("UPDATE focus_sessions SET activity=?1,checkpoint_ms=?2,elapsed_ms=?3 WHERE state='running' AND started_ms<=?2", params![text, now, bounded(elapsed)])?;
        if changed != 1 {
            return Err(invalid("No active interval can be edited at this time.").into());
        }
        tx.execute(
            "UPDATE focus_selection SET activity=?1 WHERE singleton=1",
            [text],
        )?;
        Self::remember(&tx, activity)?;
        tx.commit()?;
        Ok(())
    }
    pub(super) fn begin(
        &mut self,
        now: i64,
        activity: &Activity,
        switching: Option<u64>,
    ) -> Result<()> {
        activity.validate()?;
        let text = serde_json::to_string(activity)?;
        let tx = self.connection.transaction()?;
        let review: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM focus_sessions WHERE state='interrupted')",
            [],
            |r| r.get(0),
        )?;
        if review {
            return Err(invalid("Review interrupted time before starting.").into());
        }
        let previous: Option<i64> =
            tx.query_row("SELECT MAX(ended_ms) FROM focus_sessions", [], |r| r.get(0))?;
        if previous.is_some_and(|end| now < end) {
            return Err(invalid("The clock is earlier than saved history. Correct the system clock before starting.").into());
        }
        if let Some(elapsed) = switching {
            let changed = tx.execute("UPDATE focus_sessions SET state='stopped', ended_ms=?1, checkpoint_ms=?1, elapsed_ms=?2, end_reason='switch' WHERE state='running' AND started_ms<=?1", params![now, bounded(elapsed)])?;
            if changed != 1 {
                return Err(invalid("No active session can be switched at this time.").into());
            }
        }
        tx.execute("INSERT INTO focus_sessions(started_ms,checkpoint_ms,state,activity) VALUES(?1,?1,'running',?2)", params![now, text])?;
        tx.execute(
            "UPDATE focus_selection SET activity=?1 WHERE singleton=1",
            [text],
        )?;
        Self::remember(&tx, activity)?;
        tx.commit()?;
        Ok(())
    }
    pub(super) fn finish(&mut self, now: i64, elapsed: u64, reason: &str) -> Result<()> {
        let tx = self.connection.transaction()?;
        let changed = tx.execute("UPDATE focus_sessions SET state='stopped', ended_ms=?1, checkpoint_ms=?1, elapsed_ms=?2, end_reason=?3 WHERE state='running' AND started_ms<=?1", params![now, bounded(elapsed), reason])?;
        if changed != 1 {
            return Err(invalid("No active session can be stopped at this time.").into());
        }
        tx.commit()?;
        Ok(())
    }
    pub(super) fn checkpoint(&self, now: i64, elapsed: u64) -> Result<()> {
        self.connection.execute(
            "UPDATE focus_sessions SET checkpoint_ms=?1, elapsed_ms=?2 WHERE state='running'",
            params![now, bounded(elapsed)],
        )?;
        Ok(())
    }
    pub(super) fn interrupt(&self, reason: &str) -> Result<()> {
        self.connection.execute("UPDATE focus_sessions SET state='interrupted', ended_ms=checkpoint_ms, end_reason=?1 WHERE state='running'", [reason])?;
        Ok(())
    }
    pub(super) fn accept_recovery(&self) -> Result<()> {
        self.connection.execute(
            "UPDATE focus_sessions SET state='stopped' WHERE state='interrupted'",
            [],
        )?;
        Ok(())
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        let mut recent = Vec::new();
        let mut statement = self
            .connection
            .prepare("SELECT activity_key,used_order FROM focus_recent ORDER BY used_order DESC")?;
        let rows =
            statement.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        for row in rows {
            let (key, order) = row?;
            recent.push(RecentUse {
                key: serde_json::from_str(&key)?,
                order,
            });
        }
        let active: Option<(i64, i64)> = self
            .connection
            .query_row(
                "SELECT elapsed_ms,started_ms FROM focus_sessions WHERE state='running'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (total, count): (i64,i64) = self.connection.query_row("SELECT COALESCE(SUM(elapsed_ms),0), COUNT(*) FROM focus_sessions WHERE state='stopped'", [], |r| Ok((r.get(0)?,r.get(1)?)))?;
        let review = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM focus_sessions WHERE state='interrupted')",
            [],
            |r| r.get(0),
        )?;
        Ok(Snapshot {
            running: active.is_some(),
            active_start_ms: active.map(|a| a.1),
            elapsed_ms: active.map_or(0, |a| a.0).max(0) as u64,
            total_ms: total.max(0) as u64,
            daily_project_ms: None,
            daily_all_ms: None,
            completed: count.max(0) as u64,
            needs_review: review,
            activity: self.selected()?,
            change_mode: self.change_mode()?,
            split_on_day: self.split_on_day()?,
            recent,
            projects: self.catalog()?,
            legacy_projects: migrate::legacy_projects(&self.connection)?,
        })
    }

    pub(super) fn report(
        &self,
        request: &reports::Request,
        now: Moment,
        active_ms: u64,
    ) -> Result<reports::Report> {
        let (range, records) = self.report_records(request, now, Some(active_ms))?;
        let mut report = reports::compute(range, records, now.wall_ms);
        report.use_current_titles(&self.current_titles()?);
        Ok(report)
    }
    pub(super) fn report_records(
        &self,
        request: &reports::Request,
        now: Moment,
        active_ms: Option<u64>,
    ) -> Result<(reports::Range, Vec<reports::Record>)> {
        let range = reports::Range::new(request)?;
        let mut statement = self.connection.prepare("SELECT id,started_ms,COALESCE(ended_ms,?3),elapsed_ms,state,activity,COALESCE(end_reason,''),checkpoint_ms FROM focus_sessions WHERE started_ms<?2 AND (ended_ms IS NULL OR ended_ms>=?1) ORDER BY id")?;
        let raw =
            statement.query_map(params![range.start_ms, range.end_ms, now.wall_ms], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            })?;
        let raw = raw.collect::<std::result::Result<Vec<_>, _>>()?;
        drop(statement);
        // Release the export snapshot's read lock before JSON decoding or output formatting.
        if active_ms.is_none() {
            self.connection.execute_batch("COMMIT")?;
        }
        let mut records = Vec::new();
        for (id, start_ms, mut end_ms, elapsed, state, activity, reason, checkpoint) in raw {
            let state = match state.as_str() {
                "stopped" => reports::State::Saved,
                "running" => reports::State::Running,
                "interrupted" => reports::State::Review,
                _ => return Err(invalid("Unknown history state.").into()),
            };
            let activity: Activity = serde_json::from_str(&activity)?;
            activity.validate()?;
            if state == reports::State::Running && active_ms.is_none() {
                end_ms = checkpoint;
            }
            records.push(reports::Record {
                id,
                start_ms,
                end_ms,
                elapsed_ms: if state == reports::State::Running {
                    active_ms.unwrap_or(elapsed.max(0) as u64)
                } else {
                    elapsed.max(0) as u64
                },
                state,
                activity,
                reason,
            });
        }
        Ok((range, records))
    }
}

/// Read one immutable checkpoint snapshot without migrations, recovery writes or UI row limits.
pub fn export_snapshot(path: &Path, request: &reports::Request) -> Result<reports::Report> {
    let connection = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    connection.busy_timeout(Duration::from_millis(250))?;
    if connection.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))?
        != migrate::SCHEMA_VERSION
    {
        return Err(invalid("Unsupported history version for export.").into());
    }
    connection.execute_batch("BEGIN")?;
    let _: Option<i64> = connection
        .query_row("SELECT id FROM focus_sessions LIMIT 1", [], |r| r.get(0))
        .optional()?;
    let now = Moment {
        wall_ms: now_ms(),
        mono_ms: 0,
    };
    let store = Store { connection };
    let titles = store.current_titles()?;
    let (range, records) = store.report_records(request, now, None)?;
    let mut report = reports::compute_all(range, records, now.wall_ms);
    report.use_current_titles(&titles);
    Ok(report)
}
