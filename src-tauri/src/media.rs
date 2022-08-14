use crate::utils;
use id3::{TagLike, Version};
use std::{collections::HashMap, collections::HashSet, fs::File, path::Path};
use symphonia::core::{formats::FormatOptions, io::MediaSourceStream, probe::Hint};
use walkdir::WalkDir;

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
        match *self {
            Self::Id3v24 => "ID3v2.4".to_string(),
            Self::Id3v23 => "ID3v2.3".to_string(),
            Self::Id3v22 => "ID3v2.2".to_string(),
            Self::Id3v1 => "ID3v1".to_string(),
            Self::Mp4 => "MP4".to_string(),
        }
    }
}

#[derive(serde::Serialize)]
struct Track {
    id: String,
    path: String,
    tag_format: Option<String>,

    title: Option<String>,
    // only support one artist at the moment
    artist: Option<String>,
    album: Option<String>,
    album_artist: Option<String>,
    disc: Option<u32>,
    track: Option<u32>,
    duration: Option<u32>,
    image_ids: Vec<String>,
}

impl Track {
    fn try_from_path(path: &Path) -> Result<Self, &Path> {
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

    fn without_tags(path: &Path) -> Self {
        let path_str = path.to_string_lossy().to_string();
        Self {
            id: utils::hash(&path_str),
            path: path_str,
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
}

impl Image {
    fn from_path(path: &Path) -> Self {
        let path_str = path.to_string_lossy().to_string();
        Self {
            id: utils::hash(&path_str),
            path: path_str,
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
    root: String,
    /// All tracks by id
    tracks: HashMap<String, Track>,
    /// All images by id
    images: HashMap<String, Image>,
    /// Hierarchical representation of Artists -> Albums -> Tracks
    artists: HashMap<String, Artist>,
    /// All directory paths which contain track or image content, relative to the root directory
    directories: HashMap<String, DirectoryContent>,
}

impl Media {
    pub fn from_directory(root: String) -> Self {
        let Media {
            mut tracks,
            mut images,
            mut artists,
            mut directories,
            ..
        } = Default::default();

        let audio_extensions = HashSet::from(["mp3", "m4a"]);
        let image_extensions = HashSet::from(["jpg", "jpeg", "png"]);
        for entry in WalkDir::new(root.clone()).into_iter().flatten() {
            let path = entry.path();
            let relative_parent_path = Self::get_relative_parent_path(path, &root);

            if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                if audio_extensions.contains(&ext) {
                    let track = Track::try_from_path(path).unwrap_or_else(|path| {
                        println!("error parsing tags for '{:?}'", path.to_str());
                        Track::without_tags(path)
                    });

                    directories
                        .entry(relative_parent_path)
                        .or_insert_with(Default::default)
                        .track_ids
                        .push(track.id.clone());

                    tracks.insert(track.id.clone(), track);
                } else if image_extensions.contains(&ext) {
                    let image = Image::from_path(path);

                    directories
                        .entry(relative_parent_path)
                        .or_insert_with(Default::default)
                        .image_ids
                        .push(image.id.clone());

                    images.insert(image.id.clone(), image);
                } else {
                    println!("unhandled file type {:?}", path);
                }
            }
        }

        // Organize track data after processing all files
        for track_id in directories
            .values()
            .flat_map(|dir_content| dir_content.track_ids.clone())
        {
            let track = tracks.get_mut(&track_id).unwrap();

            // Try to get images for the track
            // TODO: We only look at the current directory. Maybe look one directory up as well?
            let relative_parent_path =
                Self::get_relative_parent_path(Path::new(&track.path), &root);
            let associated_images = directories
                .get(&relative_parent_path)
                .map(|directory_content| directory_content.image_ids.clone())
                .unwrap_or_default();

            if let Some(artist_name) = &track.artist {
                let artist = artists
                    .entry(artist_name.clone())
                    .or_insert_with(|| Artist {
                        name: artist_name.clone(),
                        ..Default::default()
                    });

                if let Some(album_name) = &track.album {
                    artist
                        .albums
                        .entry(album_name.clone())
                        .or_insert_with(|| Album {
                            name: album_name.clone(),
                            image_ids: associated_images.clone(),
                            ..Default::default()
                        })
                        .track_ids
                        .push(track_id.clone());
                } else {
                    artist.track_ids.push(track_id.clone());
                    track.image_ids = associated_images.clone()
                }
            }
        }

        Media {
            root,
            tracks,
            images,
            artists,
            directories,
        }
    }

    fn get_relative_parent_path(path: &Path, root: &str) -> String {
        path.parent()
            .unwrap()
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    }
}
