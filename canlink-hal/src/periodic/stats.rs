//! Periodic message statistics.

use std::time::{Duration, Instant};

/// Statistics for periodic message sending.
///
/// Tracks send count, timing information, and interval accuracy.
///
/// # Example
///
/// ```rust,ignore
/// use canlink_hal::periodic::PeriodicStats;
///
/// let stats = PeriodicStats::new();
/// println!("Send count: {}", stats.send_count());
/// if let Some(avg) = stats.average_interval() {
///     println!("Average interval: {:?}", avg);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct PeriodicStats {
    /// Number of messages sent
    send_count: u64,
    /// Last send time
    last_send_time: Option<Instant>,
    /// Total interval duration (for average calculation)
    total_interval: Duration,
    /// Number of interval samples
    interval_samples: u64,
    /// Minimum observed interval
    min_interval: Option<Duration>,
    /// Maximum observed interval
    max_interval: Option<Duration>,
}

impl PeriodicStats {
    /// Create a new statistics instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a send event.
    ///
    /// Updates all statistics based on the current time.
    pub fn record_send(&mut self, now: Instant) {
        self.send_count += 1;

        if let Some(last) = self.last_send_time {
            let interval = now.duration_since(last);
            self.total_interval += interval;
            self.interval_samples += 1;

            // Update min/max
            match self.min_interval {
                Some(min) if interval < min => self.min_interval = Some(interval),
                None => self.min_interval = Some(interval),
                _ => {}
            }

            match self.max_interval {
                Some(max) if interval > max => self.max_interval = Some(interval),
                None => self.max_interval = Some(interval),
                _ => {}
            }
        }

        self.last_send_time = Some(now);
    }

    /// Get the total number of messages sent.
    #[must_use]
    pub fn send_count(&self) -> u64 {
        self.send_count
    }

    /// Get the last send time.
    #[must_use]
    pub fn last_send_time(&self) -> Option<Instant> {
        self.last_send_time
    }

    /// Get the average actual interval between sends.
    ///
    /// Returns `None` if fewer than 2 messages have been sent.
    #[must_use]
    pub fn average_interval(&self) -> Option<Duration> {
        if self.interval_samples > 0 {
            let samples = u32::try_from(self.interval_samples).ok()?;
            Some(self.total_interval / samples)
        } else {
            None
        }
    }

    /// Get the minimum observed interval.
    #[must_use]
    pub fn min_interval(&self) -> Option<Duration> {
        self.min_interval
    }

    /// Get the maximum observed interval.
    #[must_use]
    pub fn max_interval(&self) -> Option<Duration> {
        self.max_interval
    }

    /// Calculate the jitter (max - min interval).
    #[must_use]
    pub fn jitter(&self) -> Option<Duration> {
        match (self.min_interval, self.max_interval) {
            (Some(min), Some(max)) => max.checked_sub(min),
            _ => None,
        }
    }

    /// Reset all statistics.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_stats() {
        let stats = PeriodicStats::new();
        assert_eq!(stats.send_count(), 0);
        assert!(stats.last_send_time().is_none());
        assert!(stats.average_interval().is_none());
    }

    #[test]
    fn test_record_single_send() {
        let mut stats = PeriodicStats::new();
        let now = Instant::now();

        stats.record_send(now);

        assert_eq!(stats.send_count(), 1);
        assert!(stats.last_send_time().is_some());
        assert!(stats.average_interval().is_none()); // Need 2+ sends for average
    }

    #[test]
    fn test_record_multiple_sends() {
        let mut stats = PeriodicStats::new();
        let start = Instant::now();

        stats.record_send(start);
        stats.record_send(start + Duration::from_millis(100));
        stats.record_send(start + Duration::from_millis(200));

        assert_eq!(stats.send_count(), 3);
        assert_eq!(stats.interval_samples, 2);

        let avg = stats.average_interval().unwrap();
        assert_eq!(avg, Duration::from_millis(100));
    }

    #[test]
    fn test_min_max_interval() {
        let mut stats = PeriodicStats::new();
        let start = Instant::now();

        stats.record_send(start);
        stats.record_send(start + Duration::from_millis(90));
        stats.record_send(start + Duration::from_millis(200)); // 110ms interval

        assert_eq!(stats.min_interval(), Some(Duration::from_millis(90)));
        assert_eq!(stats.max_interval(), Some(Duration::from_millis(110)));
    }

    #[test]
    fn test_jitter() {
        let mut stats = PeriodicStats::new();
        let start = Instant::now();

        stats.record_send(start);
        stats.record_send(start + Duration::from_millis(90));
        stats.record_send(start + Duration::from_millis(200));

        let jitter = stats.jitter().unwrap();
        assert_eq!(jitter, Duration::from_millis(20)); // 110 - 90
    }

    #[test]
    fn test_reset() {
        let mut stats = PeriodicStats::new();
        let now = Instant::now();

        stats.record_send(now);
        stats.record_send(now + Duration::from_millis(100));

        assert_eq!(stats.send_count(), 2);

        stats.reset();

        assert_eq!(stats.send_count(), 0);
        assert!(stats.last_send_time().is_none());
        assert!(stats.average_interval().is_none());
    }
}
