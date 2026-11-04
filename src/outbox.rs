//! One tracking command in flight at a time; later ones wait instead of being dropped.
use crate::tracking::Command;
use std::{
    collections::VecDeque,
    sync::mpsc::{SyncSender, TrySendError},
};

const MAX_WAITING: usize = 8;

pub struct Outbox {
    sender: SyncSender<Command>,
    in_flight: bool,
    waiting: VecDeque<Command>,
}

impl Outbox {
    pub fn new(sender: SyncSender<Command>) -> Self {
        Self {
            sender,
            in_flight: false,
            waiting: VecDeque::new(),
        }
    }

    pub fn busy(&self) -> bool {
        self.in_flight
    }

    pub fn send(&mut self, command: Command) -> Result<(), String> {
        if self.waiting.len() == MAX_WAITING {
            return Err(crate::i18n::message("errors.too_many_commands", &[]));
        }
        self.waiting.push_back(command);
        self.pump()
    }

    /// The worker finished the command in flight, successfully or not.
    pub fn finished(&mut self) -> Result<(), String> {
        self.in_flight = false;
        self.pump()
    }

    /// Send the next waiting command. If the worker's queue is full, it stays first in line
    /// for a later call, so nothing is lost or overtaken.
    pub fn pump(&mut self) -> Result<(), String> {
        if self.in_flight {
            return Ok(());
        }
        let Some(command) = self.waiting.pop_front() else {
            return Ok(());
        };
        match self.sender.try_send(command) {
            Ok(()) => {
                self.in_flight = true;
                Ok(())
            }
            Err(TrySendError::Full(command)) => {
                self.waiting.push_front(command);
                Ok(())
            }
            Err(TrySendError::Disconnected(_)) => {
                Err(crate::i18n::message("errors.tracking_stopped", &[]))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn command_sent_while_another_is_in_flight_waits_and_keeps_its_order() {
        let (sender, receiver) = mpsc::sync_channel(4);
        let mut outbox = Outbox::new(sender);
        assert!(!outbox.busy());
        outbox.send(Command::Stop).unwrap();
        assert!(outbox.busy());
        outbox.send(Command::Toggle).unwrap();
        outbox.send(Command::Exit).unwrap();
        assert!(matches!(receiver.try_recv(), Ok(Command::Stop)));
        assert!(receiver.try_recv().is_err(), "only one is in flight");
        outbox.finished().unwrap();
        assert!(outbox.busy());
        assert!(matches!(receiver.try_recv(), Ok(Command::Toggle)));
        outbox.finished().unwrap();
        assert!(matches!(receiver.try_recv(), Ok(Command::Exit)));
        outbox.finished().unwrap();
        assert!(!outbox.busy());
    }

    #[test]
    fn a_full_worker_queue_delays_commands_without_losing_or_reordering_them() {
        let (sender, receiver) = mpsc::sync_channel(1);
        // Something else, such as a report request, already fills the worker's queue.
        sender.try_send(Command::AcceptRecovery).unwrap();
        let mut outbox = Outbox::new(sender);
        outbox.send(Command::Stop).unwrap();
        outbox.send(Command::Exit).unwrap();
        assert!(!outbox.busy(), "nothing could be sent yet");
        outbox.pump().unwrap();
        assert!(!outbox.busy(), "the queue is still full");
        assert!(matches!(receiver.try_recv(), Ok(Command::AcceptRecovery)));
        outbox.pump().unwrap();
        assert!(outbox.busy());
        assert!(matches!(receiver.try_recv(), Ok(Command::Stop)));
        outbox.finished().unwrap();
        assert!(matches!(receiver.try_recv(), Ok(Command::Exit)));
    }

    #[test]
    fn waiting_commands_are_bounded_and_a_stopped_worker_is_reported() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let mut outbox = Outbox::new(sender);
        outbox.send(Command::Stop).unwrap();
        for _ in 0..MAX_WAITING {
            outbox.send(Command::Toggle).unwrap();
        }
        assert!(outbox.send(Command::Toggle).is_err());
        drop(receiver);
        assert!(outbox.finished().is_err());
    }
}
