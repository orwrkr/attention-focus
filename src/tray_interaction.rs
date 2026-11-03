//! Distinguish a single click from the releases surrounding a Windows double click.

use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Clicks {
    deadline: Option<Instant>,
    ignore_release: bool,
}

impl Clicks {
    /// Windows reports a double click's second press as the double click itself, so a
    /// plain press means its final release was lost outside the icon and is not coming.
    pub fn press(&mut self) {
        self.ignore_release = false;
    }

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

const RAISE_ATTEMPTS: u8 = 8;
const RAISE_RETRY: Duration = Duration::from_millis(80);

/// Bounded retries until Windows confirms that the shown window reached the foreground.
#[derive(Default)]
pub struct WindowRaise {
    attempts: u8,
    next_attempt: Option<Instant>,
}

impl WindowRaise {
    pub fn request(&mut self, now: Instant) {
        self.attempts = RAISE_ATTEMPTS;
        self.next_attempt = Some(now);
    }

    /// Run an attempt when one is due; returns the delay before the next one. Repaints
    /// for unrelated reasons can poll sooner without using up the retry budget.
    pub fn poll(&mut self, now: Instant, raise: impl FnOnce() -> bool) -> Option<Duration> {
        let due = self.next_attempt?;
        if now < due {
            return Some(due - now);
        }
        self.attempts = self.attempts.saturating_sub(1);
        if raise() || self.attempts == 0 {
            self.next_attempt = None;
            return None;
        }
        self.next_attempt = Some(now + RAISE_RETRY);
        Some(RAISE_RETRY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_raise_retries_only_until_confirmed_and_stays_bounded() {
        let mut raise = WindowRaise::default();
        let start = Instant::now();
        assert_eq!(
            raise.poll(start, || panic!("no attempt was requested")),
            None
        );
        raise.request(start);
        assert_eq!(raise.poll(start, || false), Some(RAISE_RETRY));
        let due = start + RAISE_RETRY;
        assert_eq!(raise.poll(due, || true), None);
        assert_eq!(
            raise.poll(due, || panic!("confirmed raise must stop")),
            None
        );
        raise.request(start);
        let mut attempts = 0;
        let mut now = start;
        while let Some(delay) = raise.poll(now, || {
            attempts += 1;
            false
        }) {
            now += delay;
        }
        assert_eq!(attempts, RAISE_ATTEMPTS);
        assert_eq!(now - start, RAISE_RETRY * u32::from(RAISE_ATTEMPTS - 1));
        assert_eq!(
            raise.poll(now, || panic!("exhausted raise must stop")),
            None
        );
        raise.request(now);
        assert_eq!(raise.poll(now, || true), None);
    }

    #[test]
    fn unrelated_repaints_do_not_use_up_window_raise_attempts() {
        let mut raise = WindowRaise::default();
        let start = Instant::now();
        raise.request(start);
        assert_eq!(raise.poll(start, || false), Some(RAISE_RETRY));
        // A frame arriving early, such as the border animation, only reports the wait.
        let early = start + Duration::from_millis(50);
        assert_eq!(
            raise.poll(early, || panic!("the next attempt is not due yet")),
            Some(Duration::from_millis(30))
        );
        let mut attempts = 1;
        let mut now = start + RAISE_RETRY;
        while raise
            .poll(now, || {
                attempts += 1;
                false
            })
            .is_some()
        {
            now += RAISE_RETRY;
        }
        assert_eq!(attempts, RAISE_ATTEMPTS);
    }

    #[test]
    fn final_release_is_ignored_however_long_the_second_press_is_held() {
        let mut clicks = Clicks::default();
        let now = Instant::now();
        let interval = Duration::from_millis(500);
        clicks.press();
        clicks.release(now, interval);
        clicks.double_click();
        let held = now + Duration::from_secs(5);
        clicks.release(held, interval);
        assert_eq!(clicks.poll(held + interval * 2), (false, None));
    }

    #[test]
    fn lost_final_release_does_not_swallow_a_later_single_click() {
        let mut clicks = Clicks::default();
        let now = Instant::now();
        let interval = Duration::from_millis(500);
        clicks.release(now, interval);
        clicks.double_click();
        // The pointer left the icon before the double click's final release.
        let later = now + Duration::from_secs(5);
        clicks.press();
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
        clicks.double_click();
        clicks.release(now + Duration::from_millis(100), interval);
        assert_eq!(clicks.poll(now + interval * 2), (false, None));
        clicks.release(now + interval * 3, interval);
        assert!(clicks.poll(now + interval * 4).0);
    }
}
