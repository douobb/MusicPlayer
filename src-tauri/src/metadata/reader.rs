use std::fs;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use lofty::config::{ParseOptions, ParsingMode};
use lofty::file::{AudioFile, TaggedFile, TaggedFileExt};
use lofty::picture::PictureType;
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey};

use crate::error::AppError;
use crate::models::artist::ArtistCredit;
use crate::models::track::{Track, TrackDetails};

/// Read a tagged file in Relaxed mode: invalid tag items (e.g. `ID3v2` timestamp
/// frames with non-digit characters, common in Japanese rips) are skipped
/// instead of failing the whole read like the default `BestAttempt` mode does.
fn read_tagged_file(path: &Path) -> lofty::error::Result<TaggedFile> {
    let options = ParseOptions::new().parsing_mode(ParsingMode::Relaxed);
    Probe::open(path)?.options(options).read()
}

#[allow(clippy::cast_possible_wrap)]
fn credits_from_names(names: Vec<String>) -> Vec<ArtistCredit> {
    names
        .into_iter()
        .enumerate()
        .filter_map(|(position, name)| {
            let name = name.trim().to_string();
            if name.is_empty() {
                return None;
            }
            Some(ArtistCredit {
                artist_id: 0,
                name,
                position: position as i64,
            })
        })
        .collect()
}

fn read_artist_names(tag: &lofty::tag::Tag, key: ItemKey) -> Vec<String> {
    tag.get_strings(key)
        .flat_map(|value| value.split('\0'))
        .map(ToString::to_string)
        .collect()
}
pub fn read_metadata(file_path: &str) -> Result<Track, AppError> {
    let path = Path::new(file_path);

    let fallback_title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown Title")
        .to_string();

    #[allow(clippy::cast_possible_wrap)]
    let file_size_bytes = fs::metadata(path).map_or(0, |m| m.len() as i64);

    match read_tagged_file(path) {
        Ok(tagged_file) => {
            let properties = tagged_file.properties();
            let duration_secs = properties.duration().as_secs_f64();

            let tag = tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag());

            let (title, performers, original_performers) = match tag {
                Some(tag) => {
                    let mut performers = read_artist_names(tag, ItemKey::TrackArtists);
                    if performers.is_empty() {
                        performers = read_artist_names(tag, ItemKey::TrackArtist);
                    }
                    if performers.is_empty() {
                        if let Some(value) = tag.artist() {
                            performers.push(value.to_string());
                        }
                    }
                    if performers.is_empty() {
                        performers.push("Unknown Artist".to_string());
                    }
                    (
                        tag.title()
                            .map_or(fallback_title, |value| value.to_string()),
                        credits_from_names(performers),
                        credits_from_names(read_artist_names(tag, ItemKey::OriginalArtist)),
                    )
                }
                None => (
                    fallback_title,
                    credits_from_names(vec!["Unknown Artist".to_string()]),
                    Vec::new(),
                ),
            };
            Ok(Track {
                id: 0,
                file_path: file_path.to_string(),
                title,
                performers,
                original_performers,
                duration_secs,
                cover_art: None,
                cover_art_path: None,
                file_size_bytes,
                play_count: 0,
                last_played_at: None,
            })
        }
        Err(e) => {
            // Last-resort fallback for tags even Relaxed mode cannot parse:
            // skip tag reading and use only audio properties + filename.
            eprintln!(
                "[musicplayer] Tag parsing failed for {file_path}: {e}; \
                 retrying without tags"
            );

            let options = ParseOptions::new().read_tags(false);
            let tagged_file = Probe::open(path)
                .and_then(|probe| probe.options(options).read())
                .map_err(|e2| AppError::Metadata(e2.to_string()))?;

            let duration_secs = tagged_file.properties().duration().as_secs_f64();

            Ok(Track {
                id: 0,
                file_path: file_path.to_string(),
                title: fallback_title,
                performers: credits_from_names(vec!["Unknown Artist".to_string()]),
                original_performers: Vec::new(),
                duration_secs,
                cover_art: None,
                cover_art_path: None,
                file_size_bytes,
                play_count: 0,
                last_played_at: None,
            })
        }
    }
}

pub fn read_track_details(file_path: &str, track: &Track) -> Result<TrackDetails, AppError> {
    let path = Path::new(file_path);

    let tagged_file = read_tagged_file(path).map_err(|e| AppError::Metadata(e.to_string()))?;

    let properties = tagged_file.properties();

    let bitrate_kbps = properties.overall_bitrate();
    let sample_rate_hz = properties.sample_rate();
    let channels = properties.channels();
    let bits_per_sample = properties.bit_depth();

    let format = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_uppercase();

    #[allow(clippy::cast_possible_wrap)]
    let file_size_bytes = fs::metadata(path).map_or(track.file_size_bytes, |m| m.len() as i64);

    Ok(TrackDetails {
        id: track.id,
        file_path: track.file_path.clone(),
        title: track.title.clone(),
        performers: track.performers.clone(),
        original_performers: track.original_performers.clone(),
        duration_secs: track.duration_secs,
        file_size_bytes,
        bitrate_kbps,
        sample_rate_hz,
        channels,
        format,
        bits_per_sample,
    })
}

/// Extract raw cover art bytes and MIME type from an audio file.
pub fn extract_cover_art_bytes(file_path: &str) -> Option<(Vec<u8>, String)> {
    let tagged_file = read_tagged_file(Path::new(file_path)).ok()?;
    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag())?;

    let picture = tag
        .pictures()
        .iter()
        .find(|p| p.pic_type() == PictureType::CoverFront)
        .or_else(|| tag.pictures().first())?;

    let mime = match picture.mime_type() {
        Some(mime) => mime.as_str().to_string(),
        None => "image/jpeg".to_string(),
    };

    Some((picture.data().to_vec(), mime))
}

/// Save cover art bytes to the filesystem and return the file path.
pub fn save_cover_art(
    app_data_dir: &Path,
    track_id: i64,
    data: &[u8],
    mime: &str,
) -> Result<String, AppError> {
    let covers_dir = app_data_dir.join("covers");
    fs::create_dir_all(&covers_dir)?;

    let ext = match mime {
        "image/png" => "png",
        "image/gif" => "gif",
        "image/bmp" => "bmp",
        "image/webp" => "webp",
        _ => "jpg",
    };

    let file_path = covers_dir.join(format!("{track_id}.{ext}"));
    fs::write(&file_path, data)?;

    Ok(file_path.to_string_lossy().to_string())
}

/// Remove a cover art file from disk. A missing file is fine (already cleaned
/// up); any other failure (e.g. permissions) is logged so orphaned covers are
/// not silently left behind.
pub fn remove_cover_art_file(cover_art_path: &str) {
    if let Err(e) = fs::remove_file(cover_art_path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            eprintln!("[musicplayer] failed to remove cover art {cover_art_path}: {e}");
        }
    }
}

/// Read a cover art file from disk and return as a base64 data URI.
pub fn read_cover_art_from_file(cover_art_path: &str) -> Option<String> {
    let path = Path::new(cover_art_path);
    let data = fs::read(path).ok()?;

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
    let mime = match ext {
        "png" => "image/png",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        _ => "image/jpeg",
    };

    let b64 = STANDARD.encode(&data);
    Some(format!("data:{mime};base64,{b64}"))
}
