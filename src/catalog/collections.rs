use anyhow::Result;
use chrono::Utc;
use rusqlite::params;
use std::path::Path;

use super::database::CatalogDb;

/// Collection types similar to Lightroom
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CollectionType {
    #[default]
    Regular,      // User-created manual collection
    Smart,        // Auto-populated based on criteria
}

impl CollectionType {
    pub fn to_string(&self) -> &str {
        match self {
            CollectionType::Regular => "regular",
            CollectionType::Smart => "smart",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "smart" => CollectionType::Smart,
            _ => CollectionType::Regular,
        }
    }
}

/// Represents a collection of images
#[derive(Debug, Clone)]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub collection_type: CollectionType,
    pub parent_id: Option<i64>,
    pub description: String,
    pub image_count: usize,
}

impl CatalogDb {
    /// Create a new collection
    pub fn create_collection(
        &mut self,
        name: &str,
        collection_type: CollectionType,
        parent_id: Option<i64>,
        description: &str,
    ) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        
        self.conn.execute(
            "INSERT INTO collections (name, collection_type, parent_id, description, date_created)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![name, collection_type.to_string(), parent_id, description, now],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get all collections
    pub fn get_collections(&self) -> Result<Vec<Collection>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.name, c.collection_type, c.parent_id, c.description,
                    COUNT(ci.image_id) as image_count
             FROM collections c
             LEFT JOIN collection_images ci ON c.id = ci.collection_id
             GROUP BY c.id
             ORDER BY c.name"
        )?;

        let collections = stmt.query_map([], |row| {
            Ok(Collection {
                id: row.get(0)?,
                name: row.get(1)?,
                collection_type: CollectionType::from_string(&row.get::<_, String>(2)?),
                parent_id: row.get(3)?,
                description: row.get(4)?,
                image_count: row.get(5)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(collections)
    }

    /// Add an image to a collection
    pub fn add_to_collection(&mut self, collection_id: i64, image_id: i64) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        
        self.conn.execute(
            "INSERT OR IGNORE INTO collection_images (collection_id, image_id, date_added)
             VALUES (?1, ?2, ?3)",
            params![collection_id, image_id, now],
        )?;

        Ok(())
    }

    /// Remove an image from a collection
    pub fn remove_from_collection(&mut self, collection_id: i64, image_id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM collection_images WHERE collection_id = ?1 AND image_id = ?2",
            params![collection_id, image_id],
        )?;

        Ok(())
    }

    /// Get all images in a collection
    pub fn get_collection_images(&self, collection_id: i64) -> Result<Vec<super::database::CatalogImage>> {
        let mut stmt = self.conn.prepare(
            "SELECT i.id, i.file_path, i.file_name, i.folder_path, i.file_size,
                    i.width, i.height, i.date_taken, i.date_added, i.date_modified,
                    i.rating, i.color_label, i.flagged, i.rejected, i.caption,
                    i.camera_make, i.camera_model, i.lens, i.iso, i.aperture,
                    i.shutter_speed, i.focal_length
             FROM images i
             JOIN collection_images ci ON i.id = ci.image_id
             WHERE ci.collection_id = ?1
             ORDER BY ci.date_added DESC"
        )?;

        let images = stmt.query_map(params![collection_id], |row| {
            Ok(super::database::CatalogImage {
                id: row.get(0)?,
                file_path: std::path::PathBuf::from(row.get::<_, String>(1)?),
                file_name: row.get(2)?,
                folder_path: std::path::PathBuf::from(row.get::<_, String>(3)?),
                file_size: row.get(4)?,
                width: row.get(5)?,
                height: row.get(6)?,
                date_taken: row.get::<_, Option<String>>(7)?
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
                date_added: row.get::<_, String>(8)?
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap_or_else(|_| chrono::Utc::now()),
                date_modified: row.get::<_, String>(9)?
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap_or_else(|_| chrono::Utc::now()),
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
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(images)
    }

    /// Delete a collection
    pub fn delete_collection(&mut self, collection_id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM collections WHERE id = ?1",
            params![collection_id],
        )?;
        Ok(())
    }

    /// Rename a collection
    pub fn rename_collection(&mut self, collection_id: i64, new_name: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE collections SET name = ?1 WHERE id = ?2",
            params![new_name, collection_id],
        )?;
        Ok(())
    }
}
