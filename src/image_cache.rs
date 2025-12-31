use crate::image_loader;
use image::DynamicImage;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::fs;
use rayon::prelude::*;

pub struct ImageCache {
    cache: Arc<Mutex<HashMap<PathBuf, CachedImage>>>,
    thumbnail_cache: Arc<Mutex<HashMap<PathBuf, CachedImage>>>,
    max_cache_size: usize,
    max_cache_items: usize,
    disk_cache_dir: Option<PathBuf>,
}

#[derive(Clone)]
pub struct CachedImage {
    pub image: DynamicImage,
    pub last_access: std::time::Instant,
    pub size_bytes: usize,
    pub priority: CachePriority, // New: priority for smarter eviction
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CachePriority {
    Low = 0,    // Distant images
    Medium = 1, // Adjacent images
} 

impl ImageCache {
    pub fn new(max_cache_size_mb: usize) -> Self {
        let disk_cache_dir = if let Some(proj_dirs) = directories::ProjectDirs::from("com", "imageviewer", "ImageViewer") {
            let cache_dir = proj_dirs.cache_dir().join("thumbnails");
            if fs::create_dir_all(&cache_dir).is_err() {
                None
            } else {
                Some(cache_dir)
            }
        } else {
            None
        };
 
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            thumbnail_cache: Arc::new(Mutex::new(HashMap::new())),
            max_cache_size: max_cache_size_mb * 1024 * 1024,
            max_cache_items: 100,
            disk_cache_dir,
        }
    }
    
    fn get_from_cache(&self, cache: &Arc<Mutex<HashMap<PathBuf, CachedImage>>>, path: &Path) -> Option<DynamicImage> {
        let mut cache = cache.lock().unwrap();
        if let Some(cached) = cache.get_mut(path) {
            cached.last_access = std::time::Instant::now();
            return Some(cached.image.clone());
        }
        None
    }

    pub fn get<P: AsRef<Path>>(&self, path: P) -> Option<DynamicImage> {
        self.get_from_cache(&self.cache, path.as_ref())
    }
    
    pub fn get_thumbnail<P: AsRef<Path>>(&self, path: P) -> Option<DynamicImage> {
        let path = path.as_ref();

        // Try in-memory thumbnail cache first
        if let Some(img) = self.get_from_cache(&self.thumbnail_cache, path) {
            return Some(img);
        }

        // Try to load from disk cache
        if let Some(image) = self.load_thumbnail_from_disk(path) {
            let size_bytes = estimate_image_size(&image);
            let mut cache = self.thumbnail_cache.lock().unwrap();
            cache.insert(path.to_path_buf(), CachedImage {
                image: image.clone(),
                last_access: std::time::Instant::now(),
                size_bytes,
                priority: CachePriority::Low,
            });
            return Some(image);
        }

        None
    }

    fn cache_key_path(&self, path: &Path) -> Option<PathBuf> {
        if let Some(cache_dir) = &self.disk_cache_dir {
            if let Some(key) = self.get_cache_key(path) {
                return Some(cache_dir.join(format!("{}.png", key)));
            }
        }
        None
    }

    fn save_thumbnail_to_disk(&self, path: &Path, image: &DynamicImage) {
        if let Some(cache_path) = self.cache_key_path(path) {
            let mut buffer = std::io::Cursor::new(Vec::new());
            if image.to_rgba8().write_to(&mut buffer, image::ImageFormat::Png).is_ok() {
                let _ = fs::write(cache_path, buffer.into_inner());
            }
        }
    }

    fn load_thumbnail_from_disk(&self, path: &Path) -> Option<DynamicImage> {
        if let Some(cache_path) = self.cache_key_path(path) {
            if cache_path.exists() {
                if let Ok(data) = fs::read(&cache_path) {
                    if let Ok(img) = image::load_from_memory(&data) {
                        return Some(img);
                    }
                }
            }
        }
        None
    }

    pub fn put<P: Into<PathBuf>>(&self, path: P, image: DynamicImage) {
        self.insert(path.into(), image);
    }

    pub fn stats(&self) -> CacheStats {
        self.get_stats()
    }    
    pub fn insert(&self, path: PathBuf, image: DynamicImage) {
        let size_bytes = estimate_image_size(&image);
        let mut cache = self.cache.lock().unwrap();

        self.evict_if_needed(&mut cache);

        cache.insert(path, CachedImage {
            image,
            last_access: std::time::Instant::now(),
            size_bytes,
            priority: CachePriority::Medium, // Default priority for inserted images
        });
    }



    pub fn insert_thumbnail(&self, path: PathBuf, image: DynamicImage) {
        let size_bytes = estimate_image_size(&image);
        let mut cache = self.thumbnail_cache.lock().unwrap();

        // Simple size limit for thumbnails
        if cache.len() > 500 {
            let mut entries: Vec<_> = cache.iter()
                .map(|(k, v)| (k.clone(), v.last_access))
                .collect();
            entries.sort_by_key(|(_, t)| *t);

            for (key, _) in entries.iter().take(100) {
                cache.remove(key);
            }
        }

        cache.insert(path.clone(), CachedImage {
            image: image.clone(),
            last_access: std::time::Instant::now(),
            size_bytes,
            priority: CachePriority::Low,
        });

        // Also save to disk cache
        self.save_thumbnail_to_disk(&path, &image);
    }
    
    fn evict_if_needed(&self, cache: &mut HashMap<PathBuf, CachedImage>) {
        let total_size: usize = cache.values().map(|c| c.size_bytes).sum();

        if total_size > self.max_cache_size || cache.len() > self.max_cache_items {
            let mut entries: Vec<_> = cache.iter()
                .map(|(k, v)| (k.clone(), v.last_access, v.size_bytes, v.priority))
                .collect();
            // Sort by priority (higher priority first), then by access time (older first)
            entries.sort_by_key(|(_, time, _, priority)| (std::cmp::Reverse(*priority), *time));

            let mut current_size = total_size;
            let target_size = self.max_cache_size / 2;
            let target_count = self.max_cache_items / 2;

            let to_remove: Vec<PathBuf> = entries.iter()
                .rev() // Start from lowest priority/oldest
                .take_while(|(_, _, size, _)| {
                    if current_size <= target_size && cache.len() <= target_count {
                        return false;
                    }
                    current_size = current_size.saturating_sub(*size);
                    true
                })
                .map(|(path, _, _, _)| path.clone())
                .collect();

            for path in to_remove {
                cache.remove(&path);
            }
        }
    }
    
    pub fn preload(&self, paths: Vec<PathBuf>) {
        let cache = Arc::clone(&self.cache);
        
        thread::spawn(move || {
            for path in paths {
                {
                    let c = cache.lock().unwrap();
                    if c.contains_key(&path) {
                        continue;
                    }
                }
                
                if let Ok(image) = image_loader::load_image(&path) {
                    let size_bytes = estimate_image_size(&image);
                    let mut c = cache.lock().unwrap();
                    c.insert(path, CachedImage {
                        image,
                        last_access: std::time::Instant::now(),
                        size_bytes,
                        priority: CachePriority::Medium, // Default priority for preloaded images
                    });
                }
            }
        });
    }
    
    pub fn preload_thumbnails_parallel(&self, paths: Vec<PathBuf>, size: u32) {
        let cache = Arc::clone(&self.thumbnail_cache);
        
        thread::spawn(move || {
            paths.par_iter().for_each(|path| {
                {
                    let c = cache.lock().unwrap();
                    if c.contains_key(path) {
                        return;
                    }
                }
                
                // For RAW files, only attempt to extract embedded JPEG thumbnails to avoid heavy RAW decoding here
                if image_loader::is_raw_file(path) {
                    if let Ok(thumb) = image_loader::load_raw_embedded_thumbnail(path, size) {
                        let size_bytes = estimate_image_size(&thumb);
                        let mut c = cache.lock().unwrap();
                        c.insert(path.clone(), CachedImage {
                            image: thumb,
                            last_access: std::time::Instant::now(),
                            size_bytes,
                            priority: CachePriority::Low, // Thumbnails have lower priority
                        });
                    }
                } else {
                    if let Ok(thumb) = image_loader::load_thumbnail(path, size) {
                        let size_bytes = estimate_image_size(&thumb);
                        let mut c = cache.lock().unwrap();
                        c.insert(path.clone(), CachedImage {
                            image: thumb,
                            last_access: std::time::Instant::now(),
                            size_bytes,
                            priority: CachePriority::Low, // Thumbnails have lower priority
                        });
                    }
                }
            });
        });
    }
    
    pub fn clear(&self) {
        self.cache.lock().unwrap().clear();
        self.thumbnail_cache.lock().unwrap().clear();
    }
    
    pub fn remove(&self, path: &Path) {
        self.cache.lock().unwrap().remove(path);
        self.thumbnail_cache.lock().unwrap().remove(path);
    }
    
    pub fn get_stats(&self) -> CacheStats {
        let cache = self.cache.lock().unwrap();
        let thumb_cache = self.thumbnail_cache.lock().unwrap();

        CacheStats {
            image_count: cache.len(),
            image_size_bytes: cache.values().map(|c| c.size_bytes).sum(),
            thumbnail_count: thumb_cache.len(),
            thumbnail_size_bytes: thumb_cache.values().map(|c| c.size_bytes).sum(),
        }
    }

    fn get_cache_key(&self, path: &Path) -> Option<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        path.metadata().ok()?.modified().ok()?.hash(&mut hasher);
        Some(format!("{:x}", hasher.finish()))
    }


}

#[derive(Debug)]
pub struct CacheStats {
    pub image_count: usize,
    pub image_size_bytes: usize,
    pub thumbnail_count: usize,
    pub thumbnail_size_bytes: usize,
} 

fn estimate_image_size(image: &DynamicImage) -> usize {
    let (w, h) = (image.width() as usize, image.height() as usize);
    w * h * 4
}
