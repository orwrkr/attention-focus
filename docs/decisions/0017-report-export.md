# ADR-0017: Background Immutable Report Export

Date: 2026-10-27.
Status: accepted.

Use the existing windows-sys GetSaveFileNameW API on a dedicated thread with the app
window as owner. Enable its dialogs feature, without a new crate. OFN_NOTESTFILECREATE
is mandatory: the native picker must not probe-create a file inside MetaPlan before
our containment validation. Use OFN_NOCHANGEDIR and OFN_DONTADDTORECENT. A canceled
picker creates no export. Existing destinations are rejected, never overwritten.

Read an independent read-only SQLite transaction, without Store::open, migration or
recovery writes. Copy raw rows and release the read transaction before JSON decoding,
aggregation, user interaction or output IO. Tracking keeps its independent worker;
SQLite may briefly serialize writes with the initial read. Export every match, not
the UI's 200-row detail limit. Running rows use their durable checkpoint, explicitly
named Running checkpoint, rather than combining stored elapsed with a newer end time.
Snapshot filters/timezone stay immutable even if UI controls later change.

CSV has one header, a Summary row and Session rows. Each repeats report metadata and
global totals; sum only Clipped ms of Saved Session rows, not repeated total columns.
Include integer milliseconds, state, source identity, original activity/note, full
timestamps with offsets and diagnostics. Use RFC-4180 field quoting and doubled quotes,
UTF-8 without BOM and project-policy LF records. Prefix potentially formula-like cells
with an apostrophe; this changes representation, not source data. It is a conservative
text-import guard, not a universal guarantee after spreadsheet editing/re-saving.
Markdown includes global/project/task/day totals and all session details, escaping
imported syntax and HTML. Both separate saved/running/review values.

Protect all registered source paths and historical source identities plus shared app
storage. Resolve destination parents/junctions before any write, validate again before
publishing a flushed same-directory temporary file with persist_noclobber. Stored
canonical source identities remain lexical guards when disconnected. Concurrent hostile
directory-link replacement remains outside the documented threat model. Test synthetic
history integrity, all rows, checkpoint consistency, containment aliases, no-clobber,
output escaping and failure propagation. Do not use real Production for experiments.

## Sources

- [OPENFILENAMEW flags and buffers](https://learn.microsoft.com/en-us/windows/win32/api/commdlg/ns-commdlg-openfilenamew)
- [CSV quoting](https://www.rfc-editor.org/info/rfc4180/)
- [Formula-like CSV cells and limitations](https://community.owasp.org/attacks/CSV_Injection)
- Installed windows-sys 0.61 dialog definitions and rusqlite 0.41 read-only API.
