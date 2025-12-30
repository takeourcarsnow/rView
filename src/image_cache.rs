use crate::image_loader;
use image::DynamicImage;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct ImageCache {
    cache: Arc<Mutex<HashMap<PathBuf, CachedImage>>>,
    thumbnail_cache: Arc<Mutex<HashMap<PathBuf, CachedImage>>>,
    max_cache_size: usize,
    max_thumbnail_cache_size: usize,
    loading_queue: Arc<Mutex<Vec<PathBuf>>>,
}

#[derive(Clone)]
pub struct CachedImage {
    pub image: DynamicImage,
    pub last_access: std::time::Instant,
}

impl ImageCache {
    pub fn new(max_cache_size_mb: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            thumbnail_cache: Arc::new(Mutex::new(HashMap::new())),
            max_cache_size: max_cache_size_mb * 1024 * 1024,
            max_thumbnail_cache_size: 100 * 1024 * 1024, // 100MB for thumbnails
            loading_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub fn get(&self, path: &Path) -> Option<DynamicImage> {
        let mut cache = self.cache.lock().unwrap();
        if let Some(cached) = cache.get_mut(path) {
            cached.last_access = std::time::Instant::now();
            return Some(cached.image.clone());
        }
        None
    }
    
    pub fn get_thumbnail(&self, path: &Path) -> Option<DynamicImage> {
        let mut cache = self.thumbnail_cache.lock().unwrap();
        if let Some(cached) = cache.get_mut(path) {
            cached.last_access = std::time::Instant::now();
            return Some(cached.image.clone());
        }
        None
    }
    
    pub fn insert(&self, path: PathBuf, image: DynamicImage) {
        let mut cache = self.cache.lock().unwrap();
        
        // Evict old entries if needed
        self.evict_if_needed(&mut cache);
        
        cache.insert(path, CachedImage {
            image,
            last_access: std::time::Instant::now(),
        });
    }
    
    pub fn insert_thumbnail(&self, path: PathBuf, image: DynamicImage) {
        let mut cache = self.thumbnail_cache.lock().unwrap();
        
        // Simple size limit
        if cache.len() > 500 {
            // Remove oldest entries
            let mut entries: Vec<_> = cache.iter().collect();
            entries.sort_by_key(|(_, v)| v.last_access);
            let to_remove: Vec<_> = entries.iter().take(100).map(|(k, _)| (*k).clone()).collect();
            for key in to_remove {
                cache.remove(&key);
            }
        }
        
        cache.insert(path, CachedImage {
            image,
            last_access: std::time::Instant::now(),
        });
    }
    
    fn evict_if_needed(&self, cache: &mut HashMap<PathBuf, CachedImage>) {
        // Rough estimate of memory usage
        let estimated_size: usize = cache.values()
            .map(|c| estimate_image_size(&c.image))
            .sum();
        
        if estimated_size > self.max_cache_size {
            // Remove oldest accessed images
            let mut entries: Vec<_> = cache.iter()
                .map(|(k, v)| (k.clone(), v.last_access, estimate_image_size(&v.image)))
                .collect();
            entries.sort_by_key(|(_, t, _)| *t);
            
            let mut current_size = estimated_size;
            let to_remove: Vec<PathBuf> = entries.iter()
                .take_while(|(_, _, size)| {
                    if current_size <= self.max_cache_size / 2 {
                        return false;
                    }
                    current_size -= size;
                    true
                })
                .map(|(path, _, _)| path.clone())
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
                // Check if already cached
                {
                    let c = cache.lock().unwrap();
                    if c.contains_key(&path) {
                        continue;
                    }
                }
                
                // Load image
                if let Ok(image) = image_loader::load_image(&path) {
                    let mut c = cache.lock().unwrap();
                    c.insert(path, CachedImage {
                        image,
                        last_access: std::time::Instant::now(),
                    });
                }
            }
        });
    }
    
    pub fn preload_thumbnail(&self, path: PathBuf, size: u32) {
        let cache = Arc::clone(&self.thumbnail_cache);
        
        thread::spawn(move || {
            // Check if already cached
            {
                let c = cache.lock().unwrap();
                if c.contains_key(&path) {
                    return;
                }
            }
            
            // Load thumbnail
            if let Ok(thumb) = image_loader::load_thumbnail(&path, size) {
                let mut c = cache.lock().unwrap();
                c.insert(path, CachedImage {
                    image: thumb,
                    last_access: std::time::Instant::now(),
                });
            }
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
}

fn estimate_image_size(image: &DynamicImage) -> usize {
    let (w, h) = (image.width() as usize, image.height() as usize);
    w * h * 4 // Rough estimate assuming RGBA
}
