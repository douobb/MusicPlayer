use std::path::Path;

use lofty::config::WriteOptions;
use lofty::file::TaggedFileExt;
use lofty::tag::{Accessor, ItemKey, ItemValue, Tag, TagExt, TagItem};

use crate::error::AppError;

fn write_artist_values(tag: &mut Tag, performers: &[String], originals: &[String]) {
    tag.remove_key(ItemKey::TrackArtist);
    tag.remove_key(ItemKey::TrackArtists);
    tag.remove_key(ItemKey::OriginalArtist);

    let display = performers.join("; ");
    if !display.is_empty() {
        tag.set_artist(display);
    }
    if !performers.is_empty() {
        tag.insert_unchecked(TagItem::new(
            ItemKey::TrackArtists,
            ItemValue::Text(performers.join("\0")),
        ));
    }
    if !originals.is_empty() {
        tag.insert_unchecked(TagItem::new(
            ItemKey::OriginalArtist,
            ItemValue::Text(originals.join("\0")),
        ));
    }
}

pub fn write_metadata(
    file_path: &str,
    title: Option<&str>,
    performers: Option<&[String]>,
    original_performers: Option<&[String]>,
) -> Result<(), AppError> {
    let path = Path::new(file_path);
    let mut tagged_file = lofty::read_from_path(path)
        .map_err(|e| AppError::MetadataWrite(format!("Failed to read file: {e}")))?;

    let has_primary = tagged_file.primary_tag_mut().is_some();
    let has_any = has_primary || tagged_file.first_tag_mut().is_some();
    let mut new_tag;
    let tag = if has_any {
        #[allow(clippy::unwrap_used)]
        if has_primary {
            tagged_file.primary_tag_mut().unwrap()
        } else {
            tagged_file.first_tag_mut().unwrap()
        }
    } else {
        new_tag = Tag::new(tagged_file.primary_tag_type());
        &mut new_tag
    };

    if let Some(value) = title {
        tag.set_title(value.to_string());
    }
    if let Some(values) = performers {
        write_artist_values(tag, values, original_performers.unwrap_or(&[]));
    } else if let Some(values) = original_performers {
        tag.remove_key(ItemKey::OriginalArtist);
        if !values.is_empty() {
            tag.insert_unchecked(TagItem::new(
                ItemKey::OriginalArtist,
                ItemValue::Text(values.join("\0")),
            ));
        }
    }
    tag.save_to_path(path, WriteOptions::default())
        .map_err(|e| AppError::MetadataWrite(format!("Failed to save tag: {e}")))?;
    Ok(())
}
