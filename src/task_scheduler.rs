use std::collections::BinaryHeap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Priority levels for different types of image loading tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Critical = 3, // Current image, immediate UI response needed
    High = 2,     // Adjacent images, preload
    Medium = 1,   // Thumbnails for visible area
    Low = 0,      // Background tasks, distant images
}

/// A prioritized task in the queue
#[derive(Debug)]
pub struct PrioritizedTask<T> {
    pub priority: TaskPriority,
    pub task_id: u64,
    pub data: T,
}

impl<T> PartialEq for PrioritizedTask<T> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.task_id == other.task_id
    }
}

impl<T> Eq for PrioritizedTask<T> {}

impl<T> PartialOrd for PrioritizedTask<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for PrioritizedTask<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first, then lower task_id (earlier tasks) first
        other.priority.cmp(&self.priority)
            .then_with(|| self.task_id.cmp(&other.task_id))
    }
}

/// Task types for the image loading system
#[derive(Debug)]
pub enum ImageTask {
    LoadImage {
        path: std::path::PathBuf,
        priority: TaskPriority,
    },
    LoadThumbnail {
        path: std::path::PathBuf,
        size: u32,
        priority: TaskPriority,
    },
    LoadExif {
        path: std::path::PathBuf,
        priority: TaskPriority,
    },
    ComputeHistogram {
        image_data: Vec<u8>,
        width: u32,
        height: u32,
        priority: TaskPriority,
    },
    ApplyAdjustments {
        image_data: Vec<u8>,
        width: u32,
        height: u32,
        adjustments: crate::image_loader::ImageAdjustments,
        priority: TaskPriority,
    },
}

/// Result types for completed tasks
#[derive(Debug)]
pub enum TaskResult {
    ImageLoaded {
        path: std::path::PathBuf,
        image: image::DynamicImage,
    },
    ThumbnailLoaded {
        path: std::path::PathBuf,
        image: image::DynamicImage,
    },
    ExifLoaded {
        path: std::path::PathBuf,
        exif: Box<crate::exif_data::ExifInfo>,
    },
    HistogramComputed {
        histogram: Vec<Vec<u32>>,
    },
    AdjustmentsApplied {
        image: image::DynamicImage,
    },
    Error {
        task: ImageTask,
        error: String,
    },
}

/// Priority-based task scheduler for image operations
pub struct TaskScheduler {
    task_queue: Arc<Mutex<BinaryHeap<PrioritizedTask<ImageTask>>>>,
    result_tx: Sender<TaskResult>,
    result_rx: Receiver<TaskResult>,
    next_task_id: Arc<Mutex<u64>>,
    workers: Vec<thread::JoinHandle<()>>,
    running: Arc<Mutex<bool>>,
}

impl TaskScheduler {
    pub fn new(num_workers: usize) -> Self {
        let (result_tx, result_rx) = mpsc::channel();
        let task_queue = Arc::new(Mutex::new(BinaryHeap::new()));
        let next_task_id = Arc::new(Mutex::new(0));
        let running = Arc::new(Mutex::new(true));

        let mut workers = Vec::with_capacity(num_workers);

        for i in 0..num_workers {
            let task_queue = Arc::clone(&task_queue);
            let result_tx = result_tx.clone();
            let running = Arc::clone(&running);

            let worker = thread::Builder::new()
                .name(format!("image-worker-{}", i))
                .spawn(move || {
                    Self::worker_loop(task_queue, result_tx, running);
                })
                .expect("Failed to spawn worker thread");

            workers.push(worker);
        }

        Self {
            task_queue,
            result_tx,
            result_rx,
            next_task_id,
            workers,
            running,
        }
    }

    fn worker_loop(
        task_queue: Arc<Mutex<BinaryHeap<PrioritizedTask<ImageTask>>>>,
        result_tx: Sender<TaskResult>,
        running: Arc<Mutex<bool>>,
    ) {
        while *running.lock().unwrap() {
            let task = {
                let mut queue = task_queue.lock().unwrap();
                queue.pop()
            };

            if let Some(prioritized_task) = task {
                let result = Self::execute_task(prioritized_task.data);
                let _ = result_tx.send(result);
            } else {
                // No tasks available, sleep briefly to avoid busy waiting
                thread::sleep(Duration::from_millis(1));
            }
        }
    }

    fn execute_task(task: ImageTask) -> TaskResult {
        match &task {
            ImageTask::LoadImage { path, .. } => {
                match crate::image_loader::load_image(path) {
                    Ok(image) => TaskResult::ImageLoaded { path: path.clone(), image },
                    Err(e) => TaskResult::Error {
                        task,
                        error: format!("Failed to load image: {}", e),
                    },
                }
            }
            ImageTask::LoadThumbnail { path, size, .. } => {
                match crate::image_loader::load_thumbnail(path, *size) {
                    Ok(image) => TaskResult::ThumbnailLoaded { path: path.clone(), image },
                    Err(e) => TaskResult::Error {
                        task,
                        error: format!("Failed to load thumbnail: {}", e),
                    },
                }
            }
            ImageTask::LoadExif { path, .. } => {
                let exif = crate::exif_data::ExifInfo::from_file(path);
                TaskResult::ExifLoaded {
                    path: path.clone(),
                    exif: Box::new(exif),
                }
            }
            ImageTask::ComputeHistogram { image_data, width, height, .. } => {
                // Create image from raw data for histogram computation
                if let Some(img_buf) = image::ImageBuffer::from_raw(*width, *height, image_data.clone()) {
                    let image = image::DynamicImage::ImageRgba8(img_buf);
                    let histogram = crate::image_loader::calculate_histogram(&image);
                    TaskResult::HistogramComputed { histogram }
                } else {
                    TaskResult::Error {
                        task,
                        error: "Failed to create image from raw data".to_string(),
                    }
                }
            }
            ImageTask::ApplyAdjustments { image_data, width, height, adjustments, .. } => {
                if let Some(img_buf) = image::ImageBuffer::from_raw(*width, *height, image_data.clone()) {
                    let image = image::DynamicImage::ImageRgba8(img_buf);
                    let adjusted = crate::image_loader::apply_adjustments(&image, adjustments);
                    TaskResult::AdjustmentsApplied { image: adjusted }
                } else {
                    TaskResult::Error {
                        task,
                        error: "Failed to create image from raw data".to_string(),
                    }
                }
            }
        }
    }

    pub fn submit_task(&self, task: ImageTask) -> u64 {
        let mut next_id = self.next_task_id.lock().unwrap();
        let task_id = *next_id;
        *next_id += 1;

        let priority = match &task {
            ImageTask::LoadImage { priority, .. } => *priority,
            ImageTask::LoadThumbnail { priority, .. } => *priority,
            ImageTask::LoadExif { priority, .. } => *priority,
            ImageTask::ComputeHistogram { priority, .. } => *priority,
            ImageTask::ApplyAdjustments { priority, .. } => *priority,
        };

        let prioritized_task = PrioritizedTask {
            priority,
            task_id,
            data: task,
        };

        self.task_queue.lock().unwrap().push(prioritized_task);
        task_id
    }

    pub fn try_recv_result(&self) -> Option<TaskResult> {
        self.result_rx.try_recv().ok()
    }

    pub fn recv_result(&self) -> Result<TaskResult, mpsc::RecvError> {
        self.result_rx.recv()
    }

    pub fn cancel_task(&self, task_id: u64) {
        let mut queue = self.task_queue.lock().unwrap();
        queue.retain(|task| task.task_id != task_id);
    }

    pub fn clear_queue(&self) {
        self.task_queue.lock().unwrap().clear();
    }

    pub fn queue_size(&self) -> usize {
        self.task_queue.lock().unwrap().len()
    }

    pub fn shutdown(self) {
        *self.running.lock().unwrap() = false;
        for worker in self.workers {
            let _ = worker.join();
        }
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        let num_workers = num_cpus::get().max(2);
        Self::new(num_workers)
    }
}

/// Memory pool for efficient allocation of frequently used buffers
pub struct MemoryPool {
    pools: Vec<Arc<Mutex<Vec<Vec<u8>>>>>,
    size_classes: Vec<usize>,
}

impl MemoryPool {
    pub fn new() -> Self {
        // Size classes for common image buffer sizes (in bytes)
        let size_classes = vec![
            1024 * 1024,      // 1MB
            4 * 1024 * 1024,  // 4MB
            16 * 1024 * 1024, // 16MB
            64 * 1024 * 1024, // 64MB
        ];

        let pools = size_classes.iter().map(|_| Arc::new(Mutex::new(Vec::new()))).collect();

        Self {
            pools,
            size_classes,
        }
    }

    pub fn allocate(&self, size: usize) -> Vec<u8> {
        // Find the appropriate size class
        for (i, &class_size) in self.size_classes.iter().enumerate() {
            if size <= class_size {
                let mut pool = self.pools[i].lock().unwrap();
                if let Some(mut buffer) = pool.pop() {
                    // Resize if needed (shouldn't happen often)
                    if buffer.len() < size {
                        buffer.resize(size, 0);
                    } else {
                        // Clear the buffer for reuse
                        buffer[..size].fill(0);
                    }
                    return buffer;
                }
            }
        }

        // No pooled buffer available, allocate new one
        vec![0; size]
    }

    pub fn deallocate(&self, mut buffer: Vec<u8>) {
        // Clear the buffer before returning to pool
        buffer.fill(0);

        // Find appropriate pool and add it back
        for (i, &class_size) in self.size_classes.iter().enumerate() {
            if buffer.capacity() <= class_size {
                let mut pool = self.pools[i].lock().unwrap();
                // Limit pool size to prevent unbounded growth
                if pool.len() < 10 {
                    pool.push(buffer);
                }
                return;
            }
        }

        // Buffer too large for any pool, just drop it
    }

    pub fn stats(&self) -> MemoryPoolStats {
        let mut total_buffers = 0;
        let mut total_memory = 0;

        for (i, pool) in self.pools.iter().enumerate() {
            let pool_size = pool.lock().unwrap().len();
            total_buffers += pool_size;
            total_memory += pool_size * self.size_classes[i];
        }

        MemoryPoolStats {
            total_buffers,
            total_memory_mb: total_memory as f64 / (1024.0 * 1024.0),
        }
    }
}

#[derive(Debug)]
pub struct MemoryPoolStats {
    pub total_buffers: usize,
    pub total_memory_mb: f64,
}

impl Default for MemoryPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Concurrent histogram computation using parallel processing
pub mod concurrent_histogram {
    use rayon::prelude::*;
    use std::sync::Mutex;

    /// Compute histogram in parallel across image tiles
    pub fn compute_parallel(image: &image::DynamicImage, num_tiles: usize) -> Vec<Vec<u32>> {
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();
        let pixels = rgba.into_raw();

        // Create histogram accumulator
        let histogram = Mutex::new(vec![vec![0u32; 256]; 4]); // RGBA channels

        // Calculate tile size
        let tile_height = height / num_tiles as u32;
        let tiles: Vec<_> = (0..num_tiles)
            .map(|i| {
                let start_y = i as u32 * tile_height;
                let end_y = if i == num_tiles - 1 { height } else { (i as u32 + 1) * tile_height };
                (start_y, end_y)
            })
            .collect();

        // Process tiles in parallel
        tiles.par_iter().for_each(|(start_y, end_y)| {
            let mut local_hist = vec![vec![0u32; 256]; 4];

            for y in *start_y..*end_y {
                for x in 0..width {
                    let idx = ((y * width + x) * 4) as usize;
                    if idx + 3 < pixels.len() {
                        local_hist[0][pixels[idx] as usize] += 1;     // R
                        local_hist[1][pixels[idx + 1] as usize] += 1; // G
                        local_hist[2][pixels[idx + 2] as usize] += 1; // B
                        local_hist[3][pixels[idx + 3] as usize] += 1; // A
                    }
                }
            }

            // Merge local histogram into global
            let mut global_hist = histogram.lock().unwrap();
            for c in 0..4 {
                for i in 0..256 {
                    global_hist[c][i] += local_hist[c][i];
                }
            }
        });

        histogram.into_inner().unwrap()
    }

    /// Adaptive tile count based on image size
    pub fn optimal_tile_count(width: u32, height: u32) -> usize {
        let total_pixels = width * height;
        let min_tiles = 2;
        let max_tiles = num_cpus::get();

        // Use more tiles for larger images
        if total_pixels < 1_000_000 {
            min_tiles // Small images
        } else if total_pixels < 10_000_000 {
            (max_tiles / 2).max(min_tiles) // Medium images
        } else {
            max_tiles // Large images
        }
    }
}