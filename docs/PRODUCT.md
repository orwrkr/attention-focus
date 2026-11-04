# Product Scope

## Confirmed by the owner

- **Tray first.** A Windows desktop tool that stays in the system tray. A click shows and raises
  the window, a double click starts or stops tracking, and hovering shows session and daily time.
- **Its own list.** Projects and tasks live in the application, without any external source.
  Pickers show the most recently tracked choice first, and each project remembers its notes.
- **Sound accounting.** A project or task change, and crossing local midnight, each start a new
  interval by default, with a persistent opt-out. Windows session end saves tracking.
- **Reports.** Time by project, task and day for any period and time zone, to the minute. Periods
  can follow the current day. Charts show a timeline, days and a weekly rhythm. Open reports
  refresh on their own, and export to CSV and Markdown.
- **Calm interface.** Window placement, sections, filters and note drafts are restored without
  restarting tracking. Stay on top and sign-in startup are optional. Motion is restrained and
  follows Windows settings. Hide to tray and Exit stay visible above scrolling content. Light
  and dark themes follow Windows or a choice in Settings, and scroll bars never cover content.
- **Honest icons.** A neutral logo identifies the application everywhere. Only the tray icon and
  the window's status show whether tracking is running.
- **Separate profiles.** Production and Development have independent data and can run together.
- **Interface languages.** Selectable, following the Windows language by default and falling back
  to English. The owner has not chosen the set yet (T-0039), so the interface is English for now.
- **Releases.** A Rust implementation under the MIT License. English-only project content.
  Consistent version metadata, and a versioned ZIP holding the EXE, an English guide
  PDF and the required notices. Main builds also produce the development EXE.
- **Public project.** A public GitHub repository named `attention-focus`, and free code signing
  through SignPath Foundation once the project qualifies (T-0037).

Windows is the only target. Other operating systems are not currently required.

The [user guide](user-guide.md) describes the resulting behavior in detail, and the ADRs record
why. Earlier versions of this file list the individual requirements.

## Deferred

- Correcting recorded sessions and adding missed intervals by hand (T-0019).
- Global shortcuts, an installer, and an automatic updater.
- Renaming tasks with their history.
- Real Windows validation of sleep, multi-monitor geometry and clean machines (T-0002).

## Accounting principles

Only one session is active at a time; switching must not double-count time. Time represents
user-declared attention, not automatically measured concentration. The application adds no
cloud services, telemetry, window-content monitoring or productivity scoring.

Project names and notes keep their language and characters; application-owned controls,
messages, and report headings are English.
