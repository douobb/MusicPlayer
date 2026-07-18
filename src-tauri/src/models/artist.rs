use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtistCredit {
    pub artist_id: i64,
    pub name: String,
    pub position: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtistRole {
    Performer,
    OriginalPerformer,
}

impl ArtistRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Performer => "performer",
            Self::OriginalPerformer => "original_performer",
        }
    }
}
