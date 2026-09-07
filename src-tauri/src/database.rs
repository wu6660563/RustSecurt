use std::sync::Mutex;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use tauri::{AppHandle, Manager};

use crate::models::HiddenItem;

pub struct Database(pub Mutex<Connection>);

impl Database {
    pub fn open(app: &AppHandle) -> Result<Self, String> {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| error.to_string())?;
        std::fs::create_dir_all(&data_dir).map_err(|error| error.to_string())?;
        let connection =
            Connection::open(data_dir.join("filehide.db")).map_err(|error| error.to_string())?;
        Self::initialize(&connection)?;
        Ok(Self(Mutex::new(connection)))
    }

    fn initialize(connection: &Connection) -> Result<(), String> {
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = FULL;
            CREATE TABLE IF NOT EXISTS hidden_item (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL,
                item_type TEXT NOT NULL,
                original_attributes INTEGER NOT NULL,
                current_status INTEGER NOT NULL,
                create_time TEXT NOT NULL,
                update_time TEXT NOT NULL,
                file_id TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_hidden_item_status ON hidden_item(current_status);
            CREATE TABLE IF NOT EXISTS app_setting (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
            )
            .map_err(|error| error.to_string())?;
        let columns: Vec<String> = connection
            .prepare("PRAGMA table_info(hidden_item)")
            .map_err(|e| e.to_string())?
            .query_map([], |row| row.get(1))
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        if !columns.iter().any(|name| name == "file_id") {
            connection
                .execute("ALTER TABLE hidden_item ADD COLUMN file_id TEXT", [])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    #[cfg(test)]
    fn in_memory() -> Self {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        Self::initialize(&connection).expect("initialize in-memory database");
        Self(Mutex::new(connection))
    }

    pub fn active_item_for_path(&self, path: &str) -> Result<Option<i64>, String> {
        self.0.lock().map_err(|_| "数据库锁定失败".to_string())?
            .query_row("SELECT id FROM hidden_item WHERE path = ?1 AND current_status = 1 ORDER BY id DESC LIMIT 1", [path], |row| row.get(0))
            .optional().map_err(|error| error.to_string())
    }

    pub fn insert(
        &self,
        path: &str,
        item_type: &str,
        original_attributes: u32,
    ) -> Result<i64, String> {
        self.insert_with_file_id(path, item_type, original_attributes, None)
    }

    pub fn insert_with_file_id(
        &self,
        path: &str,
        item_type: &str,
        original_attributes: u32,
        file_id: Option<&str>,
    ) -> Result<i64, String> {
        let now = Utc::now().to_rfc3339();
        let connection = self.0.lock().map_err(|_| "数据库锁定失败".to_string())?;
        connection.execute("INSERT INTO hidden_item(path, item_type, original_attributes, current_status, create_time, update_time, file_id) VALUES (?1, ?2, ?3, 1, ?4, ?4, ?5)", params![path, item_type, original_attributes, now, file_id]).map_err(|error| error.to_string())?;
        Ok(connection.last_insert_rowid())
    }

    pub fn get(&self, id: i64) -> Result<HiddenItem, String> {
        self.0.lock().map_err(|_| "数据库锁定失败".to_string())?
            .query_row("SELECT id, path, item_type, original_attributes, current_status, create_time, update_time, file_id FROM hidden_item WHERE id = ?1", [id], map_item)
            .map_err(|_| "未找到隐藏记录".to_string())
    }

    pub fn list(&self) -> Result<Vec<HiddenItem>, String> {
        let connection = self.0.lock().map_err(|_| "数据库锁定失败".to_string())?;
        let mut statement = connection.prepare("SELECT id, path, item_type, original_attributes, current_status, create_time, update_time, file_id FROM hidden_item ORDER BY update_time DESC").map_err(|error| error.to_string())?;
        let items = statement
            .query_map([], map_item)
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string());
        items
    }

    pub fn update_path_and_file_id(
        &self,
        id: i64,
        path: &str,
        file_id: Option<&str>,
    ) -> Result<(), String> {
        self.0
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?
            .execute(
                "UPDATE hidden_item SET path = ?2, file_id = ?3, update_time = ?4 WHERE id = ?1",
                params![id, path, file_id, Utc::now().to_rfc3339()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn auto_lock_minutes(&self) -> Result<u32, String> {
        let value: Option<String> = self
            .0
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?
            .query_row(
                "SELECT value FROM app_setting WHERE key = 'auto_lock_minutes'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(value.and_then(|v| v.parse().ok()).unwrap_or(15))
    }

    pub fn set_auto_lock_minutes(&self, minutes: u32) -> Result<(), String> {
        self.0.lock().map_err(|_| "数据库锁定失败".to_string())?.execute("INSERT INTO app_setting(key, value) VALUES ('auto_lock_minutes', ?1) ON CONFLICT(key) DO UPDATE SET value = excluded.value", [minutes.to_string()]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn mark_restored(&self, id: i64) -> Result<(), String> {
        let changed = self.0.lock().map_err(|_| "数据库锁定失败".to_string())?
            .execute("UPDATE hidden_item SET current_status = 0, update_time = ?2 WHERE id = ?1 AND current_status = 1", params![id, Utc::now().to_rfc3339()])
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            return Err("隐藏记录不存在或已恢复".into());
        }
        Ok(())
    }

    pub fn remove_active_item(&self, id: i64) -> Result<(), String> {
        let changed = self
            .0
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?
            .execute(
                "DELETE FROM hidden_item WHERE id = ?1 AND current_status = 1",
                [id],
            )
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            return Err("无法清理未完成的隐藏记录".into());
        }
        Ok(())
    }

    pub fn password_hash(&self) -> Result<Option<String>, String> {
        self.0
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?
            .query_row(
                "SELECT value FROM app_setting WHERE key = 'access_password_hash'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    pub fn set_password_hash(&self, password_hash: &str) -> Result<(), String> {
        self.0
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?
            .execute(
                "INSERT INTO app_setting(key, value) VALUES ('access_password_hash', ?1) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                [password_hash],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn clear_password_hash(&self) -> Result<(), String> {
        self.0
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?
            .execute(
                "DELETE FROM app_setting WHERE key = 'access_password_hash'",
                [],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

fn map_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<HiddenItem> {
    Ok(HiddenItem {
        id: row.get(0)?,
        path: row.get(1)?,
        item_type: row.get(2)?,
        original_attributes: row.get(3)?,
        current_status: row.get(4)?,
        protection_status: "UNKNOWN".into(),
        create_time: row.get(5)?,
        update_time: row.get(6)?,
        file_id: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::Database;

    #[test]
    fn remove_active_item_removes_only_the_compensating_history_row() {
        let database = Database::in_memory();
        let id = database.insert(r"C:\\test.txt", "FILE", 0x20).unwrap();

        database.remove_active_item(id).unwrap();

        assert_eq!(
            database.active_item_for_path(r"C:\\test.txt").unwrap(),
            None
        );
    }
}
