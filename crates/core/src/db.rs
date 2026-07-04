use crate::session::{SessionState, TorrentSession};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

/// Initializes the SQLite database and ensures the schema exists.
pub async fn init_db(db_path: &str) -> Result<SqlitePool, sqlx::Error> {
    // mode=rwc ensures the file is created if it doesn't exist
    let url = format!("sqlite://{}?mode=rwc", db_path);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;

    // Create the sessions table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS torrent_sessions (
            info_hash TEXT PRIMARY KEY,
            display_name TEXT,
            magnet_uri TEXT NOT NULL,
            state TEXT NOT NULL,
            added_at INTEGER NOT NULL,
            source TEXT
        );",
    )
    .execute(&pool)
    .await?;

    // Attempt to add source column if migrating from old schema
    let _ = sqlx::query("ALTER TABLE torrent_sessions ADD COLUMN source TEXT")
        .execute(&pool)
        .await;

    Ok(pool)
}

/// Persists a newly parsed magnet session into SQLite atomically.
pub async fn create_session(
    pool: &SqlitePool,
    session: &TorrentSession,
) -> Result<(), sqlx::Error> {
    let state_str = match session.state {
        SessionState::PendingMetadata => "PENDING_METADATA",
        _ => "UNKNOWN",
    };

    sqlx::query(
        "INSERT OR REPLACE INTO torrent_sessions (info_hash, display_name, magnet_uri, state, added_at, source)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    )
    .bind(&session.info_hash)
    .bind(&session.display_name)
    .bind(&session.magnet_uri)
    .bind(state_str)
    .bind(session.added_at)
    .bind(&session.source)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_session_state(
    pool: &SqlitePool,
    info_hash: &str,
    state: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE torrent_sessions SET state = ? WHERE info_hash = ?")
        .bind(state)
        .bind(info_hash)
        .execute(pool)
        .await?;
    Ok(())
}
