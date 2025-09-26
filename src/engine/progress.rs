//! Progress tracking and callback system for UI integration

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

/// Progress callback trait for UI integration
pub trait ProgressCallback: Send + Sync {
    /// Called when operation starts
    fn on_start(&self, operation: &str, total_work: Option<u64>);
    
    /// Called during operation progress
    fn on_progress(&self, completed: u64, total: Option<u64>, message: Option<String>);
    
    /// Called when operation completes successfully
    fn on_complete(&self, message: Option<String>);
    
    /// Called when operation fails
    fn on_error(&self, error: &str);
    
    /// Called when operation is cancelled
    fn on_cancel(&self);
    
    /// Check if operation should be cancelled
    fn should_cancel(&self) -> bool;
}

/// Detailed progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressInfo {
    /// Current operation phase
    pub phase: ProgressPhase,
    /// Progress percentage (0.0 - 100.0)
    pub percent: f64,
    /// Completed work units
    pub completed: u64,
    /// Total work units (if known)
    pub total: Option<u64>,
    /// Current operation description
    pub message: String,
    /// Time elapsed since start
    pub elapsed: Duration,
    /// Estimated time remaining
    pub eta: Option<Duration>,
    /// Current throughput (units per second)
    pub throughput: Option<f64>,
    /// Additional metrics
    pub metrics: ProgressMetrics,
}

/// Progress phases
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProgressPhase {
    /// Initializing operation
    Initializing,
    /// Probing input file
    Probing,
    /// Planning cut strategy
    Planning,
    /// Processing video/audio
    Processing,
    /// Writing output file
    Writing,
    /// Verifying output
    Verifying,
    /// Operation completed
    Complete,
    /// Operation failed
    Failed,
    /// Operation cancelled
    Cancelled,
}

/// Additional progress metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMetrics {
    /// Bytes processed
    pub bytes_processed: u64,
    /// Bytes written to output
    pub bytes_written: u64,
    /// Frames processed (for video)
    pub frames_processed: Option<u64>,
    /// Current processing speed (fps for video, samples/sec for audio)
    pub processing_speed: Option<f64>,
    /// Memory usage (bytes)
    pub memory_usage: Option<u64>,
    /// Temperature info (for hardware acceleration)
    pub temperature: Option<f32>,
    /// GPU utilization percentage
    pub gpu_utilization: Option<f32>,
}

/// Progress tracker with thread-safe updates
#[derive(Clone)]
pub struct ProgressTracker {
    inner: Arc<Mutex<ProgressTrackerInner>>,
    callbacks: Arc<Mutex<Vec<Arc<dyn ProgressCallback>>>>,
}

struct ProgressTrackerInner {
    info: ProgressInfo,
    start_time: Instant,
    last_update: Instant,
    cancelled: bool,
    update_interval: Duration,
}

impl Default for ProgressMetrics {
    fn default() -> Self {
        Self {
            bytes_processed: 0,
            bytes_written: 0,
            frames_processed: None,
            processing_speed: None,
            memory_usage: None,
            temperature: None,
            gpu_utilization: None,
        }
    }
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new(operation: &str) -> Self {
        let info = ProgressInfo {
            phase: ProgressPhase::Initializing,
            percent: 0.0,
            completed: 0,
            total: None,
            message: operation.to_string(),
            elapsed: Duration::from_secs(0),
            eta: None,
            throughput: None,
            metrics: ProgressMetrics::default(),
        };

        let inner = ProgressTrackerInner {
            info,
            start_time: Instant::now(),
            last_update: Instant::now(),
            cancelled: false,
            update_interval: Duration::from_millis(100), // Update at most 10 times per second
        };

        Self {
            inner: Arc::new(Mutex::new(inner)),
            callbacks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a progress callback
    pub fn add_callback(&self, callback: Arc<dyn ProgressCallback>) {
        if let Ok(mut callbacks) = self.callbacks.lock() {
            callbacks.push(callback);
        }
    }

    /// Start operation with optional total work estimation
    pub fn start(&self, operation: &str, total_work: Option<u64>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.info.phase = ProgressPhase::Initializing;
            inner.info.message = operation.to_string();
            inner.info.total = total_work;
            inner.start_time = Instant::now();
            inner.last_update = Instant::now();
        }

        self.notify_callbacks(|cb| cb.on_start(operation, total_work));
    }

    /// Update progress
    pub fn update(&self, completed: u64, message: Option<String>) {
        let should_update = {
            let inner = self.inner.lock().ok();
            match inner {
                Some(ref inner) => {
                    inner.last_update.elapsed() >= inner.update_interval
                }
                None => false
            }
        };

        if !should_update {
            return;
        }

        if let Ok(mut inner) = self.inner.lock() {
            let now = Instant::now();
            inner.info.completed = completed;
            inner.info.elapsed = now.duration_since(inner.start_time);
            inner.last_update = now;

            if let Some(ref msg) = message {
                inner.info.message = msg.clone();
            }

            // Calculate percentage
            if let Some(total) = inner.info.total {
                inner.info.percent = (completed as f64 / total as f64 * 100.0).min(100.0);

                // Calculate ETA
                if completed > 0 && inner.info.percent < 100.0 {
                    let rate = completed as f64 / inner.info.elapsed.as_secs_f64();
                    if rate > 0.0 {
                        let remaining_work = total - completed;
                        let eta_seconds = remaining_work as f64 / rate;
                        inner.info.eta = Some(Duration::from_secs_f64(eta_seconds));
                    }
                }

                // Calculate throughput
                if inner.info.elapsed.as_secs_f64() > 0.0 {
                    inner.info.throughput = Some(completed as f64 / inner.info.elapsed.as_secs_f64());
                }
            }
        }

        self.notify_callbacks(|cb| cb.on_progress(completed, self.get_total(), message.clone()));
    }

    /// Update progress with detailed metrics
    pub fn update_with_metrics(&self, completed: u64, metrics: ProgressMetrics, message: Option<String>) {
        self.update(completed, message);
        
        if let Ok(mut inner) = self.inner.lock() {
            inner.info.metrics = metrics;
        }
    }

    /// Set current phase
    pub fn set_phase(&self, phase: ProgressPhase, message: Option<String>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.info.phase = phase;
            if let Some(msg) = message {
                inner.info.message = msg;
            }
        }
    }

    /// Complete operation successfully
    pub fn complete(&self, message: Option<String>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.info.phase = ProgressPhase::Complete;
            inner.info.percent = 100.0;
            if let Some(ref msg) = message {
                inner.info.message = msg.clone();
            }
        }

        self.notify_callbacks(|cb| cb.on_complete(message.clone()));
    }

    /// Mark operation as failed
    pub fn error(&self, error: &str) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.info.phase = ProgressPhase::Failed;
            inner.info.message = error.to_string();
        }

        self.notify_callbacks(|cb| cb.on_error(error));
    }

    /// Cancel operation
    pub fn cancel(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.cancelled = true;
            inner.info.phase = ProgressPhase::Cancelled;
            inner.info.message = "Operation cancelled".to_string();
        }

        self.notify_callbacks(|cb| cb.on_cancel());
    }

    /// Check if operation should be cancelled
    pub fn is_cancelled(&self) -> bool {
        if let Ok(inner) = self.inner.lock() {
            if inner.cancelled {
                return true;
            }
        }

        // Check callbacks for cancellation
        if let Ok(callbacks) = self.callbacks.lock() {
            for callback in callbacks.iter() {
                if callback.should_cancel() {
                    return true;
                }
            }
        }

        false
    }

    /// Get current progress information
    pub fn get_info(&self) -> Option<ProgressInfo> {
        self.inner.lock().ok().map(|inner| inner.info.clone())
    }

    /// Get total work units
    pub fn get_total(&self) -> Option<u64> {
        self.inner.lock().ok().and_then(|inner| inner.info.total)
    }

    /// Set update interval
    pub fn set_update_interval(&self, interval: Duration) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.update_interval = interval;
        }
    }

    /// Notify all callbacks
    fn notify_callbacks<F>(&self, f: F) 
    where 
        F: Fn(&dyn ProgressCallback)
    {
        if let Ok(callbacks) = self.callbacks.lock() {
            for callback in callbacks.iter() {
                f(callback.as_ref());
            }
        }
    }
}

/// Console progress callback for CLI usage
pub struct ConsoleProgressCallback {
    verbose: bool,
}

impl ConsoleProgressCallback {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
}

impl ProgressCallback for ConsoleProgressCallback {
    fn on_start(&self, operation: &str, total_work: Option<u64>) {
        if self.verbose {
            println!("🚀 Starting: {}", operation);
            if let Some(total) = total_work {
                println!("   Total work: {} units", total);
            }
        }
    }

    fn on_progress(&self, completed: u64, total: Option<u64>, message: Option<String>) {
        if let Some(total) = total {
            let percent = (completed as f64 / total as f64 * 100.0).min(100.0);
            let bar_length = 20;
            let filled = (percent / 100.0 * bar_length as f64) as usize;
            let bar = "█".repeat(filled) + &"░".repeat(bar_length - filled);
            
            if let Some(msg) = message {
                println!("\r🔄 [{}] {:>5.1}% {}", bar, percent, msg);
            } else {
                println!("\r🔄 [{}] {:>5.1}%", bar, percent);
            }
        } else if self.verbose {
            if let Some(msg) = message {
                println!("🔄 Progress: {} units - {}", completed, msg);
            } else {
                println!("🔄 Progress: {} units", completed);
            }
        }
    }

    fn on_complete(&self, message: Option<String>) {
        if let Some(msg) = message {
            println!("✅ Completed: {}", msg);
        } else {
            println!("✅ Operation completed successfully");
        }
    }

    fn on_error(&self, error: &str) {
        println!("❌ Error: {}", error);
    }

    fn on_cancel(&self) {
        println!("⚠️  Operation cancelled");
    }

    fn should_cancel(&self) -> bool {
        // Could check for Ctrl+C or other cancellation signals
        false
    }
}

/// JSON progress callback for structured output
pub struct JsonProgressCallback {
    output_progress_events: bool,
}

impl JsonProgressCallback {
    pub fn new(output_progress_events: bool) -> Self {
        Self { output_progress_events }
    }
}

impl ProgressCallback for JsonProgressCallback {
    fn on_start(&self, operation: &str, total_work: Option<u64>) {
        if self.output_progress_events {
            let event = serde_json::json!({
                "event": "start",
                "operation": operation,
                "total_work": total_work,
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            println!("{}", event);
        }
    }

    fn on_progress(&self, completed: u64, total: Option<u64>, message: Option<String>) {
        if self.output_progress_events {
            let percent = if let Some(total) = total {
                Some((completed as f64 / total as f64 * 100.0).min(100.0))
            } else {
                None
            };

            let event = serde_json::json!({
                "event": "progress",
                "completed": completed,
                "total": total,
                "percent": percent,
                "message": message,
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            println!("{}", event);
        }
    }

    fn on_complete(&self, message: Option<String>) {
        let event = serde_json::json!({
            "event": "complete",
            "message": message,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        println!("{}", event);
    }

    fn on_error(&self, error: &str) {
        let event = serde_json::json!({
            "event": "error",
            "error": error,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        println!("{}", event);
    }

    fn on_cancel(&self) {
        let event = serde_json::json!({
            "event": "cancel",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        println!("{}", event);
    }

    fn should_cancel(&self) -> bool {
        false
    }
}

/// No-op progress callback for when progress tracking is disabled
pub struct NoOpProgressCallback;

impl ProgressCallback for NoOpProgressCallback {
    fn on_start(&self, _operation: &str, _total_work: Option<u64>) {}
    fn on_progress(&self, _completed: u64, _total: Option<u64>, _message: Option<String>) {}
    fn on_complete(&self, _message: Option<String>) {}
    fn on_error(&self, _error: &str) {}
    fn on_cancel(&self) {}
    fn should_cancel(&self) -> bool { false }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    struct TestCallback {
        started: AtomicBool,
        progress_calls: AtomicU64,
        completed: AtomicBool,
        error_called: AtomicBool,
        cancelled: AtomicBool,
        should_cancel_flag: AtomicBool,
    }

    impl TestCallback {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                started: AtomicBool::new(false),
                progress_calls: AtomicU64::new(0),
                completed: AtomicBool::new(false),
                error_called: AtomicBool::new(false),
                cancelled: AtomicBool::new(false),
                should_cancel_flag: AtomicBool::new(false),
            })
        }

        fn set_should_cancel(&self, cancel: bool) {
            self.should_cancel_flag.store(cancel, Ordering::Relaxed);
        }
    }

    impl ProgressCallback for TestCallback {
        fn on_start(&self, _operation: &str, _total_work: Option<u64>) {
            self.started.store(true, Ordering::Relaxed);
        }

        fn on_progress(&self, _completed: u64, _total: Option<u64>, _message: Option<String>) {
            self.progress_calls.fetch_add(1, Ordering::Relaxed);
        }

        fn on_complete(&self, _message: Option<String>) {
            self.completed.store(true, Ordering::Relaxed);
        }

        fn on_error(&self, _error: &str) {
            self.error_called.store(true, Ordering::Relaxed);
        }

        fn on_cancel(&self) {
            self.cancelled.store(true, Ordering::Relaxed);
        }

        fn should_cancel(&self) -> bool {
            self.should_cancel_flag.load(Ordering::Relaxed)
        }
    }

    #[test]
    fn test_progress_tracker_basic_workflow() {
        let tracker = ProgressTracker::new("test operation");
        let callback = TestCallback::new();
        tracker.add_callback(callback.clone());

        // Start operation
        tracker.start("test operation", Some(100));
        assert!(callback.started.load(Ordering::Relaxed));

        // Update progress
        tracker.update(50, Some("halfway done".to_string()));
        assert!(callback.progress_calls.load(Ordering::Relaxed) > 0);

        // Complete operation
        tracker.complete(Some("success".to_string()));
        assert!(callback.completed.load(Ordering::Relaxed));

        // Check final state
        let info = tracker.get_info().unwrap();
        assert_eq!(info.phase, ProgressPhase::Complete);
        assert_eq!(info.percent, 100.0);
    }

    #[test]
    fn test_progress_tracker_cancellation() {
        let tracker = ProgressTracker::new("test operation");
        let callback = TestCallback::new();
        tracker.add_callback(callback.clone());

        tracker.start("test operation", Some(100));
        
        // Set cancellation flag
        callback.set_should_cancel(true);
        assert!(tracker.is_cancelled());

        // Cancel operation
        tracker.cancel();
        assert!(callback.cancelled.load(Ordering::Relaxed));
    }

    #[test]
    fn test_progress_tracker_error_handling() {
        let tracker = ProgressTracker::new("test operation");
        let callback = TestCallback::new();
        tracker.add_callback(callback.clone());

        tracker.start("test operation", Some(100));
        tracker.error("something went wrong");

        assert!(callback.error_called.load(Ordering::Relaxed));
        
        let info = tracker.get_info().unwrap();
        assert_eq!(info.phase, ProgressPhase::Failed);
    }
}
