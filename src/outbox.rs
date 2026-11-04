//! One tracking command in flight at a time; later ones wait instead of being dropped.
use crate::tracking::Command;
use std::{collections::VecDeque, sync::mpsc::SyncSender};

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
        if !self.in_flight {
            return self.dispatch(command);
        }
        if self.waiting.len() == MAX_WAITING {
            return Err("Too many tracking commands are waiting. Try again shortly.".into());
        }
        self.waiting.push_back(command);
        Ok(())
    }

    /// The worker finished the command in flight, successfully or not.
    pub fn finished(&mut self) -> Result<(), String> {
        self.in_flight = false;
        match self.waiting.pop_front() {
            Some(command) => self.dispatch(command),
            None => Ok(()),
        }
    }

    fn dispatch(&mut self, command: Command) -> Result<(), String> {
        self.sender
            .try_send(command)
            .map_err(|error| format!("Cannot send tracking command: {error}"))?;
        self.in_flight = true;
        Ok(())
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
    fn waiting_commands_are_bounded_and_a_full_channel_is_reported() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let mut outbox = Outbox::new(sender);
        outbox.send(Command::Stop).unwrap();
        for _ in 0..MAX_WAITING {
            outbox.send(Command::Toggle).unwrap();
        }
        assert!(outbox.send(Command::Toggle).is_err());
        // The channel still holds the first command, so the next one cannot be sent.
        assert!(outbox.finished().is_err());
        assert!(!outbox.busy());
        assert!(matches!(receiver.try_recv(), Ok(Command::Stop)));
    }
}
