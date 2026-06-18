use crate::Result;
use pyroduct::Capture;
use pyroduct::pipeline::factory::PipelineConfig;
use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct DbStateStore {
    conn: Arc<Mutex<Connection>>,
}

impl DbStateStore {
    pub fn open(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path).capture("Failed to open SQLite database")?;

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
        .capture("Failed to initialize database table")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS playbook_callbacks (
                uuid TEXT PRIMARY KEY,
                source TEXT NOT NULL,
                callback_type TEXT NOT NULL,
                target TEXT NOT NULL
            )",
            [],
        )
        .capture("Failed to initialize database callbacks table")?;

        // Migration: add pinned_version column if it doesn't exist
        let _ = conn.execute(
            "ALTER TABLE playbooks ADD COLUMN pinned_version TEXT",
            [],
        );

        // Migration: add http_address column if it doesn't exist
        let _ = conn.execute(
            "ALTER TABLE playbooks ADD COLUMN http_address TEXT",
            [],
        );

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
        pinned_version: Option<&str>,
        http_address: Option<&str>,
    ) -> Result<()> {
        let config_json =
            serde_json::to_string(config).capture("Failed to serialize PipelineConfig to JSON")?;

        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO playbooks (name, status, config, socket_path, pinned_version, http_address)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![name, status, config_json, socket_path, pinned_version, http_address],
        )
        .capture("Failed to save playbook state to database")?;
        Ok(())
    }

    pub async fn get_playbook(
        &self,
        name: &str,
    ) -> Result<Option<(String, PipelineConfig, Option<String>, Option<String>, Option<String>)>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT status, config, socket_path, pinned_version, http_address FROM playbooks WHERE name = ?1")
            .capture("Failed to prepare SELECT statement for playbook")?;
        let mut rows = stmt
            .query(params![name])
            .capture("Failed to query playbook state")?;

        if let Some(row) = rows.next().capture("Failed to advance playbook rows")? {
            let status: String = row.get(0).capture("Failed to read status column")?;
            let config_json: String = row.get(1).capture("Failed to read config column")?;
            let socket_path: Option<String> =
                row.get(2).capture("Failed to read socket_path column")?;
            let pinned_version: Option<String> =
                row.get(3).capture("Failed to read pinned_version column")?;
            let http_address: Option<String> =
                row.get(4).capture("Failed to read http_address column")?;

            let config: PipelineConfig = serde_json::from_str(&config_json)
                .capture("Failed to deserialize PipelineConfig from JSON")?;

            Ok(Some((status, config, socket_path, pinned_version, http_address)))
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
        .capture("Failed to update playbook status in database")?;
        Ok(())
    }

    pub async fn update_http_address(&self, name: &str, http_address: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE playbooks SET http_address = ?1 WHERE name = ?2",
            params![http_address, name],
        )
        .capture("Failed to update playbook HTTP address in database")?;
        Ok(())
    }

    pub async fn delete_playbook(&self, name: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        let _ = conn.execute(
            "DELETE FROM playbook_callbacks WHERE source = ?1",
            params![name],
        );
        conn.execute("DELETE FROM playbooks WHERE name = ?1", params![name])
            .capture("Failed to delete playbook from database")?;
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
        .capture("Failed to insert callback mapping")?;
        Ok(())
    }

    pub async fn delete_callback_mapping(&self, uuid: uuid::Uuid) -> Result<()> {
        let conn = self.conn.lock().await;
        let uuid_str = uuid.to_string();
        conn.execute(
            "DELETE FROM playbook_callbacks WHERE uuid = ?1",
            params![uuid_str],
        )
        .capture("Failed to delete callback mapping")?;
        Ok(())
    }

    pub async fn get_callbacks_for_source(
        &self,
        source: &str,
    ) -> Result<Vec<(uuid::Uuid, String, String, String)>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT uuid, source, callback_type, target FROM playbook_callbacks WHERE source = ?1",
        ).capture("Failed to prepare SELECT statement for callbacks")?;
        let mut rows = stmt
            .query(params![source])
            .capture("Failed to query callbacks")?;
        let mut list = Vec::new();
        while let Some(row) = rows.next().capture("Failed to advance callback rows")? {
            let uuid_str: String = row.get(0).capture("Failed to read callback UUID column")?;
            let uuid = uuid::Uuid::parse_str(&uuid_str).capture("Invalid UUID in database")?;
            let src: String = row
                .get(1)
                .capture("Failed to read callback source column")?;
            let cb_type: String = row.get(2).capture("Failed to read callback type column")?;
            let target: String = row
                .get(3)
                .capture("Failed to read callback target column")?;
            list.push((uuid, src, cb_type, target));
        }
        Ok(list)
    }

    pub async fn list_playbooks(
        &self,
    ) -> Result<Vec<(String, String, PipelineConfig, Option<String>, Option<String>, Option<String>)>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT name, status, config, socket_path, pinned_version, http_address FROM playbooks")
            .capture("Failed to prepare SELECT statement for listing playbooks")?;
        let mut rows = stmt.query([]).capture("Failed to query all playbooks")?;
        let mut list = Vec::new();
        while let Some(row) = rows
            .next()
            .capture("Failed to advance playbook list rows")?
        {
            let name: String = row.get(0).capture("Failed to read playbook name column")?;
            let status: String = row
                .get(1)
                .capture("Failed to read playbook status column")?;
            let config_json: String = row
                .get(2)
                .capture("Failed to read playbook config column")?;
            let socket_path: Option<String> = row
                .get(3)
                .capture("Failed to read playbook socket_path column")?;
            let pinned_version: Option<String> = row
                .get(4)
                .capture("Failed to read playbook pinned_version column")?;
            let http_address: Option<String> = row
                .get(5)
                .capture("Failed to read playbook http_address column")?;
            let config: PipelineConfig = serde_json::from_str(&config_json)
                .capture("Failed to deserialize PipelineConfig from JSON")?;
            list.push((name, status, config, socket_path, pinned_version, http_address));
        }
        Ok(list)
    }
}
