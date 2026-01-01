// src-tauri/src/db.rs

use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::PathBuf;
use tauri::AppHandle;
use sqlx::FromRow;
use serde::Serialize;

#[derive(Serialize, FromRow)]
pub struct Paper 
{
    pub id: i64,
    pub title: String,
    pub pdf_path: String,
    pub created_at: Option<String>,
}



pub async fn init_db(app_handle: &AppHandle) -> Result<SqlitePool, String> {
    println!("Initializing database...");

    let current_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current dir: {}", e))?;

    let db_path = current_dir.join("paper_master.db");
    let clean_path = clean_windows_path(&db_path);
    let forward_slashes = clean_path.to_str().unwrap().replace("\\", "/");
    let db_url = format!("file:{}?mode=rwc", forward_slashes);

    let pool = match SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
    {
        Ok(pool) => {
            println!("Connected to DB!");
            init_tables(&pool).await?;
            pool
        }
        Err(e) => {
            println!("All attempts failed: {}", e);
            return Err(format!("Database connection failed: {}", e));
        }
    };

    Ok(pool)
}

fn clean_windows_path(path: &PathBuf) -> PathBuf {
    let path_str = path.to_str().unwrap_or("");
    if path_str.starts_with(r"\\?\") {
        PathBuf::from(&path_str[4..])
    } else {
        path.clone()
    }
}

async fn init_tables(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS papers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            authors TEXT,
            journal TEXT,
            year INTEGER,
            pdf_path TEXT NOT NULL,
            tags TEXT,
            notes TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to create table: {}", e))?;

    Ok(())
}

pub async fn insert_paper(
    pool: &SqlitePool,
    title: &str,
    pdf_path: &str,
) -> Result<i64, String> {
    let result = sqlx::query(
        r#"
        INSERT INTO papers (title, pdf_path)
        VALUES (?, ?)
        "#
    )
    .bind(title)
    .bind(pdf_path)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(result.last_insert_rowid())
}



pub async fn get_all_papers(pool: &SqlitePool) -> Result<Vec<Paper>, String>
{
    let papers = sqlx::query_as::<_,Paper>(
        "SELECT id, title, pdf_path, created_at FROM papers ORDER BY created_at DESC"
    ).fetch_all(pool).await.map_err(|e| format!("Faled to fetch the papers: {}",e))?;

    Ok(papers)
}
