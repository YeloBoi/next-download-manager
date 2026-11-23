use ndm_core::{Snapshot, Usage};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;

type Result<T> = std::result::Result<T, String>;

#[derive(Clone)]
pub struct Store {
    path: PathBuf,
}

impl Store {
    pub async fn open(path: PathBuf) -> Result<Self> {
        let store = Self { path };
        store.run(|connection| {
            connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
                CREATE TABLE IF NOT EXISTS state (id INTEGER PRIMARY KEY CHECK(id = 1), version INTEGER NOT NULL, snapshot TEXT NOT NULL);
                CREATE TABLE IF NOT EXISTS usage (device TEXT NOT NULL, hour INTEGER NOT NULL, category TEXT NOT NULL, bytes INTEGER NOT NULL,
                  PRIMARY KEY(device, hour, category));")?;
            Ok(())
        }).await?;
        Ok(store)
    }

    async fn run<T: Send + 'static>(
        &self,
        action: impl FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
    ) -> Result<T> {
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut connection = Connection::open(path).map_err(|e| e.to_string())?;
            connection
                .busy_timeout(std::time::Duration::from_secs(5))
                .map_err(|e| e.to_string())?;
            connection
                .pragma_update(None, "synchronous", "FULL")
                .map_err(|e| e.to_string())?;
            action(&mut connection).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    pub async fn load(&self) -> Result<Option<Snapshot>> {
        let row: Option<(u32, String)> = self
            .run(|connection| {
                connection
                    .query_row(
                        "SELECT version, snapshot FROM state WHERE id=1",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .optional()
            })
            .await?;
        match row {
            Some((1, json)) => serde_json::from_str(&json).map(Some).map_err(|e| format!("Cannot load saved queue: {e}")),
            Some((version, _)) => Err(format!("Unsupported database schema {version}. Keep this database and use a compatible NDM version.")),
            None => Ok(None)
        }
    }

    pub async fn save(&self, snapshot: &Snapshot) -> Result<()> {
        self.save_with_usage(snapshot, None).await
    }

    pub async fn save_with_usage(&self, snapshot: &Snapshot, entry: Option<Usage>) -> Result<()> {
        let json = serde_json::to_string(snapshot).map_err(|e| e.to_string())?;
        self.run(move |connection| {
            let transaction = connection.transaction()?;
            transaction.execute("INSERT INTO state (id,version,snapshot) VALUES(1,1,?1) ON CONFLICT(id) DO UPDATE SET version=1,snapshot=excluded.snapshot", params![json])?;
            if let Some(entry) = entry {
                transaction.execute("INSERT INTO usage VALUES('pc',?1,?2,?3) ON CONFLICT(device,hour,category) DO UPDATE SET bytes = usage.bytes + excluded.bytes",
                    params![entry.hour, entry.category, entry.bytes])?;
            }
            transaction.commit()
        }).await
    }

    pub async fn merge_usage(&self, device: String, entries: Vec<Usage>) -> Result<()> {
        self.run(move |connection| {
            let transaction = connection.transaction()?;
            for entry in entries {
                transaction.execute("INSERT INTO usage VALUES(?1,?2,?3,?4) ON CONFLICT(device,hour,category) DO UPDATE SET bytes = max(usage.bytes, excluded.bytes)",
                    params![device, entry.hour, entry.category, entry.bytes])?;
            }
            transaction.commit()
        }).await
    }

    pub async fn usage_since(&self, hour: i64) -> Result<Vec<Usage>> {
        self.run(move |connection| {
            let mut query = connection.prepare("SELECT hour, category, CAST(total(bytes) AS INTEGER), CASE WHEN device='pc' THEN 'pc' ELSE 'mobile' END AS source FROM usage WHERE hour>=?1 GROUP BY hour,category,source ORDER BY hour")?;
            let rows = query.query_map([hour], |row| Ok(Usage { hour: row.get(0)?, category: row.get(1)?, bytes: row.get(2)?, device: row.get(3)? }))?;
            rows.collect()
        }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndm_core::Settings;
    #[tokio::test]
    async fn durable_round_trip_and_corrupt_data_is_not_silently_reset() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::open(dir.path().join("state.db"))
            .await
            .expect("open");
        assert!(store.load().await.expect("load").is_none());
        let snapshot = Snapshot {
            user_paused: false,
            downloads: vec![],
            settings: Settings::new(dir.path().to_path_buf()),
            running: false,
            draining: false,
        };
        store.save(&snapshot).await.expect("save");
        assert_eq!(
            store
                .load()
                .await
                .expect("load")
                .expect("state")
                .settings
                .language,
            "en"
        );
        store
            .run(|c| c.execute("UPDATE state SET snapshot='broken'", []))
            .await
            .expect("update");
        assert!(store.load().await.is_err());
    }
}
