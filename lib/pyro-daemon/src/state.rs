use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use rusqlite::{params, Connection};
use pyroduct::pipeline::factory::PipelineConfig;

#[derive(Clone)]
pub struct DbStateStore {
    conn: Arc<Mutex<Connection>>,
}

impl DbStateStore {
    pub fn open(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .context("Failed to open SQLite database")?;
        
        // Initialize table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS playbooks (
                name TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                config TEXT NOT NULL,
                socket_path TEXT
            )",
            [],
        )
        .context("Failed to initialize database table")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS playbook_callbacks (
                uuid TEXT PRIMARY KEY,
                source TEXT NOT NULL,
                callback_type TEXT NOT NULL,
                target TEXT NOT NULL
            )",
            [],
        )
        .context("Failed to initialize database callbacks table")?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn save_playbook(
        &self,
        name: &str,
        status: &str,
        config: &PipelineConfig,
        socket_path: Option<&str>,
    ) -> Result<()> {
        let config_json = serde_json::to_string(config)
            .context("Failed to serialize PipelineConfig to JSON")?;
        
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO playbooks (name, status, config, socket_path)
             VALUES (?1, ?2, ?3, ?4)",
            params![name, status, config_json, socket_path],
        )
        .context("Failed to save playbook state to database")?;
        Ok(())
    }

    pub async fn get_playbook(
        &self,
        name: &str,
    ) -> Result<Option<(String, PipelineConfig, Option<String>)>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT status, config, socket_path FROM playbooks WHERE name = ?1",
        )?;
        let mut rows = stmt.query(params![name])?;
        
        if let Some(row) = rows.next()? {
            let status: String = row.get(0)?;
            let config_json: String = row.get(1)?;
            let socket_path: Option<String> = row.get(2)?;
            
            let config: PipelineConfig = serde_json::from_str(&config_json)
                .context("Failed to deserialize PipelineConfig from JSON")?;
            
            Ok(Some((status, config, socket_path)))
        } else {
            Ok(None)
        }
    }

    pub async fn update_status(&self, name: &str, status: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE playbooks SET status = ?1 WHERE name = ?2",
            params![status, name],
        )
        .context("Failed to update playbook status in database")?;
        Ok(())
    }

    pub async fn delete_playbook(&self, name: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        let _ = conn.execute(
            "DELETE FROM playbook_callbacks WHERE source = ?1",
            params![name],
        );
        conn.execute(
            "DELETE FROM playbooks WHERE name = ?1",
            params![name],
        )
        .context("Failed to delete playbook from database")?;
        Ok(())
    }

    pub async fn add_callback_mapping(
        &self,
        uuid: uuid::Uuid,
        source: &str,
        callback_type: &str,
        target: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().await;
        let uuid_str = uuid.to_string();
        conn.execute(
            "INSERT INTO playbook_callbacks (uuid, source, callback_type, target)
             VALUES (?1, ?2, ?3, ?4)",
            params![uuid_str, source, callback_type, target],
        )
        .context("Failed to insert callback mapping")?;
        Ok(())
    }

    pub async fn delete_callback_mapping(&self, uuid: uuid::Uuid) -> Result<()> {
        let conn = self.conn.lock().await;
        let uuid_str = uuid.to_string();
        conn.execute(
            "DELETE FROM playbook_callbacks WHERE uuid = ?1",
            params![uuid_str],
        )
        .context("Failed to delete callback mapping")?;
        Ok(())
    }

    pub async fn get_callbacks_for_source(&self, source: &str) -> Result<Vec<(uuid::Uuid, String, String, String)>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT uuid, source, callback_type, target FROM playbook_callbacks WHERE source = ?1",
        )?;
        let mut rows = stmt.query(params![source])?;
        let mut list = Vec::new();
        while let Some(row) = rows.next()? {
            let uuid_str: String = row.get(0)?;
            let uuid = uuid::Uuid::parse_str(&uuid_str).context("Invalid UUID in database")?;
            let src: String = row.get(1)?;
            let cb_type: String = row.get(2)?;
            let target: String = row.get(3)?;
            list.push((uuid, src, cb_type, target));
        }
        Ok(list)
    }

    pub async fn list_playbooks(&self) -> Result<Vec<(String, String, PipelineConfig, Option<String>)>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT name, status, config, socket_path FROM playbooks",
        )?;
        let mut rows = stmt.query([])?;
        let mut list = Vec::new();
        while let Some(row) = rows.next()? {
            let name: String = row.get(0)?;
            let status: String = row.get(1)?;
            let config_json: String = row.get(2)?;
            let socket_path: Option<String> = row.get(3)?;
            let config: PipelineConfig = serde_json::from_str(&config_json)
                .context("Failed to deserialize PipelineConfig from JSON")?;
            list.push((name, status, config, socket_path));
        }
        Ok(list)
    }
}
