use std::time::{Duration, Instant, SystemTime};

/// The widest accepted steady clock window around a system clock read
const MAX_SAMPLING_WINDOW: Duration = Duration::from_millis(1);

/// How many times the system clock is read at most when the clock is started
const MAX_SAMPLING_ATTEMPTS: usize = 3;

/// A clock which reads the system clock only at start and follows the time with the steady clock
///
/// The system clock is read only when the clock is started. The current time is derived by adding
/// the time elapsed on the steady clock since then, so changes to the system clock do not affect it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Clock {
    start_system_time: SystemTime,
    start_steady_time: Instant,
}

impl Clock {
    /// Starts the clock at the current system time
    ///
    /// The system clock read is bracketed by two steady clock reads, and the system time is paired
    /// with their midpoint. If the thread is paused around the system clock read, the window between
    /// the steady clock reads gets wide and the pairing gets inaccurate, so the reading is retried,
    /// up to [`MAX_SAMPLING_ATTEMPTS`] times. The reading with the narrowest window is used.
    pub fn start() -> Clock {
        Self::start_with(SystemTime::now, Instant::now)
    }

    fn start_with(
        mut read_system_clock: impl FnMut() -> SystemTime,
        mut read_steady_clock: impl FnMut() -> Instant,
    ) -> Clock {
        let mut sample = || {
            let steady_before = read_steady_clock();
            let system_time = read_system_clock();
            let window = read_steady_clock().duration_since(steady_before);

            let clock = Clock {
                start_system_time: system_time,
                start_steady_time: steady_before + window / 2,
            };
            (window, clock)
        };

        let (mut best_window, mut best_clock) = sample();
        for _ in 1..MAX_SAMPLING_ATTEMPTS {
            if best_window <= MAX_SAMPLING_WINDOW {
                break;
            }

            let (window, clock) = sample();
            if window < best_window {
                best_window = window;
                best_clock = clock;
            }
        }

        best_clock
    }

    /// Creates a clock which showed `system_time` when the steady clock was at `steady_time`
    #[cfg(test)]
    pub fn new(system_time: SystemTime, steady_time: Instant) -> Clock {
        Clock {
            start_system_time: system_time,
            start_steady_time: steady_time,
        }
    }

    /// Returns with the current time of the clock
    ///
    /// Reads only the steady clock, not the system clock.
    pub fn now(&self) -> SystemTime {
        self.start_system_time + self.start_steady_time.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_follows_steady_clock() {
        let system_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let steady_time = Instant::now().checked_sub(Duration::from_secs(10)).unwrap();
        let clock = Clock::new(system_time, steady_time);

        let now = clock.now();
        let elapsed = steady_time.elapsed();

        assert!(now >= system_time + Duration::from_secs(10));
        assert!(now <= system_time + elapsed);
    }

    #[test]
    fn now_never_goes_backwards() {
        let start = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let clock = Clock::new(start, Instant::now());

        let now_1 = clock.now();
        let now_2 = clock.now();

        assert!(now_1 >= start);
        assert!(now_2 >= now_1);
    }

    #[test]
    fn start_pairs_system_time_with_steady_midpoint() {
        let base = Instant::now();
        let system_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let mut steady_times = vec![base, base + Duration::from_micros(100)].into_iter();
        let mut system_reads = 0;

        let clock = Clock::start_with(
            || {
                system_reads += 1;
                system_time
            },
            || steady_times.next().unwrap(),
        );

        assert_eq!(system_reads, 1);
        assert_eq!(clock.start_system_time, system_time);
        assert_eq!(clock.start_steady_time, base + Duration::from_micros(50));
    }

    #[test]
    fn start_retries_when_sampling_window_is_too_wide() {
        let base = Instant::now();
        let system_time_1 = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let system_time_2 = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000);
        let mut steady_times = vec![
            base,
            base + Duration::from_millis(200),
            base + Duration::from_millis(300),
            base + Duration::from_millis(300) + Duration::from_micros(100),
        ]
        .into_iter();
        let mut system_times = vec![system_time_1, system_time_2].into_iter();

        let clock = Clock::start_with(
            || system_times.next().unwrap(),
            || steady_times.next().unwrap(),
        );

        assert_eq!(clock.start_system_time, system_time_2);
        assert_eq!(
            clock.start_steady_time,
            base + Duration::from_millis(300) + Duration::from_micros(50)
        );
    }

    #[test]
    fn start_uses_narrowest_window_when_all_are_too_wide() {
        let base = Instant::now();
        let system_time_1 = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let system_time_2 = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000);
        let system_time_3 = SystemTime::UNIX_EPOCH + Duration::from_secs(3_000_000);
        let mut steady_times = vec![
            base,
            base + Duration::from_millis(200),
            base + Duration::from_millis(300),
            base + Duration::from_millis(350),
            base + Duration::from_millis(400),
            base + Duration::from_millis(500),
        ]
        .into_iter();
        let mut system_times = vec![system_time_1, system_time_2, system_time_3].into_iter();
        let mut system_reads = 0;

        let clock = Clock::start_with(
            || {
                system_reads += 1;
                system_times.next().unwrap()
            },
            || steady_times.next().unwrap(),
        );

        assert_eq!(system_reads, 3);
        assert_eq!(clock.start_system_time, system_time_2);
        assert_eq!(clock.start_steady_time, base + Duration::from_millis(325));
    }
}
