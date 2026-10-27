//! Distinguish a single click from the releases surrounding a Windows double click.

use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Clicks {
    deadline: Option<Instant>,
    ignore_release: bool,
}

impl Clicks {
    pub fn release(&mut self, now: Instant, double_click_time: Duration) {
        if self.ignore_release {
            self.ignore_release = false;
        } else {
            self.deadline = Some(now + double_click_time);
        }
    }

    pub fn double_click(&mut self) {
        self.deadline = None;
        self.ignore_release = true;
    }

    pub fn poll(&mut self, now: Instant) -> (bool, Option<Duration>) {
        match self.deadline {
            Some(deadline) if now >= deadline => {
                self.deadline = None;
                (true, None)
            }
            Some(deadline) => (false, Some(deadline - now)),
            None => (false, None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_click_shows_once_after_system_interval() {
        let mut clicks = Clicks::default();
        let now = Instant::now();
        let interval = Duration::from_millis(500);
        clicks.release(now, interval);
        assert!(!clicks.poll(now).0);
        assert!(clicks.poll(now + interval).0);
        assert!(!clicks.poll(now + interval).0);
    }

    #[test]
    fn double_click_cancels_both_surrounding_releases() {
        let mut clicks = Clicks::default();
        let now = Instant::now();
        let interval = Duration::from_millis(500);
        clicks.release(now, interval);
        clicks.double_click();
        clicks.release(now + Duration::from_millis(100), interval);
        assert_eq!(clicks.poll(now + interval * 2), (false, None));
        clicks.release(now + interval * 3, interval);
        assert!(clicks.poll(now + interval * 4).0);
    }
}
