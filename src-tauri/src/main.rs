// src-tauri/src/main.rs

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
use db::{init_db, insert_paper, get_all_papers};
use tauri::{State, Manager};
use sqlx::SqlitePool;
use std::fs;


use tauri_plugin_dialog::{DialogExt, FilePath};

struct AppState {
    db: SqlitePool,
}


#[tauri::command]
async fn read_pdf_file(path: String) -> Result<Vec<u8>, String> {
    use tokio::fs;
    let bytes = fs::read(&path)
        .await
        .map_err(|e| format!("Failed to read PDF: {}", e))?;
    Ok(bytes)
}



#[tauri::command]
async fn  get_papers(state: State<'_, AppState>)->Result<Vec<db::Paper>, String>
{
    get_all_papers(&state.db)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! The app is ready 🦀", name)
}

#[tauri::command]
async fn db_test(state: State<'_, AppState>) -> Result<String, String> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM papers")
        .fetch_one(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("Database connected. Papers stored: {}", row.0))
}

#[tauri::command]
async fn add_paper(handle: tauri::AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    //bridge callback-based dialog API into async
    let (tx, rx) = tokio::sync::oneshot::channel();

    handle
        .dialog()
        .file()
        .add_filter("PDF Files", &["pdf"])
        .pick_file(move |file_path| {
            let _ = tx.send(file_path);
        });

    // Wait for user selection
    let selected_path = rx.await.map_err(|_| "File dialog cancelled".to_string())?;

    // Convert FilePath enum into PathBuf
    let selected_path = match selected_path {
        Some(FilePath::Path(path)) => path,
        Some(FilePath::Url(_)) => return Err("URL selection not supported".to_string()),
        None => return Ok("No file selected".to_string()),
    };

    // Resolve app-local data directory
    let app_data_dir = handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Path resolve error: {}", e))?;

    let papers_dir = app_data_dir.join("papers");
    fs::create_dir_all(&papers_dir).map_err(|e| e.to_string())?;

    // Extract filename
    let file_name = selected_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid file name")?
        .to_string();

    let dest_path = papers_dir.join(&file_name);

    // Avoid overwriting existing files
    let mut counter = 1;
    let mut final_dest = dest_path.clone();
    while final_dest.exists() {
        let new_name = format!("{}_{}.pdf", file_name.trim_end_matches(".pdf"), counter);
        final_dest = papers_dir.join(new_name);
        counter += 1;
    }

    // Copy file into app storage
    fs::copy(&selected_path, &final_dest).map_err(|e| format!("Copy failed: {}", e))?;

    
    let title = final_dest
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();

    let internal_path = final_dest.to_str().unwrap().to_string();

    // Insert metadata into database
    insert_paper(&state.db, &title, &internal_path)
        .await
        .map_err(|e| format!("Database insert failed: {}", e))?;

    Ok(format!("Paper added successfully: {}", title))
}



fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let pool = tauri::async_runtime::block_on(init_db(&handle))
                .expect("Failed to connect to database");
            app.manage(AppState { db: pool });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, db_test, add_paper, get_papers, read_pdf_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

