//! Progress tracking for acquisition operations

use std::sync::Arc;
use std::time::{Duration, Instant};

/// Progress information during acquisition
#[derive(Debug, Clone)]
pub struct AcquireProgress {
    /// Total bytes to process (if known)
    pub total_bytes: Option<u64>,
    /// Bytes processed so far
    pub bytes_processed: u64,
    /// Current transfer rate in bytes/second
    pub bytes_per_second: f64,
    /// Estimated time remaining
    pub estimated_remaining: Option<Duration>,
    /// Time elapsed since start
    pub elapsed: Duration,
    /// Percentage complete (0.0 - 100.0)
    pub percent_complete: Option<f64>,
    /// Current operation description
    pub operation: String,
}

impl AcquireProgress {
    /// Calculate progress from current state
    pub fn calculate(
        total_bytes: Option<u64>,
        bytes_processed: u64,
        start_time: Instant,
        operation: &str,
    ) -> Self {
        let elapsed = start_time.elapsed();
        let elapsed_secs = elapsed.as_secs_f64();

        let bytes_per_second = if elapsed_secs > 0.0 {
            bytes_processed as f64 / elapsed_secs
        } else {
            0.0
        };

        let (percent_complete, estimated_remaining) = if let Some(total) = total_bytes {
            let percent = if total > 0 {
                (bytes_processed as f64 / total as f64) * 100.0
            } else {
                100.0
            };

            let remaining_bytes = total.saturating_sub(bytes_processed);
            let remaining_secs = if bytes_per_second > 0.0 {
                remaining_bytes as f64 / bytes_per_second
            } else {
                f64::INFINITY
            };

            let remaining = if remaining_secs.is_finite() {
                Some(Duration::from_secs_f64(remaining_secs))
            } else {
                None
            };

            (Some(percent), remaining)
        } else {
            (None, None)
        };

        Self {
            total_bytes,
            bytes_processed,
            bytes_per_second,
            estimated_remaining,
            elapsed,
            percent_complete,
            operation: operation.to_string(),
        }
    }

    /// Format progress as human-readable string
    pub fn format(&self) -> String {
        let size_str = format_bytes(self.bytes_processed);
        let speed_str = format!("{}/s", format_bytes(self.bytes_per_second as u64));

        if let Some(percent) = self.percent_complete {
            let eta_str = if let Some(remaining) = self.estimated_remaining {
                format_duration(remaining)
            } else {
                "calculating...".to_string()
            };

            format!(
                "{}: {:.1}% complete - {} @ {} - ETA: {}",
                self.operation, percent, size_str, speed_str, eta_str
            )
        } else {
            format!(
                "{}: {} @ {}",
                self.operation, size_str, speed_str
            )
        }
    }
}

/// Callback type for progress updates
pub type ProgressCallback = Arc<dyn Fn(&AcquireProgress) + Send + Sync>;

/// Format bytes as human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format duration as human-readable string
fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();

    if total_secs >= 3600 {
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        format!("{}h {}m", hours, mins)
    } else if total_secs >= 60 {
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", total_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m");
    }

    #[test]
    fn test_progress_calculation() {
        let start = Instant::now();
        std::thread::sleep(Duration::from_millis(10));

        let progress = AcquireProgress::calculate(
            Some(1000),
            500,
            start,
            "Copying",
        );

        assert!(progress.percent_complete.is_some());
        assert!((progress.percent_complete.unwrap() - 50.0).abs() < 0.1);
        assert!(progress.bytes_per_second > 0.0);
    }
}
