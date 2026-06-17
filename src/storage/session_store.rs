use std::sync::Arc;

use crate::storage::db::AgentDb;
use crate::traits::{SessionStore, StoredMessage};
use async_trait::async_trait;
use tokio::sync::Mutex as AsyncMutex;

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// SQLite-backed implementation of SessionStore.
pub struct SqliteSessionStore {
    db: AgentDb,
}

impl SqliteSessionStore {
    pub fn new(db: AgentDb) -> Self {
        Self { db }
    }

    pub fn db(&self) -> &AgentDb {
        &self.db
    }
}

#[async_trait]
impl SessionStore for SqliteSessionStore {
    async fn load_active_history(
        &self,
        session_id: &str,
    ) -> Result<Vec<(String, String, String)>, String> {
        let db = self.db.clone();
        let sid = session_id.to_string();
        tokio::task::spawn_blocking(move || -> Result<Vec<(String, String, String)>, String> {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, role, content FROM agent_messages \
                     WHERE session_id = ?1 \
                     ORDER BY created_at ASC, id ASC",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(rusqlite::params![sid], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| e.to_string())?);
            }
            Ok(out)
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn write_message(
        &self,
        id: &str,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), String> {
        let db = self.db.clone();
        let id = id.to_string();
        let sid = session_id.to_string();
        let role = role.to_string();
        let content = content.to_string();
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "INSERT INTO agent_messages (id, session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, sid, role, content, now_ms()],
            )
            .map_err(|e| format!("Failed to insert message: {}", e))?;
            conn.execute(
                "UPDATE agent_sessions SET updated_at = ?1 WHERE id = ?2",
                rusqlite::params![now_ms(), sid],
            )
            .map_err(|e| format!("Failed to update session: {}", e))?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn update_session_status(&self, session_id: &str, status: &str) -> Result<(), String> {
        let db = self.db.clone();
        let sid = session_id.to_string();
        let status = status.to_string();
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "UPDATE agent_sessions SET status = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![status, now_ms(), sid],
            )
            .map_err(|e| format!("Failed to update session status: {}", e))?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn insert_tool_call(
        &self,
        id: &str,
        message_id: &str,
        session_id: &str,
        tool_name: &str,
        arguments: &str,
        status: &str,
    ) -> Result<(), String> {
        let db = self.db.clone();
        let id = id.to_string();
        let mid = message_id.to_string();
        let sid = session_id.to_string();
        let name = tool_name.to_string();
        let args = arguments.to_string();
        let st = status.to_string();
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "INSERT INTO agent_tool_calls (id, message_id, session_id, tool_name, arguments, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![id, mid, sid, name, args, st, now_ms()],
            )
            .map_err(|e| format!("Failed to insert tool_call: {}", e))?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn update_tool_call_status(&self, id: &str, status: &str) -> Result<(), String> {
        let db = self.db.clone();
        let id = id.to_string();
        let status = status.to_string();
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "UPDATE agent_tool_calls SET status = ?1 WHERE id = ?2",
                rusqlite::params![status, id],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn insert_tool_result(
        &self,
        tool_call_id: &str,
        full_result: &str,
    ) -> Result<String, String> {
        let db = self.db.clone();
        let result_id = new_id();
        let tcid = tool_call_id.to_string();
        let result = full_result.to_string();
        tokio::task::spawn_blocking(move || -> Result<String, String> {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "INSERT INTO tool_result_store (id, tool_call_id, full_result, created_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![&result_id, tcid, result, now_ms()],
            )
            .map_err(|e| format!("Failed to insert tool_result: {}", e))?;
            Ok(result_id)
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn load_messages_for_compact(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredMessage>, String> {
        let db = self.db.clone();
        let sid = session_id.to_string();
        tokio::task::spawn_blocking(move || -> Result<Vec<StoredMessage>, String> {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, role, content FROM agent_messages \
                     WHERE session_id = ?1 \
                     ORDER BY created_at ASC, id ASC",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(rusqlite::params![sid], |row| {
                    Ok(StoredMessage {
                        id: row.get::<_, String>(0)?,
                        role: row.get::<_, String>(1)?,
                        content: row.get::<_, String>(2)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| e.to_string())?);
            }
            Ok(out)
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn load_all_messages(&self, session_id: &str) -> Result<Vec<StoredMessage>, String> {
        let db = self.db.clone();
        let sid = session_id.to_string();
        tokio::task::spawn_blocking(move || -> Result<Vec<StoredMessage>, String> {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, role, content FROM agent_messages \
                     WHERE session_id = ?1 \
                     ORDER BY created_at ASC, id ASC",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(rusqlite::params![sid], |row| {
                    Ok(StoredMessage {
                        id: row.get::<_, String>(0)?,
                        role: row.get::<_, String>(1)?,
                        content: row.get::<_, String>(2)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| e.to_string())?);
            }
            Ok(out)
        })
        .await
        .map_err(|e| e.to_string())?
    }

    fn compact_lock(&self, session_id: &str) -> Arc<AsyncMutex<()>> {
        // Unified with conversation::lock_for() — single source of truth
        // prevents race between manual compact, in-loop compact, and background compact.
        crate::conversation::lock_for(session_id)
    }
}
