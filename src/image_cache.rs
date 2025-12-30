use crate::image_loader;
use image::DynamicImage;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use rayon::prelude::*;

pub struct ImageCache {
    cache: Arc<Mutex<HashMap<PathBuf, CachedImage>>>,
    thumbnail_cache: Arc<Mutex<HashMap<PathBuf, CachedImage>>>,
    max_cache_size: usize,
    max_cache_items: usize,
}

#[derive(Clone)]
pub struct CachedImage {
    pub image: DynamicImage,
    pub last_access: std::time::Instant,
    pub size_bytes: usize,
}

impl ImageCache {
    pub fn new(max_cache_size_mb: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            thumbnail_cache: Arc::new(Mutex::new(HashMap::new())),
            max_cache_size: max_cache_size_mb * 1024 * 1024,
            max_cache_items: 100,
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
        let size_bytes = estimate_image_size(&image);
        let mut cache = self.cache.lock().unwrap();
        
        self.evict_if_needed(&mut cache);
        
        cache.insert(path, CachedImage {
            image,
            last_access: std::time::Instant::now(),
            size_bytes,
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
        
        cache.insert(path, CachedImage {
            image,
            last_access: std::time::Instant::now(),
            size_bytes,
        });
    }
    
    fn evict_if_needed(&self, cache: &mut HashMap<PathBuf, CachedImage>) {
        let total_size: usize = cache.values().map(|c| c.size_bytes).sum();
        
        if total_size > self.max_cache_size || cache.len() > self.max_cache_items {
            let mut entries: Vec<_> = cache.iter()
                .map(|(k, v)| (k.clone(), v.last_access, v.size_bytes))
                .collect();
            entries.sort_by_key(|(_, t, _)| *t);
            
            let mut current_size = total_size;
            let target_size = self.max_cache_size / 2;
            let target_count = self.max_cache_items / 2;
            
            let to_remove: Vec<PathBuf> = entries.iter()
                .take_while(|(_, _, size)| {
                    if current_size <= target_size && cache.len() <= target_count {
                        return false;
                    }
                    current_size = current_size.saturating_sub(*size);
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
                
                if let Ok(thumb) = image_loader::load_thumbnail(path, size) {
                    let size_bytes = estimate_image_size(&thumb);
                    let mut c = cache.lock().unwrap();
                    c.insert(path.clone(), CachedImage {
                        image: thumb,
                        last_access: std::time::Instant::now(),
                        size_bytes,
                    });
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
