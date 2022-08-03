#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::HashSet,
    sync::{mpsc::channel, Mutex},
    thread,
    time::Duration,
};
use walkdir::WalkDir;

fn main() {
    tauri::Builder::default()
        .manage(WatchState::default())
        .invoke_handler(tauri::generate_handler![watch, list_media_files])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

struct WatchData {
    path: String,
    watcher: RecommendedWatcher,
}

// Use Mutex for interior mutability
#[derive(Default)]
struct WatchState(Mutex<Option<WatchData>>);

#[derive(Clone, serde::Serialize)]
struct FilesystemEvent {
    event_type: String,
    path: Option<String>,
}

/// Watch the filesystem for changes. For simplicity, only watch one path at a time.
#[tauri::command]
fn watch(window: tauri::Window, state: tauri::State<WatchState>, path: String) {
    if let Some(watch_data) = &mut *state.0.lock().unwrap() {
        let WatchData { path, watcher } = watch_data;
        watcher.unwatch(path).expect("error while unwatching path");
    }

    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = watcher(tx, Duration::from_secs(2)).unwrap();

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
                let (event_type, path) = match event {
                    DebouncedEvent::NoticeWrite(path) => ("notice-write", Some(path)),
                    DebouncedEvent::NoticeRemove(path) => ("notice-remove", Some(path)),
                    DebouncedEvent::Create(path) => ("create", Some(path)),
                    DebouncedEvent::Write(path) => ("write", Some(path)),
                    DebouncedEvent::Chmod(path) => ("chmod", Some(path)),
                    DebouncedEvent::Remove(path) => ("remove", Some(path)),
                    DebouncedEvent::Rename(_, path) => ("rename", Some(path)),
                    DebouncedEvent::Rescan => ("rescan", None),
                    DebouncedEvent::Error(err, path) => {
                        println!("error watching path '{:?}': {:?}", path, err);
                        ("error", path)
                    }
                };

                let filesystem_event = FilesystemEvent {
                    event_type: event_type.to_string(),
                    path: path.map(|p| {
                        p.to_str()
                            .expect("path likely contains invalid unicode")
                            .to_string()
                    }),
                };

                window.emit("filesystem-event", filesystem_event).unwrap();
            }
            Err(_) => {
                println!("unwatched {:?}", path);
                break;
            }
        }
    });
}

#[derive(serde::Serialize)]
struct Media {
    tracks: Vec<String>,
    images: Vec<String>,
}

#[tauri::command]
fn list_media_files(path: String) -> Media {
    let audio_extensions = HashSet::from(["mp3", "m4a"]);
    let image_extensions = HashSet::from(["jpg", "jpeg", "png"]);

    let mut tracks: Vec<String> = vec![];
    let mut images: Vec<String> = vec![];

    for entry in WalkDir::new(path).into_iter().flatten() {
        let path = entry
            .path()
            .to_str()
            .expect("path likely contains invalid unicode")
            .to_string();
        let extension = entry.path().extension().and_then(|ext| ext.to_str());

        if let Some(ext) = extension {
            if audio_extensions.contains(&ext) {
                tracks.push(path);
            } else if image_extensions.contains(&ext) {
                images.push(path);
            }
        }
    }

    Media { tracks, images }
}
