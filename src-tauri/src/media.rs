use crate::utils;
use id3::{TagLike, Version};
use std::{collections::HashMap, fs::File, path::Path};
use symphonia::core::{formats::FormatOptions, io::MediaSourceStream, probe::Hint};

#[derive(Clone, Copy, serde::Serialize)]
pub enum TagType {
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
pub struct Track {
    pub id: String,
    pub path: String,
    pub tag_format: Option<String>,

    pub title: Option<String>,
    // only support one artist at the moment
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
                let tag_type = match tag.version() {
                    Version::Id3v24 => TagType::Id3v24,
                    Version::Id3v23 => TagType::Id3v23,
                    Version::Id3v22 => TagType::Id3v22,
                };

                return Ok(Self {
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
                    image: None,
                });
            }
            Err(e) => err_messages.push(format!("id3v2 ({})", e.description)),
        };

        match mp4ameta::Tag::read_from_path(path) {
            Ok(tag) => {
                let path_str = path.to_string_lossy().to_string();
                let tag_type = TagType::Mp4;
                return Ok(Self {
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
