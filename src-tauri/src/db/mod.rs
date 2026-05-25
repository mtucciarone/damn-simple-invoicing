use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};
use tauri::{path::BaseDirectory, AppHandle, Manager};

use crate::error::{AppError, AppResult};

const DATABASE_FILE_NAME: &str = "damn-simple-invoicing.sqlite3";

pub struct Database {
    path: PathBuf,
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
        }
    }
}

impl Database {
    pub fn from_app_handle(app: &AppHandle) -> AppResult<Self> {
        let path = app
            .path()
            .resolve(DATABASE_FILE_NAME, BaseDirectory::AppLocalData)
            .map_err(|err| AppError::Backup(err.to_string()))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn open(&self) -> AppResult<Connection> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let flags = OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_READ_WRITE;
        let connection = Connection::open_with_flags(&self.path, flags)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
       PRAGMA journal_mode = WAL;
       PRAGMA synchronous = NORMAL;",
        )?;
        Ok(connection)
    }

    pub fn migrate(&self) -> AppResult<()> {
        let mut connection = self.open()?;
        apply_migrations(&mut connection)?;
        Ok(())
    }

    pub fn checkpoint(&self) -> AppResult<()> {
        let connection = self.open()?;
        connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }
}

pub fn apply_migrations(connection: &mut Connection) -> AppResult<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
      version TEXT PRIMARY KEY,
      applied_at TEXT NOT NULL
    );",
    )?;

    let applied_versions = {
        let mut statement = connection.prepare("SELECT version FROM schema_migrations")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut versions = Vec::new();
        for row in rows {
            versions.push(row?);
        }
        versions
    };

    for (version, sql) in migrations() {
        if applied_versions.iter().any(|existing| existing == version) {
            continue;
        }

        let transaction = connection.transaction()?;
        transaction.execute_batch(sql)?;
        transaction.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![version, chrono::Utc::now().to_rfc3339()],
        )?;
        transaction.commit()?;
    }

    Ok(())
}

fn migrations() -> [(&'static str, &'static str); 2] {
    [
        (
            "0001_init.sql",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/migrations/0001_init.sql"
            )),
        ),
        (
            "0002_seed_settings.sql",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/migrations/0002_seed_settings.sql"
            )),
        ),
    ]
}
