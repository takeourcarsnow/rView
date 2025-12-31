use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Performance profiler for real-time monitoring
#[derive(Debug, Default)]
pub struct Profiler {
    timers: HashMap<String, Instant>,
    measurements: HashMap<String, Vec<Duration>>,
    counters: HashMap<String, u64>,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            timers: HashMap::new(),
            measurements: HashMap::new(),
            counters: HashMap::new(),
        }
    } 

    pub fn start_timer(&mut self, name: &str) {
        tracing::trace!(timer = name, "start_timer");
        self.timers.insert(name.to_string(), Instant::now());
    }

    pub fn end_timer(&mut self, name: &str) {
        if let Some(start) = self.timers.remove(name) {
            let duration = start.elapsed();
            tracing::debug!(timer = name, duration_ms = ?duration.as_millis(), "end_timer");
            self.measurements.entry(name.to_string())
                .or_default()
                .push(duration);
        }
    }

    pub fn increment_counter(&mut self, name: &str) {
        tracing::trace!(counter = name, "increment_counter");
        *self.counters.entry(name.to_string()).or_insert(0) += 1;
    }

    #[allow(dead_code)]
    pub fn add_measurement(&mut self, name: &str, duration: Duration) {
        self.measurements.entry(name.to_string())
            .or_default()
            .push(duration);
    }

    pub fn get_stats(&self) -> ProfilerStats {
        let mut stats = HashMap::new();

        for (name, measurements) in &self.measurements {
            if !measurements.is_empty() {
                let total: Duration = measurements.iter().sum();
                let avg = total / measurements.len() as u32;
                let min = measurements.iter().min().unwrap();
                let max = measurements.iter().max().unwrap();

                stats.insert(name.clone(), MeasurementStats {
                    count: measurements.len(),
                    total_time: total,
                    average_time: avg,
                    min_time: *min,
                    max_time: *max,
                });
            }
        }

        ProfilerStats {
            measurements: stats,
            counters: self.counters.clone(),
        }
    }

    pub fn reset(&mut self) {
        self.timers.clear();
        self.measurements.clear();
        self.counters.clear();
    }
}

#[derive(Debug, Clone)]
pub struct MeasurementStats {
    pub count: usize,
    #[allow(dead_code)]
    pub total_time: Duration,
    pub average_time: Duration,
    #[allow(dead_code)]
    pub min_time: Duration,
    #[allow(dead_code)]
    pub max_time: Duration,
} 

#[derive(Debug, Clone)]
pub struct ProfilerStats {
    pub measurements: HashMap<String, MeasurementStats>,
    pub counters: HashMap<String, u64>,
}

/// Cache statistics
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub total_images: usize,
    pub cached_images: usize,
    pub cache_memory_usage: usize, // bytes
    #[allow(dead_code)]
    pub thumbnail_count: usize,
    pub thumbnail_memory_usage: usize, // bytes
    pub cache_hit_count: u64,
    pub cache_miss_count: u64,
    pub eviction_count: u64,
} 

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.cache_hit_count + self.cache_miss_count;
        if total == 0 {
            0.0
        } else {
            self.cache_hit_count as f64 / total as f64
        }
    }

    pub fn memory_usage_mb(&self) -> f64 {
        (self.cache_memory_usage + self.thumbnail_memory_usage) as f64 / (1024.0 * 1024.0)
    }
}

/// Loading diagnostics
#[derive(Debug, Default, Clone)]
pub struct LoadingDiagnostics {
    pub total_load_time: Duration,
    #[allow(dead_code)]
    pub image_decode_time: Duration,
    #[allow(dead_code)]
    pub thumbnail_generation_time: Duration,
    #[allow(dead_code)]
    pub cache_lookup_time: Duration,
    #[allow(dead_code)]
    pub io_time: Duration,
    pub images_loaded: usize,
    pub thumbnails_generated: usize,
    pub errors_encountered: usize,
    pub bottlenecks: Vec<String>,
} 

impl LoadingDiagnostics {
    #[allow(dead_code)]
    pub fn add_bottleneck(&mut self, description: String) {
        self.bottlenecks.push(description);
    }

    pub fn average_load_time(&self) -> Duration {
        if self.images_loaded == 0 {
            Duration::default()
        } else {
            self.total_load_time / self.images_loaded as u32
        }
    }
} 

use std::cell::RefCell;
use std::thread_local;

thread_local! {
    static PROFILER: RefCell<Profiler> = RefCell::new(Profiler::new());
}

pub fn with_profiler<F, R>(f: F) -> R
where
    F: FnOnce(&mut Profiler) -> R,
{
    PROFILER.with(|p| f(&mut p.borrow_mut()))
}