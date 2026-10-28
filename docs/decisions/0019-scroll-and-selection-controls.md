# ADR-0019: Stable Scroll Reflow and Explicit Interaction Feedback

Date: 2026-10-28.
Status: accepted.

The user reported unnecessary/invisible scrollbars, stale duplicated painting after
collapse, missing maximized edges, no primary-button feedback and inconvenient pickers.

Use VisibleWhenNeeded with a consistently visible dormant handle on overflow. Keep
floating scrollbar allocation at zero and reserve symmetric 12-point content margins;
the bar lives in the right margin. Content width no longer changes when overflow
changes, avoiding narrow/wide counter-layout feedback. Repaint once when content size
changes and use an opaque framebuffer clear every frame. Regression tests repeatedly
expand/collapse, scroll, settle, verify overflow/reset and reject duplicate timer paint.

Use scoped standard Button visuals with distinct inactive/hovered/active blue fills,
not an explicit fill that overrides all states. Keep built-in focus, click and disabled
semantics. Test actual painted colors and disabled behavior, rather than palette values.

Windows suppresses ordinary non-client edges while maximized, even when our width cap
leaves the window narrower than the monitor. Paint a visible boundary in native
non-client notifications, plus a one-physical-pixel client outline after egui rendering.
Keep native caption controls, working-area bounds and user-owned height unchanged.
Do not treat a caption being visible as proof that the whole boundary is visible.

Put search in the project popup and keep it open for editing; explicit selection closes
it. Distinguish Unassigned focus with a colored bold italic no-project label while
preserving durable recent-use order. Searching never changes the activity by itself.

Use existing windows-sys Shell APIs for a resizable filesystem-folder browser on its
own STA thread. Initialize COM, release the returned shell item and COM apartment,
preserve UTF-16 paths and hide New Folder. Cancel leaves the draft unchanged; selection
only fills the path. Save and refresh still performs normalization, link resolution
and source/storage boundary validation. No picker code writes files or changes cwd.
This smaller existing-API integration avoids a new GUI dependency; a modern IFileDialog
is an optional later UX replacement. No new distributed DLL or schema change.

References: [BROWSEINFOW flags](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/ns-shlobj_core-browseinfow),
[SHBrowseForFolderW lifecycle](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shbrowseforfolderw).
