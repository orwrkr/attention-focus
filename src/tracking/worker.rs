//! The single thread that owns the history database and serializes every command.
use super::{Command, Engine, Event, Moment, Store, now_ms};
use std::time::{Duration, Instant};

pub(super) fn command_moment(command: &Option<Command>, fallback: Moment, boot: Instant) -> Moment {
    match command {
        Some(Command::Interrupt {
            boundary: Some(boundary),
            ..
        }) => Moment {
            wall_ms: boundary.wall_ms,
            mono_ms: boundary
                .instant
                .saturating_duration_since(boot)
                .as_millis()
                .min(u128::from(u64::MAX)) as u64,
        },
        _ => fallback,
    }
}
pub fn spawn(
    path: std::path::PathBuf,
    notify: impl Fn(Event) + Send + 'static,
) -> std::sync::mpsc::SyncSender<Command> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(16);
    std::thread::spawn(move || {
        let boot = Instant::now();
        let now = || Moment {
            wall_ms: now_ms(),
            mono_ms: boot.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        };
        let store = match Store::open(&path) {
            Ok(store) => store,
            Err(error) => {
                notify(Event::Error(format!("Cannot open focus history: {error}")));
                while let Ok(command) = receiver.recv() {
                    if matches!(command, Command::Exit) {
                        notify(Event::ExitReady);
                        break;
                    }
                    notify(Event::Rejected(
                        "Tracking is unavailable. Resolve the storage error and restart.".into(),
                    ));
                }
                return;
            }
        };
        let mut engine = Engine::new(store, now());
        // Every command from the interface gets exactly one Committed or Rejected, so the
        // interface knows when it may send the next one. Other work reports separately.
        let publish = |engine: &Engine, committed: bool| match engine.snapshot(now()) {
            Ok(state) => notify(if committed {
                Event::Committed(state)
            } else {
                Event::State(state)
            }),
            Err(error) if committed => notify(Event::Rejected(error.to_string())),
            Err(error) => notify(Event::Error(error.to_string())),
        };
        publish(&engine, false);
        loop {
            let command = if engine.started.is_some() {
                match receiver.recv_timeout(Duration::from_secs(1)) {
                    Ok(command) => Some(command),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                    Err(_) => {
                        let _ = engine.store.interrupt("controller_lost");
                        break;
                    }
                }
            } else {
                match receiver.recv_timeout(Duration::from_secs(30)) {
                    Ok(command) => Some(command),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                    Err(_) => break,
                }
            };
            let completion = match &command {
                Some(Command::Interrupt { done, .. }) => done.clone(),
                _ => None,
            };
            // Preserve the event boundary even if the worker resumes after a short sleep.
            let moment = command_moment(&command, now(), boot);
            let report_request = match &command {
                Some(Command::Report { id, request }) => Some((*id, request.clone())),
                _ => None,
            };
            let from_interface = matches!(
                &command,
                Some(c) if !matches!(c, Command::Report { .. } | Command::Interrupt { .. })
            );
            let succeeded = match engine.apply(command, moment) {
                Ok(true) => {
                    notify(Event::ExitReady);
                    break;
                }
                Ok(false) => {
                    if let Some(done) = completion {
                        let _ = done.send(());
                    }
                    if let Some((id, request)) = report_request {
                        let result = engine
                            .snapshot(moment)
                            .and_then(|state| {
                                let at = engine.on_timeline(moment);
                                engine.store.report(&request, at, state.elapsed_ms)
                            })
                            .map(Box::new)
                            .map_err(|error| format!("Cannot load report: {error}"));
                        notify(Event::Report { id, result });
                    }
                    true
                }
                Err(error) => {
                    let message = format!("History was not updated: {error}");
                    if let Some((id, _)) = report_request {
                        notify(Event::Report {
                            id,
                            result: Err(message.clone()),
                        });
                    }
                    notify(if from_interface {
                        Event::Rejected(message)
                    } else {
                        Event::Error(message)
                    });
                    false
                }
            };
            publish(&engine, from_interface && succeeded);
        }
    });
    sender
}
