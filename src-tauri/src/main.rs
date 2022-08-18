#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use riff::db::{Db, Media};
use std::{
    sync::{mpsc::channel, Mutex},
    thread,
    time::Duration,
};

fn main() {
    tauri::Builder::default()
        .manage(WatchState::default())
        .manage(DbState(Mutex::new(Db::create())))
        .invoke_handler(tauri::generate_handler![watch, populate_db])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Use Mutex for interior mutability
#[derive(Default)]
struct WatchState(Mutex<Option<WatchData>>);

struct WatchData {
    path: String,
    watcher: RecommendedWatcher,
}

#[derive(Clone, serde::Serialize)]
struct FilesystemEvent {
    event_type: String,
    path: Option<String>,
}

impl FilesystemEvent {
    fn from_debounced_event(event: DebouncedEvent) -> Self {
        use DebouncedEvent::*;
        let (event_type, path) = match event {
            NoticeWrite(path) => ("notice-write", Some(path)),
            NoticeRemove(path) => ("notice-remove", Some(path)),
            Create(path) => ("create", Some(path)),
            Write(path) => ("write", Some(path)),
            Chmod(path) => ("chmod", Some(path)),
            Remove(path) => ("remove", Some(path)),
            Rename(_, path) => ("rename", Some(path)),
            Rescan => ("rescan", None),
            Error(err, path) => {
                println!("error watching path '{:?}': {:?}", path, err);
                ("error", path)
            }
        };

        Self {
            event_type: event_type.to_string(),
            path: path.map(|p| p.to_string_lossy().to_string()),
        }
    }
}

/// Watch the filesystem for changes. For simplicity, only watch one path at a time.
#[tauri::command]
fn watch(window: tauri::Window, state: tauri::State<WatchState>, path: String) {
    if let Some(watch_data) = &mut *state.0.lock().unwrap() {
        let WatchData { path, watcher } = watch_data;
        watcher.unwatch(path).expect("error while unwatching path");
    }

    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_secs(2)).unwrap();

    println!("watching {}", path);
    watcher
        .watch(&path, RecursiveMode::Recursive)
        .expect("error while watching path");

    *state.0.lock().unwrap() = Some(WatchData {
        path: path.clone(),
        watcher,
    });

    thread::spawn(move || loop {
        match rx.recv() {
            Ok(event) => {
                let filesystem_event = FilesystemEvent::from_debounced_event(event);
                window.emit("filesystem-event", filesystem_event).unwrap();
            }
            Err(_) => {
                println!("unwatched {:?}", path);
                break;
            }
        }
    });
}

struct DbState(Mutex<Db>);

#[tauri::command]
fn populate_db(state: tauri::State<DbState>, directory: String) -> Media {
    let db = &mut *state.0.lock().unwrap();
    db.populate(&directory)
}
