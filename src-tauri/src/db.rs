use crate::utils;
use id3::{TagLike, Version};
use rusqlite::{params, Connection};
use std::{collections::HashMap, collections::HashSet, fs::File, path::Path};
use symphonia::core::{formats::FormatOptions, io::MediaSourceStream, probe::Hint};
use walkdir::WalkDir;

macro_rules! replace {
    ($i:ident, $l:literal) => {
        $l
    };
}

macro_rules! insert {
    ($tx:expr; table: $table:literal; $struct:expr => { $first_field:ident $(, $field:ident )* $(,)?}) => {
        {
            let mut statement = $tx.prepare_cached(concat!(
                "INSERT INTO ",
                $table,
                " (",
                stringify!($first_field) $(, concat!(", ", stringify!($field)) )*,
                ") ",
                "VALUES (",
                replace!($first_field, "?") $(, replace!($field, ", ?") )*,
                ")"
            )).unwrap();
            statement.execute(params![ $struct.$first_field $(, $struct.$field )* ]).unwrap();
        }
    }
}

pub struct Db {
    connection: Connection,
}

impl Db {
    pub fn create() -> Self {
        let db = Self {
            connection: Connection::open_in_memory().expect("couldn't open sqlite db"),
        };
        db.create_tables();
        db
    }

    fn create_tables(&self) {
        self.connection
            .execute_batch(
                "BEGIN;

                CREATE TABLE track (
                    id TEXT PRIMARY KEY,
                    path TEXT UNIQUE,
                    relative_parent_path TEXT NOT NULL,
                    tag_format TEXT,
                    title TEXT,
                    artist TEXT,
                    album TEXT,
                    album_artist TEXT,
                    disc INT,
                    track INT,
                    duration INT
                ) STRICT;

                CREATE TABLE image (
                    id TEXT PRIMARY KEY,
                    path TEXT UNIQUE,
                    relative_parent_path TEXT NOT NULL
                ) STRICT;

                COMMIT;",
            )
            .expect("failed to create tables");
    }

    pub fn populate(&mut self, root: &str) -> Media {
        let media = Media::from_directory(root);

        let tx = self.connection.transaction().unwrap();

        for track in media.tracks.values() {
            insert!(tx; table: "track"; track => {
                id, path, relative_parent_path, tag_format, title, artist, album, album_artist, disc, track, duration
            });
        }

        for image in media.images.values() {
            insert!(tx; table: "image"; image => { id, path, relative_parent_path });
        }

        tx.commit().unwrap();

        media
    }
}

fn get_relative_parent_path(path: &Path, root: &str) -> String {
    path.parent()
        .unwrap()
        .strip_prefix(root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}

#[derive(Clone, Copy, serde::Serialize)]
#[allow(dead_code)]
enum TagType {
    Id3v24,
    Id3v23,
    Id3v22,
    Id3v1,
    Mp4,
}

impl TagType {
    fn format_str(&self, s: Option<&str>) -> Option<String> {
        match *self {
            // I think foobar2000 displays "\0" as "/" in text fields
            Self::Id3v23 => s.map(|s| s.to_string().replace('\0', "/")),
            _ => s.map(String::from),
        }
    }

    fn name(&self) -> String {
        String::from(match *self {
            Self::Id3v24 => "ID3v2.4",
            Self::Id3v23 => "ID3v2.3",
            Self::Id3v22 => "ID3v2.2",
            Self::Id3v1 => "ID3v1",
            Self::Mp4 => "MP4",
        })
    }
}

#[derive(serde::Serialize)]
struct Track {
    id: String,
    path: String,
    relative_parent_path: String,
    tag_format: Option<String>,
    title: Option<String>,
    artist: Option<String>, // only support one artist at the moment
    album: Option<String>,
    album_artist: Option<String>,
    disc: Option<u32>,
    track: Option<u32>,
    duration: Option<u32>,
    image_ids: Vec<String>,
}

impl<'a> Track {
    fn try_from_path(path: &'a Path, root: &str) -> Result<Self, &'a Path> {
        if let Ok(tag) = id3::Tag::read_from_path(path) {
            let path_str = path.to_string_lossy().to_string();
            let tag_type = match tag.version() {
                Version::Id3v24 => TagType::Id3v24,
                Version::Id3v23 => TagType::Id3v23,
                Version::Id3v22 => TagType::Id3v22,
            };
            Ok(Self {
                id: utils::hash(&path_str),
                path: path_str,
                relative_parent_path: get_relative_parent_path(path, root),
                tag_format: Some(tag_type.name()),
                title: tag_type.format_str(tag.title()),
                artist: tag_type.format_str(tag.artist()),
                album: tag_type.format_str(tag.album()),
                album_artist: tag_type.format_str(tag.album_artist()),
                disc: tag.disc(),
                track: tag.track(),
                duration: tag.duration().or_else(|| read_track_duration(path)),
                image_ids: Default::default(),
            })
        } else if let Ok(tag) = mp4ameta::Tag::read_from_path(path) {
            let path_str = path.to_string_lossy().to_string();
            let tag_type = TagType::Mp4;
            Ok(Self {
                id: utils::hash(&path_str),
                path: path_str,
                relative_parent_path: get_relative_parent_path(path, root),
                tag_format: Some(tag_type.name()),
                title: tag_type.format_str(tag.title()),
                artist: tag_type.format_str(tag.artist()),
                album: tag_type.format_str(tag.album()),
                album_artist: tag_type.format_str(tag.album_artist()),
                disc: tag.disc_number().map(u32::from),
                track: tag.track_number().map(u32::from),
                duration: tag
                    .duration()
                    .map(|d| d.as_millis().try_into().unwrap())
                    .or_else(|| read_track_duration(path)),
                image_ids: Default::default(),
            })
        } else {
            Err(path)
        }
    }

    fn without_tags(path: &Path, root: &str) -> Self {
        let path_str = path.to_string_lossy().to_string();
        Self {
            id: utils::hash(&path_str),
            path: path_str,
            relative_parent_path: get_relative_parent_path(path, root),
            tag_format: None,
            title: None,
            artist: None,
            album: None,
            album_artist: None,
            disc: None,
            track: None,
            duration: None,
            image_ids: Default::default(),
        }
    }
}

/// Reads an audio file for its track duration, in milliseconds
fn read_track_duration(path: &Path) -> Option<u32> {
    let mut hint = Hint::new();
    if let Some(ext) = path.extension() {
        hint.with_extension(ext.to_str().unwrap());
    }

    let mss = MediaSourceStream::new(Box::new(File::open(path).unwrap()), Default::default());

    let format_opts: FormatOptions = FormatOptions {
        seek_index_fill_rate: 10,
        ..Default::default()
    };

    match symphonia::default::get_probe().format(&hint, mss, &format_opts, &Default::default()) {
        Ok(probed) => {
            let reader = probed.format;
            let track = reader.default_track().unwrap();
            let codec_params = &track.codec_params;
            let sample_rate: u32 = codec_params.sample_rate?;
            let frames: u64 = codec_params.n_frames?;
            let delay: u32 = track.codec_params.delay.unwrap_or_default();
            let padding: u32 = track.codec_params.padding.unwrap_or_default();

            let duration_seconds: f64 =
                (frames - delay as u64 - padding as u64) as f64 / sample_rate as f64;

            Some((duration_seconds * 1000.0) as u32)
        }
        Err(err) => {
            use symphonia::core::errors::Error::*;
            let msg = match err {
                IoError(_) => "IO error occured while reading stream",
                DecodeError(_) => "stream contained malformed data",
                SeekError(_) => "stream could not be seeked",
                Unsupported(_) => "unsupported container or codec",
                LimitError(_) => "decode limit reached",
                ResetRequired => "decoder requires a reset",
            };
            println!("error while reading track metadata: {}", msg);

            None
        }
    }
}

#[derive(serde::Serialize)]
struct Image {
    id: String,
    path: String,
    relative_parent_path: String,
}

impl Image {
    fn from_path(path: &Path, root: &str) -> Self {
        let path_str = path.to_string_lossy().to_string();
        Self {
            id: utils::hash(&path_str),
            path: path_str,
            relative_parent_path: get_relative_parent_path(path, root),
        }
    }
}

#[derive(Default, serde::Serialize)]
struct Album {
    name: String,
    track_ids: Vec<String>,
    /// Associated images should come from
    /// 1) TODO: the first track added to the album, and
    /// 2) the closest image in the directory tree
    image_ids: Vec<String>,
}

#[derive(Default, serde::Serialize)]
struct Artist {
    name: String,
    albums: HashMap<String, Album>,
    /// Tracks with no album tag
    track_ids: Vec<String>,
}

#[derive(Default, serde::Serialize)]
struct DirectoryContent {
    image_ids: Vec<String>,
    track_ids: Vec<String>,
}

#[derive(Default, serde::Serialize)]
pub struct Media {
    /// All tracks by id
    tracks: HashMap<String, Track>,
    /// All images by id
    images: HashMap<String, Image>,
}

impl Media {
    pub fn from_directory(root: &str) -> Self {
        let Media {
            mut tracks,
            mut images,
        } = Default::default();

        let audio_extensions = HashSet::from(["mp3", "m4a"]);
        let image_extensions = HashSet::from(["jpg", "jpeg", "png"]);
        for entry in WalkDir::new(root).into_iter().flatten() {
            let path = entry.path();

            if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                if audio_extensions.contains(&ext) {
                    let track = Track::try_from_path(path, root).unwrap_or_else(|path| {
                        println!("error parsing tags for '{:?}'", path);
                        Track::without_tags(path, root)
                    });
                    tracks.insert(track.id.clone(), track);
                } else if image_extensions.contains(&ext) {
                    let image = Image::from_path(path, root);
                    images.insert(image.id.clone(), image);
                } else {
                    println!("unhandled file type {:?}", path);
                }
            }
        }

        Media { tracks, images }
    }
}
