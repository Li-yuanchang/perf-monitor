use chrono::{DateTime, Utc, Duration};
use sqlx::{sqlite::{SqlitePoolOptions, SqliteConnectOptions}, Pool, Sqlite, Row};
use serde::{Serialize, Deserialize};
use std::str::FromStr;

use crate::collector::Metric;

pub struct Store {
    pool: Pool<Sqlite>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeMark {
    pub id: i64,
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub note: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessRanking {
    pub name: String,
    pub avg_cpu: f64,
    pub peak_cpu: f64,
    pub avg_mem: f64,
    pub peak_mem: f64,
    pub avg_rss_mb: f64,
    pub peak_rss_mb: f64,
    pub sample_count: i64,
}

impl Store {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let home_dir = dirs::home_dir().ok_or("Cannot get home dir")?;
        let data_dir = home_dir.join(".macos-perf-monitor");
        tokio::fs::create_dir_all(&data_dir).await?;

        let db_path = data_dir.join("metrics.db");
        let database_url = format!("sqlite:{}", db_path.to_string_lossy());

        let connect_options = SqliteConnectOptions::from_str(&database_url)?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(connect_options)
            .await?;

        let store = Self { pool };
        store.init_db().await?;

        Ok(store)
    }

    async fn init_db(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                cpu REAL DEFAULT 0,
                memory REAL DEFAULT 0,
                swap_used REAL DEFAULT 0,
                disk_usage REAL DEFAULT 0,
                load_avg REAL DEFAULT 0
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_metrics_timestamp ON metrics(timestamp)"
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS process_snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                pid INTEGER,
                name TEXT,
                cpu REAL DEFAULT 0,
                mem REAL DEFAULT 0,
                rss INTEGER DEFAULT 0
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_process_timestamp ON process_snapshots(timestamp)"
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_process_name ON process_snapshots(name)"
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS time_marks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE,
                timestamp TEXT NOT NULL,
                note TEXT
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn insert_metric(&self, metric: &Metric) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO metrics (timestamp, cpu, memory, swap_used, disk_usage, load_avg)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#
        )
        .bind(metric.timestamp.to_rfc3339())
        .bind(metric.cpu)
        .bind(metric.memory)
        .bind(metric.swap_used)
        .bind(metric.disk_usage)
        .bind(metric.load_avg)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_latest_metric(&self) -> Result<Option<Metric>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id, timestamp, cpu, memory, swap_used, disk_usage, load_avg
            FROM metrics
            ORDER BY timestamp DESC
            LIMIT 1
            "#
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let timestamp_str: String = row.get("timestamp");
                let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                    .unwrap_or_else(|_| chrono::DateTime::from_timestamp(0, 0).unwrap().into())
                    .with_timezone(&Utc);

                Ok(Some(Metric {
                    id: row.get("id"),
                    timestamp,
                    cpu: row.get("cpu"),
                    memory: row.get("memory"),
                    swap_used: row.get("swap_used"),
                    disk_usage: row.get("disk_usage"),
                    load_avg: row.get("load_avg"),
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn query_metrics(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Metric>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, timestamp, cpu, memory, swap_used, disk_usage, load_avg
            FROM metrics
            WHERE timestamp >= ?1 AND timestamp <= ?2
            ORDER BY timestamp
            "#
        )
        .bind(start.to_rfc3339())
        .bind(end.to_rfc3339())
        .fetch_all(&self.pool)
        .await?;

        let mut metrics = Vec::new();
        for row in rows {
            let timestamp_str: String = row.get("timestamp");
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .unwrap_or_else(|_| chrono::DateTime::from_timestamp(0, 0).unwrap().into())
                .with_timezone(&Utc);

            metrics.push(Metric {
                id: row.get("id"),
                timestamp,
                cpu: row.get("cpu"),
                memory: row.get("memory"),
                swap_used: row.get("swap_used"),
                disk_usage: row.get("disk_usage"),
                load_avg: row.get("load_avg"),
            });
        }

        Ok(metrics)
    }

    pub async fn set_time_mark(&self, name: &str, note: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO time_marks (name, timestamp, note)
            VALUES (?1, ?2, ?3)
            "#
        )
        .bind(name)
        .bind(Utc::now().to_rfc3339())
        .bind(note)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_time_marks(&self) -> Result<Vec<TimeMark>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, timestamp, note
            FROM time_marks
            ORDER BY timestamp
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut marks = Vec::new();
        for row in rows {
            let timestamp_str: String = row.get("timestamp");
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .unwrap_or_else(|_| chrono::DateTime::from_timestamp(0, 0).unwrap().into())
                .with_timezone(&Utc);

            marks.push(TimeMark {
                id: row.get("id"),
                name: row.get("name"),
                timestamp,
                note: row.get("note"),
            });
        }

        Ok(marks)
    }

    pub async fn save_process_snapshot(&self, processes: &[crate::collector::ProcessInfo]) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        for p in processes {
            sqlx::query(
                "INSERT INTO process_snapshots (timestamp, pid, name, cpu, mem, rss) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
            )
            .bind(&now)
            .bind(p.pid)
            .bind(&p.name)
            .bind(p.cpu)
            .bind(p.mem)
            .bind(p.rss)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn get_process_ranking(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<ProcessRanking>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT name,
                   ROUND(AVG(cpu), 1) as avg_cpu,
                   ROUND(MAX(cpu), 1) as peak_cpu,
                   ROUND(AVG(mem), 1) as avg_mem,
                   ROUND(MAX(mem), 1) as peak_mem,
                   ROUND(AVG(rss) / 1024.0 / 1024.0, 1) as avg_rss_mb,
                   ROUND(MAX(rss) / 1024.0 / 1024.0, 1) as peak_rss_mb,
                   COUNT(*) as sample_count
            FROM process_snapshots
            WHERE timestamp >= ?1 AND timestamp <= ?2
            GROUP BY name
            ORDER BY avg_mem DESC
            LIMIT 20
            "#
        )
        .bind(start.to_rfc3339())
        .bind(end.to_rfc3339())
        .fetch_all(&self.pool)
        .await?;

        let mut rankings = Vec::new();
        for row in rows {
            rankings.push(ProcessRanking {
                name: row.get("name"),
                avg_cpu: row.get("avg_cpu"),
                peak_cpu: row.get("peak_cpu"),
                avg_mem: row.get("avg_mem"),
                peak_mem: row.get("peak_mem"),
                avg_rss_mb: row.get("avg_rss_mb"),
                peak_rss_mb: row.get("peak_rss_mb"),
                sample_count: row.get("sample_count"),
            });
        }
        Ok(rankings)
    }

    pub async fn delete_time_mark(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM time_marks WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn cleanup_old_data(&self, days: i64) -> Result<(), sqlx::Error> {
        let cutoff = Utc::now() - Duration::days(days);

        sqlx::query("DELETE FROM metrics WHERE timestamp < ?1")
            .bind(cutoff.to_rfc3339())
            .execute(&self.pool)
            .await?;

        sqlx::query("DELETE FROM process_snapshots WHERE timestamp < ?1")
            .bind(cutoff.to_rfc3339())
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
