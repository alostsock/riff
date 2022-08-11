use crate::utils;
use id3::{TagLike, Version};
use std::{collections::HashMap, fs::File, path::Path};
use symphonia::core::{
    formats::FormatOptions, io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
};

#[derive(serde::Serialize)]
pub enum TagFormat {
    Id3v24,
    Id3v23,
    Id3v22,
    Id3v1,
    Mp4,
}

#[derive(serde::Serialize)]
pub struct Track {
    pub id: String,
    pub path: String,
    pub tag_format: Option<TagFormat>,

    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub disc: Option<u32>,
    pub track: Option<u32>,
    pub duration: Option<u32>,
    pub image: Option<usize>,
}

impl Track {
    pub fn from_path(path: &Path) -> Result<Self, &Path> {
        let mut err_messages = vec![];

        match id3::Tag::read_from_path(path) {
            Ok(tag) => {
                let path_str = path.to_string_lossy().to_string();
                return Ok(Self {
                    id: utils::hash(&path_str),
                    path: path_str,
                    tag_format: Some(match tag.version() {
                        Version::Id3v24 => TagFormat::Id3v24,
                        Version::Id3v23 => TagFormat::Id3v23,
                        Version::Id3v22 => TagFormat::Id3v22,
                    }),
                    title: tag.title().map(|s| s.to_string()),
                    artist: tag.artist().map(|s| s.to_string()),
                    album: tag.album().map(|s| s.to_string()),
                    album_artist: tag.album_artist().map(|s| s.to_string()),
                    disc: tag.disc(),
                    track: tag.track(),
                    duration: tag.duration().or_else(|| read_track_duration(path)),
                    image: None,
                });
            }
            Err(e) => err_messages.push(format!("id3v2 ({})", e.description)),
        };

        match mp4ameta::Tag::read_from_path(path) {
            Ok(tag) => {
                let path_str = path.to_string_lossy().to_string();
                return Ok(Self {
                    id: utils::hash(&path_str),
                    path: path_str,
                    tag_format: Some(TagFormat::Mp4),
                    title: tag.title().map(|s| s.to_string()),
                    artist: tag.artist().map(|s| s.to_string()),
                    album: tag.album().map(|s| s.to_string()),
                    album_artist: tag.album_artist().map(|s| s.to_string()),
                    disc: tag.disc_number().map(|d| d as u32),
                    track: tag.track_number().map(|d| d as u32),
                    duration: tag
                        .duration()
                        .map(|d| d.as_millis().try_into().unwrap())
                        .or_else(|| read_track_duration(path)),
                    image: None,
                });
            }
            Err(e) => err_messages.push(format!("mp4 ({})", e.description)),
        }

        println!("tried parsing tags: {}", err_messages.join(", "));
        Err(path)
    }

    pub fn without_tags(path: &Path) -> Self {
        let path = path.to_string_lossy().to_string();
        Self {
            id: utils::hash(&path),
            path,
            tag_format: None,
            title: None,
            artist: None,
            album: None,
            album_artist: None,
            disc: None,
            track: None,
            duration: None,
            image: None,
        }
    }
}

#[derive(serde::Serialize)]
pub struct Image {
    pub id: String,
    pub path: String,
    pub thumb: Option<String>,
}

/// Keep track of physical track and image files by hashing their path
#[derive(Default, serde::Serialize)]
pub struct Media {
    pub tracks: HashMap<String, Track>,
    pub images: HashMap<String, Image>,
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

    let metadata_opts: MetadataOptions = Default::default();

    match symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts) {
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
