use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TagSummary {
    pub id: i64,
    pub name: String,
    pub track_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TagStatistics {
    pub tag_count: i64,
    pub tagged_track_count: i64,
    pub untagged_track_count: i64,
    pub assignment_count: i64,
    pub average_tags_per_tagged_track: f64,
    pub most_used_tag: Option<TagSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TagAssignment {
    pub id: i64,
    pub name: String,
    pub assigned_count: i64,
}
