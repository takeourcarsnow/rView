use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Represents an image in the catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogImage {
    pub id: i64,
    pub file_path: PathBuf,
    pub file_name: String,
    pub folder_path: PathBuf,
    pub file_size: i64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub date_taken: Option<DateTime<Utc>>,
    pub date_added: DateTime<Utc>,
    pub date_modified: DateTime<Utc>,
    pub rating: u8,
    pub color_label: String,
    pub flagged: bool,
    pub rejected: bool,
    pub keywords: Vec<String>,
    pub caption: String,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens: Option<String>,
    pub iso: Option<u32>,
    pub aperture: Option<f32>,
    pub shutter_speed: Option<String>,
    pub focal_length: Option<f32>,
}

/// Main catalog database
pub struct CatalogDb {
    pub(crate) conn: Connection,
}

#[allow(dead_code)]
impl CatalogDb {
    /// Create a new catalog or open existing one
    pub fn new() -> Result<Self> {
        let proj_dirs = directories::ProjectDirs::from("com", "imageviewer", "ImageViewer")
            .context("Failed to get project directory")?;

        let data_dir = proj_dirs.data_dir();
        std::fs::create_dir_all(data_dir)?;

        let db_path = data_dir.join("catalog.db");
        let conn = Connection::open(&db_path)
            .context(format!("Failed to open catalog database at {:?}", db_path))?;

        let mut catalog = Self { conn };
        catalog.init_database()?;

        Ok(catalog)
    }

    /// Initialize database schema
    fn init_database(&mut self) -> Result<()> {
        // Images table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS images (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL UNIQUE,
                file_name TEXT NOT NULL,
                folder_path TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                width INTEGER,
                height INTEGER,
                date_taken TEXT,
                date_added TEXT NOT NULL,
                date_modified TEXT NOT NULL,
                rating INTEGER DEFAULT 0,
                color_label TEXT DEFAULT '',
                flagged INTEGER DEFAULT 0,
                rejected INTEGER DEFAULT 0,
                caption TEXT DEFAULT '',
                camera_make TEXT,
                camera_model TEXT,
                lens TEXT,
                iso INTEGER,
                aperture REAL,
                shutter_speed TEXT,
                focal_length REAL
            )",
            [],
        )?;

        // Keywords table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS keywords (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                keyword TEXT NOT NULL UNIQUE
            )",
            [],
        )?;

        // Image-Keywords junction table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS image_keywords (
                image_id INTEGER NOT NULL,
                keyword_id INTEGER NOT NULL,
                PRIMARY KEY (image_id, keyword_id),
                FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE CASCADE,
                FOREIGN KEY (keyword_id) REFERENCES keywords(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Collections table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS collections (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                collection_type TEXT NOT NULL,
                parent_id INTEGER,
                description TEXT DEFAULT '',
                date_created TEXT NOT NULL,
                FOREIGN KEY (parent_id) REFERENCES collections(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Collection-Images junction table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS collection_images (
                collection_id INTEGER NOT NULL,
                image_id INTEGER NOT NULL,
                date_added TEXT NOT NULL,
                PRIMARY KEY (collection_id, image_id),
                FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
                FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Smart collection filters
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS smart_filters (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                collection_id INTEGER NOT NULL,
                filter_type TEXT NOT NULL,
                filter_value TEXT NOT NULL,
                FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Create indices for better performance
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_folder_path ON images(folder_path)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_rating ON images(rating)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_flagged ON images(flagged)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_date_taken ON images(date_taken)",
            [],
        )?;

        Ok(())
    }

    /// Import an image into the catalog
    pub fn import_image(&mut self, path: &Path) -> Result<i64> {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let folder_path = path.parent().unwrap_or_else(|| Path::new("")).to_path_buf();

        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len() as i64;

        let now = Utc::now();
        let date_modified = metadata.modified().ok().map(|t| t.into()).unwrap_or(now);

        // Try to get image dimensions
        let (width, height) = if let Ok(img) = image::open(path) {
            (Some(img.width()), Some(img.height()))
        } else {
            (None, None)
        };

        self.conn.execute(
            "INSERT INTO images (
                file_path, file_name, folder_path, file_size, 
                width, height, date_added, date_modified
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(file_path) DO UPDATE SET
                file_size = excluded.file_size,
                width = excluded.width,
                height = excluded.height,
                date_modified = excluded.date_modified",
            params![
                path.to_string_lossy().to_string(),
                file_name,
                folder_path.to_string_lossy().to_string(),
                file_size,
                width,
                height,
                now.to_rfc3339(),
                date_modified.to_rfc3339(),
            ],
        )?;

        let id = self.conn.last_insert_rowid();
        Ok(id)
    }

    /// Get an image by path
    pub fn get_image(&self, path: &Path) -> Result<Option<CatalogImage>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, file_path, file_name, folder_path, file_size, 
                    width, height, date_taken, date_added, date_modified,
                    rating, color_label, flagged, rejected, caption,
                    camera_make, camera_model, lens, iso, aperture, 
                    shutter_speed, focal_length
             FROM images WHERE file_path = ?1",
        )?;

        let result = stmt
            .query_row(params![path.to_string_lossy().to_string()], |row| {
                Ok(CatalogImage {
                    id: row.get(0)?,
                    file_path: PathBuf::from(row.get::<_, String>(1)?),
                    file_name: row.get(2)?,
                    folder_path: PathBuf::from(row.get::<_, String>(3)?),
                    file_size: row.get(4)?,
                    width: row.get(5)?,
                    height: row.get(6)?,
                    date_taken: row
                        .get::<_, Option<String>>(7)?
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    date_added: row
                        .get::<_, String>(8)?
                        .parse::<DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                    date_modified: row
                        .get::<_, String>(9)?
                        .parse::<DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                    rating: row.get(10)?,
                    color_label: row.get(11)?,
                    flagged: row.get::<_, i32>(12)? != 0,
                    rejected: row.get::<_, i32>(13)? != 0,
                    keywords: Vec::new(), // Loaded separately
                    caption: row.get(14)?,
                    camera_make: row.get(15)?,
                    camera_model: row.get(16)?,
                    lens: row.get(17)?,
                    iso: row.get(18)?,
                    aperture: row.get(19)?,
                    shutter_speed: row.get(20)?,
                    focal_length: row.get(21)?,
                })
            })
            .optional()?;

        // Load keywords if image found
        if let Some(mut img) = result {
            img.keywords = self.get_keywords_for_image(img.id)?;
            Ok(Some(img))
        } else {
            Ok(None)
        }
    }

    /// Get all images in the catalog
    pub fn get_all_images(&self) -> Result<Vec<CatalogImage>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, file_path, file_name, folder_path, file_size, 
                    width, height, date_taken, date_added, date_modified,
                    rating, color_label, flagged, rejected, caption,
                    camera_make, camera_model, lens, iso, aperture, 
                    shutter_speed, focal_length
             FROM images ORDER BY date_added DESC",
        )?;

        let images = stmt
            .query_map([], |row| {
                Ok(CatalogImage {
                    id: row.get(0)?,
                    file_path: PathBuf::from(row.get::<_, String>(1)?),
                    file_name: row.get(2)?,
                    folder_path: PathBuf::from(row.get::<_, String>(3)?),
                    file_size: row.get(4)?,
                    width: row.get(5)?,
                    height: row.get(6)?,
                    date_taken: row
                        .get::<_, Option<String>>(7)?
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    date_added: row
                        .get::<_, String>(8)?
                        .parse::<DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                    date_modified: row
                        .get::<_, String>(9)?
                        .parse::<DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                    rating: row.get(10)?,
                    color_label: row.get(11)?,
                    flagged: row.get::<_, i32>(12)? != 0,
                    rejected: row.get::<_, i32>(13)? != 0,
                    keywords: Vec::new(),
                    caption: row.get(14)?,
                    camera_make: row.get(15)?,
                    camera_model: row.get(16)?,
                    lens: row.get(17)?,
                    iso: row.get(18)?,
                    aperture: row.get(19)?,
                    shutter_speed: row.get(20)?,
                    focal_length: row.get(21)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(images)
    }

    /// Update image rating
    pub fn set_rating(&mut self, image_id: i64, rating: u8) -> Result<()> {
        self.conn.execute(
            "UPDATE images SET rating = ?1 WHERE id = ?2",
            params![rating, image_id],
        )?;
        Ok(())
    }

    /// Update image color label
    pub fn set_color_label(&mut self, image_id: i64, color: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE images SET color_label = ?1 WHERE id = ?2",
            params![color, image_id],
        )?;
        Ok(())
    }

    /// Toggle flag status
    pub fn toggle_flag(&mut self, image_id: i64) -> Result<bool> {
        let current: i32 = self.conn.query_row(
            "SELECT flagged FROM images WHERE id = ?1",
            params![image_id],
            |row| row.get(0),
        )?;

        let new_value = if current == 0 { 1 } else { 0 };
        self.conn.execute(
            "UPDATE images SET flagged = ?1 WHERE id = ?2",
            params![new_value, image_id],
        )?;

        Ok(new_value != 0)
    }

    /// Add a keyword to an image
    pub fn add_keyword(&mut self, image_id: i64, keyword: &str) -> Result<()> {
        // Insert keyword if it doesn't exist
        self.conn.execute(
            "INSERT OR IGNORE INTO keywords (keyword) VALUES (?1)",
            params![keyword],
        )?;

        // Get keyword ID
        let keyword_id: i64 = self.conn.query_row(
            "SELECT id FROM keywords WHERE keyword = ?1",
            params![keyword],
            |row| row.get(0),
        )?;

        // Link image to keyword
        self.conn.execute(
            "INSERT OR IGNORE INTO image_keywords (image_id, keyword_id) VALUES (?1, ?2)",
            params![image_id, keyword_id],
        )?;

        Ok(())
    }

    /// Remove a keyword from an image
    pub fn remove_keyword(&mut self, image_id: i64, keyword: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM image_keywords 
             WHERE image_id = ?1 
             AND keyword_id = (SELECT id FROM keywords WHERE keyword = ?2)",
            params![image_id, keyword],
        )?;
        Ok(())
    }

    /// Get all keywords for an image
    pub fn get_keywords_for_image(&self, image_id: i64) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT k.keyword 
             FROM keywords k
             JOIN image_keywords ik ON k.id = ik.keyword_id
             WHERE ik.image_id = ?1
             ORDER BY k.keyword",
        )?;

        let keywords = stmt
            .query_map(params![image_id], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(keywords)
    }

    /// Search images by keyword
    pub fn search_by_keyword(&self, keyword: &str) -> Result<Vec<CatalogImage>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT i.id, i.file_path, i.file_name, i.folder_path, i.file_size,
                    i.width, i.height, i.date_taken, i.date_added, i.date_modified,
                    i.rating, i.color_label, i.flagged, i.rejected, i.caption,
                    i.camera_make, i.camera_model, i.lens, i.iso, i.aperture,
                    i.shutter_speed, i.focal_length
             FROM images i
             JOIN image_keywords ik ON i.id = ik.image_id
             JOIN keywords k ON ik.keyword_id = k.id
             WHERE k.keyword LIKE ?1
             ORDER BY i.date_added DESC",
        )?;

        let search_pattern = format!("%{}%", keyword);
        let images = stmt
            .query_map(params![search_pattern], |row| {
                Ok(CatalogImage {
                    id: row.get(0)?,
                    file_path: PathBuf::from(row.get::<_, String>(1)?),
                    file_name: row.get(2)?,
                    folder_path: PathBuf::from(row.get::<_, String>(3)?),
                    file_size: row.get(4)?,
                    width: row.get(5)?,
                    height: row.get(6)?,
                    date_taken: row
                        .get::<_, Option<String>>(7)?
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    date_added: row
                        .get::<_, String>(8)?
                        .parse::<DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                    date_modified: row
                        .get::<_, String>(9)?
                        .parse::<DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                    rating: row.get(10)?,
                    color_label: row.get(11)?,
                    flagged: row.get::<_, i32>(12)? != 0,
                    rejected: row.get::<_, i32>(13)? != 0,
                    keywords: Vec::new(),
                    caption: row.get(14)?,
                    camera_make: row.get(15)?,
                    camera_model: row.get(16)?,
                    lens: row.get(17)?,
                    iso: row.get(18)?,
                    aperture: row.get(19)?,
                    shutter_speed: row.get(20)?,
                    focal_length: row.get(21)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(images)
    }

    /// Filter images by rating
    pub fn filter_by_rating(&self, min_rating: u8) -> Result<Vec<CatalogImage>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, file_path, file_name, folder_path, file_size,
                    width, height, date_taken, date_added, date_modified,
                    rating, color_label, flagged, rejected, caption,
                    camera_make, camera_model, lens, iso, aperture,
                    shutter_speed, focal_length
             FROM images
             WHERE rating >= ?1
             ORDER BY rating DESC, date_added DESC",
        )?;

        let images = stmt
            .query_map(params![min_rating], |row| {
                Ok(CatalogImage {
                    id: row.get(0)?,
                    file_path: PathBuf::from(row.get::<_, String>(1)?),
                    file_name: row.get(2)?,
                    folder_path: PathBuf::from(row.get::<_, String>(3)?),
                    file_size: row.get(4)?,
                    width: row.get(5)?,
                    height: row.get(6)?,
                    date_taken: row
                        .get::<_, Option<String>>(7)?
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    date_added: row
                        .get::<_, String>(8)?
                        .parse::<DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                    date_modified: row
                        .get::<_, String>(9)?
                        .parse::<DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                    rating: row.get(10)?,
                    color_label: row.get(11)?,
                    flagged: row.get::<_, i32>(12)? != 0,
                    rejected: row.get::<_, i32>(13)? != 0,
                    keywords: Vec::new(),
                    caption: row.get(14)?,
                    camera_make: row.get(15)?,
                    camera_model: row.get(16)?,
                    lens: row.get(17)?,
                    iso: row.get(18)?,
                    aperture: row.get(19)?,
                    shutter_speed: row.get(20)?,
                    focal_length: row.get(21)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(images)
    }

    /// Get image count
    pub fn get_image_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM images", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Remove an image from the catalog
    pub fn remove_image(&mut self, image_id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM images WHERE id = ?1", params![image_id])?;
        Ok(())
    }
}
