#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use riff::{
    media::{Image, Media, Track},
    utils,
};
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

// Use Mutex for interior mutability
#[derive(Default)]
pub struct WatchState(Mutex<Option<WatchData>>);

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

#[tauri::command]
fn list_media_files(directory: String) -> Media {
    let audio_extensions = HashSet::from(["mp3", "m4a"]);
    let image_extensions = HashSet::from(["jpg", "jpeg", "png"]);

    let mut media = Media::default();

    for entry in WalkDir::new(directory).into_iter().flatten() {
        let path = entry.path();
        let path_str = path.to_string_lossy().to_string();
        let extension = path.extension().and_then(|ext| ext.to_str());
        if let Some(ext) = extension {
            if audio_extensions.contains(&ext) {
                let track = match Track::from_path(path) {
                    Ok(track) => track,
                    Err(path) => {
                        println!("error parsing tags for '{:?}'", path.to_str());
                        Track::without_tags(path)
                    }
                };
                media.tracks.insert(track.id.clone(), track);
            } else if image_extensions.contains(&ext) {
                let image = Image {
                    id: utils::hash(&path_str),
                    path: path_str,
                    thumb: None,
                };
                media.images.insert(image.id.clone(), image);
            }
        }
    }

    media
}
