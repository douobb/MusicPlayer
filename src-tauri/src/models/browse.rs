use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtistSummary {
    pub id: i64,
    pub name: String,
    pub track_count: i64,
    pub performer_track_count: i64,
    pub original_track_count: i64,
}
