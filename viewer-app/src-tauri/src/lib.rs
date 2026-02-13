pub mod db;

use db::ViewerDb;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher, EventKind};
use std::sync::Mutex;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

/// Application state holding the database connection
pub struct AppState {
    pub db: Mutex<Option<ViewerDb>>,
    pub db_path: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            db: Mutex::new(None),
            db_path: String::new(),
        }
    }

    pub fn with_db(db: ViewerDb, path: String) -> Self {
        Self {
            db: Mutex::new(Some(db)),
            db_path: path,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Greet command for testing
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to the Viewer App.", name)
}

/// Get paginated content sources
#[tauri::command]
fn get_content_sources(
    state: State<'_, AppState>,
    page: i32,
    limit: i32,
) -> Result<db::PaginatedResponse<db::ContentSourceView>, String> {
    let db_guard = state.db.lock().map_err(|e| e.to_string())?;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;
    db.get_sources(page, limit).map_err(|e| e.to_string())
}

/// Get full content detail by ehl_doc_id
#[tauri::command]
fn get_content_detail(
    state: State<'_, AppState>,
    ehl_doc_id: String,
) -> Result<db::ContentDetail, String> {
    let db_guard = state.db.lock().map_err(|e| e.to_string())?;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;
    db.get_detail(&ehl_doc_id).map_err(|e| e.to_string())
}

/// Get database statistics
#[tauri::command]
fn get_stats(state: State<'_, AppState>) -> Result<db::DbStats, String> {
    let db_guard = state.db.lock().map_err(|e| e.to_string())?;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;
    db.get_stats().map_err(|e| e.to_string())
}

/// Delete a content source by ehl_doc_id
#[tauri::command]
fn delete_content_source(
    state: State<'_, AppState>,
    ehl_doc_id: String,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|e| e.to_string())?;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;
    db.delete_content_source(&ehl_doc_id).map_err(|e| e.to_string())
}

/// Start watching the database file for changes
fn start_db_watcher(app_handle: AppHandle, db_path: String) {
    std::thread::spawn(move || {
        let path = Path::new(&db_path);
        let parent_dir = path.parent().unwrap_or(path);
        
        let (tx, rx) = std::sync::mpsc::channel();
        
        let config = Config::default()
            .with_poll_interval(Duration::from_secs(1));
        
        let mut watcher: RecommendedWatcher = match Watcher::new(tx, config) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Failed to create file watcher: {}", e);
                return;
            }
        };
        
        if let Err(e) = watcher.watch(parent_dir, RecursiveMode::NonRecursive) {
            eprintln!("Failed to watch database directory: {}", e);
            return;
        }
        
        println!("✓ Watching database for changes: {}", db_path);
        
        for res in rx {
            match res {
                Ok(event) => {
                    // Check if this event is for our database file
                    let is_db_event = event.paths.iter().any(|p| {
                        p.file_name()
                            .map(|n| n.to_string_lossy().contains("content.db"))
                            .unwrap_or(false)
                    });
                    
                    if is_db_event {
                        match event.kind {
                            EventKind::Modify(_) | EventKind::Create(_) => {
                                if let Err(e) = app_handle.emit("db-changed", ()) {
                                    eprintln!("Failed to emit db-changed event: {}", e);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => eprintln!("Watch error: {}", e),
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize database
    let db_path = db::get_default_db_path();
    println!("Attempting to connect to database at: {}", db_path);
    println!("Path exists: {}", std::path::Path::new(&db_path).exists());
    
    let app_state = match ViewerDb::open(&db_path) {
        Ok(db) => {
            println!("✓ Successfully connected to database at: {}", db_path);
            match db.get_source_count() {
                Ok(count) => println!("✓ Database has {} content sources", count),
                Err(e) => println!("✗ Failed to query database: {}", e),
            }
            AppState::with_db(db, db_path.clone())
        }
        Err(e) => {
            eprintln!("✗ Failed to connect to database at {}: {}", db_path, e);
            AppState::new()
        }
    };

    let db_path_for_watcher = db_path.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(move |app| {
            // Start the database file watcher
            start_db_watcher(app.handle().clone(), db_path_for_watcher);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_content_sources,
            get_content_detail,
            get_stats,
            delete_content_source
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
