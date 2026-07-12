//! Локальная история расшифровок (SQLite через rusqlite).
//! Всё под одним корнем данных (SPEC: чистая установка без мусора по системе).

use std::path::PathBuf;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoredJob {
    pub id: String,
    pub name: String,
    pub path: String,
    pub diarize: bool,
    pub status: String, // "done" | "error"
    pub text: String,
    pub error: String,
    pub created_at: i64,
    pub speakers: String, // JSON {номер: имя}
    #[serde(default)]
    pub duration_sec: Option<f64>, // длительность записи, сек (для столбца в истории)
}

/// Единый корень данных приложения.
pub fn data_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("SpeakAgent");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn conn() -> Result<Connection, String> {
    let c = Connection::open(data_dir().join("speakagent.db")).map_err(|e| e.to_string())?;
    c.execute(
        "CREATE TABLE IF NOT EXISTS jobs (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL,
            diarize INTEGER NOT NULL,
            status TEXT NOT NULL,
            text TEXT NOT NULL,
            error TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            speakers TEXT NOT NULL DEFAULT '{}',
            duration_sec REAL
        )",
        [],
    )
    .map_err(|e| e.to_string())?;
    // Миграции для БД, созданных до появления колонок (ошибку «колонка есть» игнорируем).
    let _ = c.execute("ALTER TABLE jobs ADD COLUMN speakers TEXT NOT NULL DEFAULT '{}'", []);
    let _ = c.execute("ALTER TABLE jobs ADD COLUMN duration_sec REAL", []);
    c.execute(
        "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    )
    .map_err(|e| e.to_string())?;
    // «Итоги встречи»: N артефактов на запись (summary/business/interview/todo)
    // + служебный 'digest' (кэш map-reduce, в UI не показывается).
    c.execute(
        "CREATE TABLE IF NOT EXISTS job_results (
            job_id TEXT NOT NULL,
            kind TEXT NOT NULL,
            text TEXT NOT NULL,
            model TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            PRIMARY KEY (job_id, kind)
        )",
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(c)
}

pub fn get_setting(key: &str) -> Option<String> {
    let c = conn().ok()?;
    c.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
        .ok()
}

pub fn set_setting(key: &str, value: &str) -> Result<(), String> {
    conn()?
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn save(j: &StoredJob) -> Result<(), String> {
    conn()?
        .execute(
            "INSERT OR REPLACE INTO jobs (id, name, path, diarize, status, text, error, created_at, speakers, duration_sec)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                j.id, j.name, j.path, j.diarize as i32, j.status, j.text, j.error, j.created_at, j.speakers, j.duration_sec
            ],
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn list() -> Result<Vec<StoredJob>, String> {
    let c = conn()?;
    let mut stmt = c
        .prepare("SELECT id, name, path, diarize, status, text, error, created_at, speakers, duration_sec FROM jobs ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(StoredJob {
                id: r.get(0)?,
                name: r.get(1)?,
                path: r.get(2)?,
                diarize: r.get::<_, i32>(3)? != 0,
                status: r.get(4)?,
                text: r.get(5)?,
                error: r.get(6)?,
                created_at: r.get(7)?,
                speakers: r.get(8)?,
                duration_sec: r.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn get(id: &str) -> Option<StoredJob> {
    let c = conn().ok()?;
    c.query_row(
        "SELECT id, name, path, diarize, status, text, error, created_at, speakers, duration_sec FROM jobs WHERE id = ?1",
        [id],
        |r| {
            Ok(StoredJob {
                id: r.get(0)?,
                name: r.get(1)?,
                path: r.get(2)?,
                diarize: r.get::<_, i32>(3)? != 0,
                status: r.get(4)?,
                text: r.get(5)?,
                error: r.get(6)?,
                created_at: r.get(7)?,
                speakers: r.get(8)?,
                duration_sec: r.get(9)?,
            })
        },
    )
    .ok()
}

pub fn delete(id: &str) -> Result<(), String> {
    let c = conn()?;
    c.execute("DELETE FROM job_results WHERE job_id = ?1", [id])
        .map_err(|e| e.to_string())?;
    c.execute("DELETE FROM jobs WHERE id = ?1", [id])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn clear() -> Result<(), String> {
    let c = conn()?;
    c.execute("DELETE FROM job_results", []).map_err(|e| e.to_string())?;
    c.execute("DELETE FROM jobs", [])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// ── «Итоги встречи» ──

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JobResult {
    pub job_id: String,
    pub kind: String, // "summary" | "business" | "interview" | "todo" | "digest"
    pub text: String,
    pub model: String,
    pub created_at: i64,
}

pub fn save_result(r: &JobResult) -> Result<(), String> {
    conn()?
        .execute(
            "INSERT OR REPLACE INTO job_results (job_id, kind, text, model, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![r.job_id, r.kind, r.text, r.model, r.created_at],
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Артефакты записи для UI ('digest' — служебный, не отдаём).
pub fn results_for(job_id: &str) -> Result<Vec<JobResult>, String> {
    let c = conn()?;
    let mut stmt = c
        .prepare(
            "SELECT job_id, kind, text, model, created_at FROM job_results
             WHERE job_id = ?1 AND kind != 'digest' ORDER BY created_at",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([job_id], |r| {
            Ok(JobResult {
                job_id: r.get(0)?,
                kind: r.get(1)?,
                text: r.get(2)?,
                model: r.get(3)?,
                created_at: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Кэшированный дайджест map-reduce (если запись длинная и уже разбиралась).
pub fn digest_for(job_id: &str) -> Option<String> {
    let c = conn().ok()?;
    c.query_row(
        "SELECT text FROM job_results WHERE job_id = ?1 AND kind = 'digest'",
        [job_id],
        |r| r.get(0),
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_list_delete_roundtrip() {
        let id = "test_selfcheck_9271".to_string();
        let j = StoredJob {
            id: id.clone(),
            name: "тест".into(),
            path: "p".into(),
            diarize: true,
            status: "done".into(),
            text: "привет".into(),
            error: "".into(),
            created_at: 123,
            speakers: "{}".into(),
            duration_sec: Some(42.0),
        };
        save(&j).unwrap();
        let found = list().unwrap().into_iter().find(|x| x.id == id).unwrap();
        assert_eq!(found.text, "привет");
        assert!(found.diarize);
        delete(&id).unwrap();
        assert!(list().unwrap().into_iter().all(|x| x.id != id));
    }
}
