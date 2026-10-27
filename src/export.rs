//! Immutable report exports. Destination validation precedes every filesystem write.
use crate::{
    reports::{Report, State},
    source,
};
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug)]
pub enum Format {
    Csv,
    Markdown,
}
impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Markdown => "md",
        }
    }
}
fn csv(value: &str) -> String {
    let first = value.trim_start().chars().next();
    let guard = matches!(
        first,
        Some('=' | '+' | '-' | '@' | '\u{ff1d}' | '\u{ff0b}' | '\u{ff0d}' | '\u{ff20}')
    ) || value.starts_with(['\t', '\r', '\n']);
    format!(
        "\"{}{}\"",
        if guard { "'" } else { "" },
        value.replace('"', "\"\"")
    )
}
fn markdown(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '|' => out.push_str("&#124;"),
            '\n' => out.push_str("<br>"),
            '\r' => out.push_str("&#13;"),
            '\\' | '`' | '*' | '_' | '[' | ']' | '!' | '#' | '~' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}
fn state(state: State) -> &'static str {
    match state {
        State::Saved => "Saved",
        State::Running => "Running checkpoint",
        State::Review => "Needs review",
    }
}
fn row(out: &mut impl Write, values: &[String], format: Format) -> io::Result<()> {
    match format {
        Format::Csv => writeln!(
            out,
            "{}",
            values.iter().map(|s| csv(s)).collect::<Vec<_>>().join(",")
        ),
        Format::Markdown => writeln!(
            out,
            "| {} |",
            values
                .iter()
                .map(|s| markdown(s))
                .collect::<Vec<_>>()
                .join(" | ")
        ),
    }
}
pub fn render(out: &mut impl Write, report: &Report, format: Format) -> io::Result<()> {
    let request = &report.range.request;
    let metadata = vec![
        env!("CARGO_PKG_VERSION").into(),
        report.range.format_time(report.generated_ms),
        request.timezone.clone(),
        format!("{} {}", request.from, request.from_time),
        format!("{} {}", request.through, request.through_time),
        report.saved_ms.to_string(),
        report.running_ms.to_string(),
        report.review_ms.to_string(),
        report.diagnostics.join("; "),
    ];
    if matches!(format, Format::Markdown) {
        writeln!(
            out,
            "# Attention Focus report\n\nVersion: {}\n\nSnapshot: {}\n\nRange: {} to {} (end excluded), {}\n\nSaved: {} ms | Running checkpoint: {} ms | Needs review: {} ms\n\nRunning time uses its last durable checkpoint, not a finalized session.\n",
            metadata[0],
            markdown(&metadata[1]),
            markdown(&metadata[3]),
            markdown(&metadata[4]),
            markdown(&metadata[2]),
            metadata[5],
            metadata[6],
            metadata[7]
        )?;
        for (title, groups) in [
            ("By project", &report.projects),
            ("By project and task", &report.groups),
        ] {
            writeln!(
                out,
                "## {title}\n\n| Project | Task | Source | Saved ms |\n| --- | --- | --- | --- |"
            )?;
            for g in groups {
                row(
                    out,
                    &[
                        g.project.clone(),
                        g.task.clone().unwrap_or_default(),
                        g.source.clone().unwrap_or_default(),
                        g.saved_ms.to_string(),
                    ],
                    format,
                )?;
            }
            writeln!(out)?;
        }
        writeln!(out, "## By day\n\n| Day | Saved ms |\n| --- | --- |")?;
        for day in &report.range.days {
            row(out, &[day.date.clone(), day.saved_ms.to_string()], format)?;
        }
        writeln!(out, "\n## Sessions\n")?;
    }
    let session_columns = [
        "Session ID",
        "State",
        "Source ID",
        "Source path",
        "Project slug",
        "Project",
        "Task",
        "Note",
        "Started",
        "Ended or checkpoint",
        "Full elapsed ms",
        "Clipped ms",
        "End reason",
    ];
    let mut columns: Vec<String> = if matches!(format, Format::Csv) {
        [
            "Row type",
            "Version",
            "Snapshot",
            "Time zone",
            "From included",
            "Until excluded",
            "Saved total ms",
            "Running checkpoint total ms",
            "Needs review total ms",
            "Diagnostics",
        ]
        .map(String::from)
        .to_vec()
    } else {
        Vec::new()
    };
    columns.extend(session_columns.map(String::from));
    row(out, &columns, format)?;
    if matches!(format, Format::Markdown) {
        writeln!(out, "| {} |", vec!["---"; columns.len()].join(" | "))?;
    }
    if matches!(format, Format::Csv) {
        let mut values = vec!["Summary".into()];
        values.extend(metadata.clone());
        values.resize(columns.len(), String::new());
        row(out, &values, format)?;
    }
    for h in &report.history {
        let r = &h.record;
        let p = r.activity.project.as_ref();
        let mut values = if matches!(format, Format::Csv) {
            let mut v = vec!["Session".into()];
            v.extend(metadata.clone());
            v
        } else {
            Vec::new()
        };
        values.extend([
            r.id.to_string(),
            state(r.state).into(),
            p.map(|p| p.source_id.to_string()).unwrap_or_default(),
            p.map(|p| p.source_path.clone()).unwrap_or_default(),
            p.map(|p| p.slug.clone()).unwrap_or_default(),
            r.activity.label().into(),
            p.and_then(|p| p.task.clone()).unwrap_or_default(),
            r.activity.note.clone(),
            report.range.format_time(r.start_ms),
            report.range.format_time(r.end_ms),
            r.elapsed_ms.to_string(),
            h.clipped_ms.to_string(),
            r.reason.clone(),
        ]);
        row(out, &values, format)?;
    }
    if !report.diagnostics.is_empty() && matches!(format, Format::Markdown) {
        writeln!(out, "\n## Diagnostics\n")?;
        for text in &report.diagnostics {
            writeln!(out, "- {}", markdown(text))?;
        }
    }
    Ok(())
}
fn destination(path: &Path, protected: &[PathBuf]) -> io::Result<PathBuf> {
    let target = source::resolve(path)?;
    if target.try_exists()? {
        return Err(io::Error::other(
            "Choose a new filename; existing files are never overwritten.",
        ));
    }
    let key = target.to_string_lossy().to_lowercase();
    for root in protected {
        // Stored source identities are canonical. A disconnected source still has a
        // lexical identity; an unresolved destination is never allowed.
        let root = source::resolve(root).or_else(|error| {
            if root.is_absolute()
                && !root
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                Ok(root.clone())
            } else {
                Err(error)
            }
        })?;
        if Path::new(&key).starts_with(Path::new(&root.to_string_lossy().to_lowercase())) {
            return Err(io::Error::other(
                "Export outside MetaPlan and application storage.",
            ));
        }
    }
    Ok(target)
}
pub fn write(
    report: &Report,
    path: &Path,
    format: Format,
    protected: &[PathBuf],
) -> io::Result<PathBuf> {
    let target = destination(path, protected)?;
    let parent = target
        .parent()
        .ok_or_else(|| io::Error::other("Choose an export folder."))?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    render(file.as_file_mut(), report, format)?;
    file.as_file_mut().sync_all()?;
    destination(&target, protected)?;
    file.persist_noclobber(&target).map_err(|e| e.error)?;
    Ok(target)
}

#[cfg(windows)]
pub fn pick_path(owner: usize, format: Format) -> io::Result<Option<PathBuf>> {
    use windows_sys::Win32::UI::Controls::Dialogs::*;
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let title = wide("Export Attention Focus report (choose a new filename)");
    let ext = wide(format.extension());
    let filter = wide(&format!("Report files\0*.{}\0", format.extension()));
    let name = format!(
        "AttentionFocus-report-{}.{}",
        jiff::Timestamp::now().strftime("%Y%m%d-%H%M%S"),
        format.extension()
    );
    let mut buffer = vec![0u16; 32_768];
    let name = wide(&name);
    buffer[..name.len()].copy_from_slice(&name);
    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: owner as _,
        lpstrFilter: filter.as_ptr(),
        nFilterIndex: 1,
        lpstrFile: buffer.as_mut_ptr(),
        nMaxFile: buffer.len() as u32,
        lpstrTitle: title.as_ptr(),
        lpstrDefExt: ext.as_ptr(),
        Flags: OFN_EXPLORER
            | OFN_NOCHANGEDIR
            | OFN_PATHMUSTEXIST
            | OFN_NOTESTFILECREATE
            | OFN_DONTADDTORECENT,
        ..Default::default()
    };
    if unsafe { GetSaveFileNameW(&mut dialog) } == 0 {
        let error = unsafe { CommDlgExtendedError() };
        if error == 0 {
            return Ok(None);
        }
        return Err(io::Error::other(format!(
            "Cannot select export file (Windows error {error})."
        )));
    }
    use std::os::windows::ffi::OsStringExt;
    let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
    Ok(Some(PathBuf::from(std::ffi::OsString::from_wide(
        &buffer[..end],
    ))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{reports::Request, tracking};
    fn fixture(count: usize) -> (tempfile::TempDir, PathBuf, Request) {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("app");
        std::fs::create_dir(&app).unwrap();
        let path = app.join("focus.sqlite3");
        drop(tracking::Store::open(&path).unwrap());
        let conn = rusqlite::Connection::open(&path).unwrap();
        let activity = serde_json::to_string(&tracking::Activity {
            project: None,
            note: "Synthetic note".into(),
        })
        .unwrap();
        conn.execute_batch("BEGIN").unwrap();
        for i in 0..count {
            conn.execute("INSERT INTO focus_sessions(started_ms,checkpoint_ms,ended_ms,elapsed_ms,state,activity,end_reason) VALUES(?1,?2,?2,1000,'stopped',?3,'stop')",rusqlite::params![(i*1000) as i64,((i+1)*1000) as i64,activity]).unwrap();
        }
        conn.execute_batch("COMMIT").unwrap();
        drop(conn);
        let request = Request {
            from: "1970-01-01".into(),
            through: "1970-01-01".into(),
            timezone: "UTC".into(),
            from_time: "00:00".into(),
            through_time: "24:00".into(),
        };
        (dir, path, request)
    }
    #[test]
    fn snapshot_exports_every_match_and_checkpoint_categories_without_history_writes() {
        let (_dir, path, request) = fixture(205);
        let conn = rusqlite::Connection::open(&path).unwrap();
        let activity = serde_json::to_string(&tracking::Activity::default()).unwrap();
        conn.execute("INSERT INTO focus_sessions(started_ms,checkpoint_ms,elapsed_ms,state,activity) VALUES(300000,310000,10000,'running',?1)",[&activity]).unwrap();
        conn.execute("INSERT INTO focus_sessions(started_ms,checkpoint_ms,ended_ms,elapsed_ms,state,activity) VALUES(400000,410000,410000,10000,'interrupted',?1)",[&activity]).unwrap();
        drop(conn);
        let before = std::fs::read(&path).unwrap();
        let snapshot = tracking::export_snapshot(&path, &request).unwrap();
        let report = snapshot.report;
        assert_eq!((report.matching_sessions, report.history.len()), (207, 207));
        assert_eq!(
            (report.saved_ms, report.running_ms, report.review_ms),
            (205000, 10000, 10000)
        );
        assert_eq!(
            report
                .history
                .iter()
                .find(|h| h.record.state == State::Running)
                .unwrap()
                .record
                .end_ms,
            310000
        );
        let mut bytes = Vec::new();
        render(&mut bytes, &report, Format::Csv).unwrap();
        let csv = String::from_utf8(bytes).unwrap();
        assert_eq!(
            csv.lines().filter(|l| l.starts_with("\"Session\"")).count(),
            207
        );
        assert!(csv.contains("Running checkpoint") && csv.contains("Needs review"));
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let mut bytes = Vec::new();
        render(&mut bytes, &report, Format::Markdown).unwrap();
        let md = String::from_utf8(bytes).unwrap();
        assert!(md.contains("## By day") && md.contains("## By project and task"));
    }
    #[test]
    fn escaping_keeps_unicode_and_makes_imported_syntax_literal() {
        for value in ["=1", "+1", "-1", "@SUM(A1)", " \u{ff1d}1", "\t=1"] {
            assert!(csv(value).starts_with("\"'"));
        }
        assert_eq!(
            csv("Example, \"text\"\n\u{1f331}"),
            "\"Example, \"\"text\"\"\n\u{1f331}\""
        );
        assert_eq!(
            markdown("<b>|[link]\n\u{1f331}"),
            "&lt;b&gt;&#124;\\[link\\]<br>\u{1f331}"
        );
    }
    #[test]
    fn writing_rejects_sources_storage_aliases_and_overwrites() {
        let (dir, path, request) = fixture(1);
        let source = dir.path().join("source");
        std::fs::create_dir(&source).unwrap();
        let report = tracking::export_snapshot(&path, &request).unwrap().report;
        let protected = vec![source.clone(), path.parent().unwrap().to_path_buf()];
        assert!(
            write(
                &report,
                &source.join("blocked.csv"),
                Format::Csv,
                &protected
            )
            .is_err()
        );
        assert!(
            write(
                &report,
                &path.with_file_name("blocked.csv"),
                Format::Csv,
                &protected
            )
            .is_err()
        );
        assert_eq!(std::fs::read_dir(&source).unwrap().count(), 0);
        let target = dir.path().join("report.csv");
        write(&report, &target, Format::Csv, &protected).unwrap();
        let before = std::fs::read(&target).unwrap();
        assert!(write(&report, &target, Format::Markdown, &protected).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), before);
        assert!(
            write(
                &report,
                &dir.path().join("missing/report.csv"),
                Format::Csv,
                &protected
            )
            .is_err()
        );
        #[cfg(windows)]
        {
            let link = dir.path().join("alias");
            let status=std::process::Command::new("powershell.exe").args(["-NoProfile","-NonInteractive","-Command","$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:AF_EXPORT_LINK -Target $env:AF_EXPORT_TARGET | Out-Null"]).env("AF_EXPORT_LINK",&link).env("AF_EXPORT_TARGET",&source).status().unwrap();
            assert!(status.success());
            assert!(write(&report, &link.join("blocked.csv"), Format::Csv, &protected).is_err());
            assert_eq!(std::fs::read_dir(&source).unwrap().count(), 0);
        }
    }
    #[test]
    fn output_failure_is_propagated() {
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("Synthetic disk error"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let (_dir, path, request) = fixture(0);
        let report = tracking::export_snapshot(&path, &request).unwrap().report;
        assert!(render(&mut Broken, &report, Format::Csv).is_err());
    }
    #[test]
    fn independent_snapshot_can_read_large_history_while_tracking_stops() {
        let (_dir, path, request) = fixture(5000);
        let moment = |ms| tracking::Moment {
            wall_ms: 6_000_000 + ms as i64,
            mono_ms: ms,
        };
        let mut engine = tracking::Engine::new(tracking::Store::open(&path).unwrap(), moment(0));
        engine
            .apply(
                Some(tracking::Command::Start(tracking::Activity::default())),
                moment(0),
            )
            .unwrap();
        engine.apply(None, moment(10000)).unwrap();
        let reader = std::thread::spawn(move || tracking::export_snapshot(&path, &request));
        engine
            .apply(Some(tracking::Command::Stop), moment(12000))
            .unwrap();
        let snapshot = reader.join().unwrap().unwrap();
        assert_eq!(snapshot.report.history.len(), 5001);
        assert!(!engine.snapshot(moment(12000)).unwrap().running);
    }
}
