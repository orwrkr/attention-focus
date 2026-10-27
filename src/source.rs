//! Read-only MetaPlan snapshots and isolated source settings.

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::{
    collections::HashMap,
    fs, io,
    path::{Component, Path, PathBuf},
};

// Literal MetaPlan protocol labels: project, status, focus, current cycle.
const PROJECT: &str = "\u{41f}\u{440}\u{43e}\u{435}\u{43a}\u{442}:";
const STATUS: &str = "\u{421}\u{442}\u{430}\u{442}\u{443}\u{441}";
const FOCUS: &str = "\u{424}\u{43e}\u{43a}\u{443}\u{441}:";
const CYCLE: &str =
    "\u{422}\u{435}\u{43a}\u{443}\u{449}\u{438}\u{439} \u{446}\u{438}\u{43a}\u{43b}";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Debug)]
pub struct Selection {
    pub id: i64,
    pub path: PathBuf,
}
#[derive(Debug)]
pub struct Project {
    pub slug: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub archived: bool,
    pub tasks: Vec<String>,
}
#[derive(Default, Debug)]
pub struct Catalog {
    pub projects: Vec<Project>,
    pub diagnostics: Vec<String>,
}

fn invalid(message: &str) -> io::Error {
    io::Error::other(message)
}

/// Resolve existing ancestors, including junctions, before appending missing components.
/// Parent components are rejected rather than guessed across a missing directory.
pub fn resolve(path: &Path) -> io::Result<PathBuf> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(invalid("Use an absolute path without parent components."));
    }
    match fs::canonicalize(path) {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let parent = path.parent().ok_or(error)?;
            Ok(resolve(parent)?.join(path.file_name().ok_or_else(|| invalid("Invalid path."))?))
        }
        Err(error) => Err(error),
    }
}

fn key(path: &Path) -> PathBuf {
    PathBuf::from(path.as_os_str().to_string_lossy().to_lowercase())
}

pub fn known_paths(directory: &Path) -> Result<Vec<PathBuf>> {
    check_storage(directory)?;
    let path = directory.join("source.sqlite3");
    if !path.try_exists()? {
        return Ok(Vec::new());
    }
    let connection = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut statement = connection.prepare("SELECT path FROM sources")?;
    Ok(statement
        .query_map([], |r| r.get::<_, String>(0))?
        .map(|r| r.map(PathBuf::from))
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

pub fn validate_boundary(source: &Path, storage: &Path) -> io::Result<PathBuf> {
    let source = resolve(source)?;
    let destination = resolve(storage)?;
    if key(&source).starts_with(key(&destination)) || key(&destination).starts_with(key(&source)) {
        return Err(invalid(
            "MetaPlan and application storage must not contain one another.",
        ));
    }
    Ok(source)
}

fn linked(path: &Path) -> io::Result<bool> {
    let metadata = fs::symlink_metadata(path)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        Ok(metadata.file_attributes() & 0x400 != 0)
    }
    #[cfg(not(windows))]
    {
        Ok(metadata.file_type().is_symlink())
    }
}

/// Reject redirected app files (including SQLite journals) before writable initialization.
fn check_storage(directory: &Path) -> io::Result<()> {
    match fs::read_dir(directory) {
        Ok(entries) => {
            for entry in entries {
                let path = entry?.path();
                if linked(&path)? {
                    return Err(invalid("Application storage contains a linked entry."));
                }
                if path.is_dir() {
                    check_storage(&path)?;
                }
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => (),
        Err(error) => return Err(error),
    }
    Ok(())
}

/// Read settings without creating a database or running migrations.
pub fn restore(directory: &Path) -> Result<Option<Selection>> {
    check_storage(directory)?;
    restore_source_only(directory)
}

/// Read only the legacy settings file; unrelated profile directories stay independent.
pub fn restore_source_only(directory: &Path) -> Result<Option<Selection>> {
    let file = directory.join("source.sqlite3");
    if !file.try_exists()? {
        return Ok(None);
    }
    if linked(&file)? {
        return Err(invalid("Linked source settings are not supported.").into());
    }
    let db = Connection::open_with_flags(file, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let selected: Option<(i64, String)> = db.query_row(
        "SELECT id, path FROM sources WHERE id=(SELECT source_id FROM selection WHERE singleton=1)",
        [], |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional()?;
    selected
        .map(|(id, path)| {
            Ok(Selection {
                id,
                path: validate_boundary(Path::new(&path), directory)?,
            })
        })
        .transpose()
}

pub fn save(directory: &Path, source: &Path) -> Result<Selection> {
    check_storage(directory)?;
    let path = validate_boundary(source, directory)?;
    check_layout(&path)?;
    fs::create_dir_all(directory)?;
    let mut db = Connection::open(directory.join("source.sqlite3"))?;
    let tx = db.transaction()?;
    tx.execute_batch("CREATE TABLE IF NOT EXISTS sources(id INTEGER PRIMARY KEY, path TEXT NOT NULL UNIQUE);
        CREATE TABLE IF NOT EXISTS selection(singleton INTEGER PRIMARY KEY CHECK(singleton=1), source_id INTEGER NOT NULL REFERENCES sources(id));")?;
    let text = path
        .to_str()
        .ok_or_else(|| invalid("Source path is not valid Unicode."))?;
    tx.execute("INSERT OR IGNORE INTO sources(path) VALUES (?1)", [text])?;
    let id = tx.query_row("SELECT id FROM sources WHERE path=?1", [text], |r| r.get(0))?;
    tx.execute("INSERT INTO selection VALUES (1, ?1) ON CONFLICT(singleton) DO UPDATE SET source_id=excluded.source_id", [id])?;
    tx.commit()?;
    Ok(Selection { id, path })
}

fn check_layout(root: &Path) -> io::Result<()> {
    for path in [root.join("PROJECTS"), root.join("PROJECTS/Active")] {
        if linked(&path)? || !path.is_dir() {
            return Err(invalid(
                "Expected ordinary PROJECTS/Active directories inside Instance.",
            ));
        }
    }
    Ok(())
}

fn read_text(path: &Path) -> io::Result<String> {
    if linked(path)? || !path.is_file() {
        return Err(invalid("Linked or non-file source entry skipped."));
    }
    if fs::metadata(path)?.len() > 2_000_000 {
        return Err(invalid("Source file exceeds the 2 MB limit."));
    }
    fs::read_to_string(path)
}

fn parse_project(slug: String, text: &str, archived: bool) -> io::Result<Project> {
    let heading = text
        .lines()
        .find_map(|l| l.trim_start_matches('\u{feff}').strip_prefix("# "))
        .ok_or_else(|| invalid("Missing project heading."))?;
    let title = heading
        .strip_prefix(PROJECT)
        .unwrap_or(heading)
        .trim()
        .to_string();
    if title.is_empty() {
        return Err(invalid("Empty project title."));
    }
    let mut in_status = false;
    let mut status = None;
    let mut priority = String::from("Unspecified");
    for line in text.lines().map(str::trim) {
        if let Some(heading) = line.strip_prefix("## ") {
            in_status = heading == STATUS;
            continue;
        }
        if in_status && !line.is_empty() && status.is_none() {
            status = Some(line.to_string());
        }
        if let Some(value) = line.strip_prefix(FOCUS) {
            priority = value.trim().to_string();
        }
    }
    let status = status
        .filter(|s| ["Active", "Paused", "Completed", "Suspended"].contains(&s.as_str()))
        .ok_or_else(|| invalid("Missing or unsupported project status."))?;
    Ok(Project {
        slug,
        title,
        status,
        priority,
        archived,
        tasks: Vec::new(),
    })
}

fn parse_tasks(text: &str) -> Vec<String> {
    let mut active = false;
    let mut section = String::new();
    let mut tasks = Vec::new();
    let mut fenced = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with("```") || line.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        if let Some(heading) = line.strip_prefix("## ") {
            active = heading == CYCLE
                || heading
                    .strip_prefix(CYCLE)
                    .is_some_and(|s| s.starts_with(" ("));
            section.clear();
        } else if let Some(heading) = line.strip_prefix("### ") {
            section = heading.to_string();
        } else if active
            && let Some(task) = line
                .strip_prefix("- [ ]")
                .or_else(|| line.strip_prefix("* [ ]"))
        {
            let task = task.trim();
            if !task.is_empty() {
                tasks.push(if section.is_empty() {
                    task.into()
                } else {
                    format!("{section}: {task}")
                });
            }
        }
    }
    tasks
}

pub fn read(root: &Path) -> Result<Catalog> {
    check_layout(root)?;
    let mut catalog = Catalog::default();
    let mut counts = HashMap::new();
    for (folder, archived) in [("Active", false), ("Archive", true)] {
        let directory = root.join("PROJECTS").join(folder);
        if !directory.try_exists()? {
            continue;
        }
        if linked(&directory)? {
            catalog
                .diagnostics
                .push(format!("{folder}: linked directory skipped."));
            continue;
        }
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            if linked(&path)? {
                catalog.diagnostics.push(format!(
                    "{}: linked entry skipped.",
                    entry.file_name().to_string_lossy()
                ));
                continue;
            }
            if !path.is_dir() {
                continue;
            }
            let slug = entry.file_name().to_string_lossy().into_owned();
            *counts.entry(slug.to_lowercase()).or_insert(0) += 1;
            match read_text(&path.join("project.md"))
                .and_then(|text| parse_project(slug.clone(), &text, archived))
            {
                Ok(mut project) => {
                    match read_text(&path.join("tasks.md")) {
                        Ok(text) => project.tasks = parse_tasks(&text),
                        Err(error) => catalog
                            .diagnostics
                            .push(format!("{slug}/tasks.md: {error}")),
                    }
                    catalog.projects.push(project);
                }
                Err(error) => catalog
                    .diagnostics
                    .push(format!("{slug}/project.md: {error}")),
            }
        }
    }
    catalog
        .projects
        .retain(|p| counts[&p.slug.to_lowercase()] == 1);
    for (slug, count) in counts {
        if count > 1 {
            catalog
                .diagnostics
                .push(format!("{slug}: duplicate slug excluded."));
        }
    }
    catalog.projects.sort_by(|a, b| {
        a.archived
            .cmp(&b.archived)
            .then_with(|| a.title.cmp(&b.title))
    });
    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tree(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        fn visit(root: &Path, path: &Path, output: &mut Vec<(PathBuf, Vec<u8>)>) {
            for entry in fs::read_dir(path).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    visit(root, &path, output);
                } else {
                    output.push((
                        path.strip_prefix(root).unwrap().to_owned(),
                        fs::read(path).unwrap(),
                    ));
                }
            }
        }
        let mut output = Vec::new();
        visit(root, root, &mut output);
        output.sort();
        output
    }
    fn fixture(root: &Path, folder: &str, slug: &str) {
        let directory = root.join("PROJECTS").join(folder).join(slug);
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("project.md"),
            format!("# {PROJECT} Example \u{1f331}\n## {STATUS}\nActive\n{FOCUS} Primary\n"),
        )
        .unwrap();
        fs::write(directory.join("tasks.md"), format!("## {CYCLE} (2026)\n- [ ] First\n- [ ]\n- [x] Finished\n### Design\n- [ ] Draft\n## History\n- [ ] Old\n")).unwrap();
    }
    #[test]
    fn reader_preserves_source_and_filters_tasks_and_duplicates() {
        let temp = tempfile::tempdir().unwrap();
        fixture(temp.path(), "Active", "sample");
        fixture(temp.path(), "Archive", "old");
        let path = temp.path().join("PROJECTS/Active/sample/tasks.md");
        let before = fs::read(&path).unwrap();
        let before_tree = tree(temp.path());
        let catalog = read(temp.path()).unwrap();
        assert_eq!(before_tree, tree(temp.path()));
        assert_eq!(catalog.projects.len(), 2);
        assert_eq!(catalog.projects[0].title, "Example \u{1f331}");
        assert_eq!(catalog.projects[0].tasks, ["First", "Design: Draft"]);
        assert!(catalog.projects[1].archived);
        assert_eq!(before, fs::read(path).unwrap());
        fixture(temp.path(), "Archive", "sample");
        assert_eq!(read(temp.path()).unwrap().projects.len(), 1);
    }
    #[test]
    fn settings_restore_and_reject_overlap_without_replacing_selection() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let storage = temp.path().join("source-data");
        fixture(&source, "Active", "sample");
        let before = tree(&source);
        let selected = save(&storage, &source).unwrap();
        assert_eq!(before, tree(&source));
        assert_eq!(restore(&storage).unwrap().unwrap().id, selected.id);
        assert!(save(&storage, temp.path()).is_err());
        assert!(validate_boundary(&source, &source.join("data")).is_err());
        assert!(validate_boundary(&storage.join("nested"), &storage).is_err());
        assert_eq!(restore(&storage).unwrap().unwrap().path, selected.path);
        let another = temp.path().join("another");
        fixture(&another, "Active", "sample");
        assert_ne!(save(&storage, &another).unwrap().id, selected.id);
        assert_eq!(save(&storage, &source).unwrap().id, selected.id);
        fs::remove_dir_all(&source).unwrap();
        assert!(restore(&storage).unwrap().is_some());
    }

    #[test]
    fn invalid_source_creates_no_storage_and_corrupt_settings_stay_unchanged() {
        let temp = tempfile::tempdir().unwrap();
        let storage = temp.path().join("data");
        assert!(save(&storage, &temp.path().join("missing")).is_err());
        assert!(!storage.exists());
        fs::create_dir(&storage).unwrap();
        let settings = storage.join("source.sqlite3");
        fs::write(&settings, b"Not a database").unwrap();
        assert!(restore(&storage).is_err());
        assert_eq!(fs::read(settings).unwrap(), b"Not a database");
    }
    #[test]
    fn malformed_and_missing_files_are_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        fixture(temp.path(), "Active", "sample");
        fs::remove_file(temp.path().join("PROJECTS/Active/sample/tasks.md")).unwrap();
        let catalog = read(temp.path()).unwrap();
        assert_eq!(catalog.projects.len(), 1);
        assert_eq!(catalog.diagnostics.len(), 1);
        fs::write(
            temp.path().join("PROJECTS/Active/sample/project.md"),
            "Invalid",
        )
        .unwrap();
        assert!(read(temp.path()).unwrap().projects.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn case_and_junction_aliases_cannot_bypass_boundary() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        fixture(&source, "Active", "sample");
        assert!(
            validate_boundary(
                &source,
                &PathBuf::from(source.to_string_lossy().to_uppercase()).join("data")
            )
            .is_err()
        );
        let alias = temp.path().join("alias");
        let status = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:AF_TEST_LINK -Target $env:AF_TEST_TARGET | Out-Null"])
            .env("AF_TEST_LINK", &alias).env("AF_TEST_TARGET", &source).status().unwrap();
        assert!(
            status.success(),
            "Junction creation must succeed to verify this Windows boundary."
        );
        assert!(validate_boundary(&source, &alias.join("data")).is_err());
        assert!(validate_boundary(&alias, &source).is_err());
        fs::remove_dir(&alias).unwrap();

        let project_link = source.join("PROJECTS/Active/linked");
        let status = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:AF_TEST_LINK -Target $env:AF_TEST_TARGET | Out-Null"])
            .env("AF_TEST_LINK", &project_link).env("AF_TEST_TARGET", source.join("PROJECTS/Active/sample")).status().unwrap();
        assert!(status.success());
        let catalog = read(&source).unwrap();
        assert_eq!(catalog.projects.len(), 1);
        assert!(
            catalog
                .diagnostics
                .iter()
                .any(|d| d.contains("linked entry skipped"))
        );
        fs::remove_dir(&project_link).unwrap();
    }
}
