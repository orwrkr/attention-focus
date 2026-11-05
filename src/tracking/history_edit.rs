//! Atomic corrections of stopped history, with durable before and after snapshots.
use super::{Activity, IntervalEdit, Result, Store, catalog, invalid, reject};
use jiff::{Timestamp, tz::TimeZone};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

struct Stored {
    start: i64,
    end: Option<i64>,
    elapsed: i64,
    reason: Option<String>,
    activity: String,
}
impl Stored {
    /// `recorded` is what the interface showed. IDs can be reused after a deletion, so a row
    /// with other boundaries is a different interval, and must not be changed by mistake.
    fn load(connection: &Connection, id: i64, recorded: Option<(i64, i64)>) -> Result<Self> {
        let row = connection.query_row(
            "SELECT started_ms,ended_ms,elapsed_ms,end_reason,activity,state FROM focus_sessions WHERE id=?1",
            [id], |r| Ok((Self {
                start: r.get(0)?, end: r.get(1)?, elapsed: r.get(2)?,
                reason: r.get(3)?, activity: r.get(4)?,
            }, r.get::<_, String>(5)?)),
        ).optional()?.ok_or_else(|| reject("errors.interval_missing"))?;
        if row.1 != "stopped" {
            return Err(reject("errors.interval_locked").into());
        }
        if recorded.is_some_and(|(start, end)| row.0.start != start || row.0.end != Some(end)) {
            return Err(reject("errors.interval_missing").into());
        }
        Ok(row.0)
    }
    fn snapshot(&self) -> Result<String> {
        Ok(serde_json::to_string(&json!({
            "started_ms": self.start, "ended_ms": self.end, "elapsed_ms": self.elapsed,
            "end_reason": self.reason, "activity": serde_json::from_str::<Value>(&self.activity)?,
        }))?)
    }
}
impl Store {
    pub(super) fn save_interval(&mut self, edit: &IntervalEdit, now: i64) -> Result<()> {
        let tx = self.connection.transaction()?;
        edit.activity.validate()?;
        let old = edit
            .id
            .map(|id| Stored::load(&tx, id, edit.recorded))
            .transpose()?;
        let boundaries = old
            .as_ref()
            .is_none_or(|r| r.start != edit.start_ms || r.end != Some(edit.end_ms));
        if boundaries {
            if edit.start_ms >= edit.end_ms {
                return Err(reject("errors.interval_order").into());
            }
            if edit.end_ms > now {
                return Err(reject("errors.interval_future").into());
            }
            let conflict: Option<(i64, i64)> = tx
                .query_row(
                    "SELECT started_ms,COALESCE(ended_ms,?4) FROM focus_sessions
                 WHERE (?1 IS NULL OR id!=?1) AND started_ms<?3 AND COALESCE(ended_ms,?4)>?2
                 AND COALESCE(ended_ms,?4)>started_ms ORDER BY started_ms,id LIMIT 1",
                    params![edit.id, edit.start_ms, edit.end_ms, now],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            if let Some((start, end)) = conflict {
                let zone = TimeZone::get(&edit.timezone).unwrap_or_else(|_| TimeZone::system());
                let format = |ms| -> Result<String> {
                    Ok(Timestamp::from_millisecond(ms)?
                        .to_zoned(zone.clone())
                        .strftime("%Y-%m-%d %H:%M:%S %:z")
                        .to_string())
                };
                return Err(invalid(&crate::i18n::message(
                    "errors.interval_overlap",
                    &[("start", &format(start)?), ("end", &format(end)?)],
                ))
                .into());
            }
        }
        let recorded = old
            .as_ref()
            .map(|row| serde_json::from_str::<Activity>(&row.activity))
            .transpose()?;
        let activity_changed = recorded.as_ref() != Some(&edit.activity);
        if !boundaries && !activity_changed {
            return Ok(());
        }
        let activity = if activity_changed {
            let changed = match recorded {
                // With the same project and task, only the note differs, so the recorded
                // snapshot stays, even if that project was renamed since.
                Some(recorded) if recorded.key() == edit.activity.key() => Activity {
                    note: edit.activity.note.clone(),
                    ..recorded
                },
                _ => catalog::history_activity(&tx, &edit.activity)?,
            };
            serde_json::to_string(&changed)?
        } else {
            old.as_ref().unwrap().activity.clone()
        };
        let before = old.as_ref().map(Stored::snapshot).transpose()?;
        let after = Stored {
            start: edit.start_ms,
            end: Some(edit.end_ms),
            elapsed: if boundaries {
                edit.end_ms
                    .checked_sub(edit.start_ms)
                    .ok_or_else(|| reject("errors.interval_order"))?
            } else {
                old.as_ref().unwrap().elapsed
            },
            reason: old
                .as_ref()
                .map_or_else(|| Some("manual".into()), |r| r.reason.clone()),
            activity,
        };
        let (id, action) = if let Some(id) = edit.id {
            let changed = if boundaries {
                tx.execute("UPDATE focus_sessions SET started_ms=?2,ended_ms=?3,checkpoint_ms=?3,elapsed_ms=?4,activity=?5 WHERE id=?1 AND state='stopped'",
                    params![id, after.start, after.end, after.elapsed, after.activity])?
            } else {
                tx.execute(
                    "UPDATE focus_sessions SET activity=?2 WHERE id=?1 AND state='stopped'",
                    params![id, after.activity],
                )?
            };
            if changed != 1 {
                return Err(reject("errors.interval_locked").into());
            }
            (id, "update")
        } else {
            tx.execute("INSERT INTO focus_sessions(started_ms,ended_ms,checkpoint_ms,elapsed_ms,state,end_reason,activity) VALUES(?1,?2,?2,?3,'stopped','manual',?4)",
                params![after.start, after.end, after.elapsed, after.activity])?;
            (tx.last_insert_rowid(), "create")
        };
        tx.execute("INSERT INTO focus_session_edits(session_id,action,edited_ms,before,after) VALUES(?1,?2,?3,?4,?5)",
            params![id, action, now, before, after.snapshot()?])?;
        tx.commit()?;
        Ok(())
    }
    pub(super) fn delete_interval(
        &mut self,
        id: i64,
        recorded: (i64, i64),
        now: i64,
    ) -> Result<()> {
        let tx = self.connection.transaction()?;
        let before = Stored::load(&tx, id, Some(recorded))?.snapshot()?;
        if tx.execute(
            "DELETE FROM focus_sessions WHERE id=?1 AND state='stopped'",
            [id],
        )? != 1
        {
            return Err(reject("errors.interval_locked").into());
        }
        tx.execute("INSERT INTO focus_session_edits(session_id,action,edited_ms,before) VALUES(?1,'delete',?2,?3)", params![id, now, before])?;
        tx.commit()?;
        Ok(())
    }
}
