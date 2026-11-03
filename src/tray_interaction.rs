//! Distinguish a single click from the releases surrounding a Windows double click.

use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Clicks {
    deadline: Option<Instant>,
    ignore_release_until: Option<Instant>,
}

impl Clicks {
    pub fn release(&mut self, now: Instant, double_click_time: Duration) {
        // A release lost outside the icon must not swallow a later single click.
        if !self
            .ignore_release_until
            .take()
            .is_some_and(|until| now <= until)
        {
            self.deadline = Some(now + double_click_time);
        }
    }

    pub fn double_click(&mut self, now: Instant, double_click_time: Duration) {
        self.deadline = None;
        self.ignore_release_until = Some(now + double_click_time);
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

const RAISE_ATTEMPTS: u8 = 8;
const RAISE_RETRY: Duration = Duration::from_millis(80);

/// Bounded retries until Windows confirms that the shown window reached the foreground.
#[derive(Default)]
pub struct WindowRaise {
    attempts: u8,
}

impl WindowRaise {
    pub fn request(&mut self) {
        self.attempts = RAISE_ATTEMPTS;
    }

    /// Run one attempt when requested; returns the delay before the next attempt.
    pub fn poll(&mut self, raise: impl FnOnce() -> bool) -> Option<Duration> {
        if self.attempts == 0 {
            return None;
        }
        self.attempts -= 1;
        if raise() {
            self.attempts = 0;
        }
        (self.attempts > 0).then_some(RAISE_RETRY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_raise_retries_only_until_confirmed_and_stays_bounded() {
        let mut raise = WindowRaise::default();
        assert_eq!(raise.poll(|| panic!("no attempt was requested")), None);
        raise.request();
        assert_eq!(raise.poll(|| false), Some(RAISE_RETRY));
        assert_eq!(raise.poll(|| true), None);
        assert_eq!(raise.poll(|| panic!("confirmed raise must stop")), None);
        raise.request();
        let mut attempts = 0;
        while raise
            .poll(|| {
                attempts += 1;
                false
            })
            .is_some()
        {}
        assert_eq!(attempts, RAISE_ATTEMPTS);
        assert_eq!(raise.poll(|| panic!("exhausted raise must stop")), None);
        raise.request();
        assert_eq!(raise.poll(|| true), None);
    }

    #[test]
    fn lost_final_release_does_not_swallow_a_later_single_click() {
        let mut clicks = Clicks::default();
        let now = Instant::now();
        let interval = Duration::from_millis(500);
        clicks.release(now, interval);
        clicks.double_click(now + Duration::from_millis(100), interval);
        // The pointer left the icon before the double click's final release.
        let later = now + Duration::from_secs(5);
        clicks.release(later, interval);
        assert!(clicks.poll(later + interval).0);
    }

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
        clicks.double_click(now + Duration::from_millis(50), interval);
        clicks.release(now + Duration::from_millis(100), interval);
        assert_eq!(clicks.poll(now + interval * 2), (false, None));
        clicks.release(now + interval * 3, interval);
        assert!(clicks.poll(now + interval * 4).0);
    }
}
