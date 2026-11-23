//! Request body and matching logic for `POST /api/v1/filter`.
//!
//! Filters are applied in the order they are given. Missing required fields
//! (including `type` and `field`) are rejected by serde, which axum turns into
//! a `400 Bad Request`.

use serde::Deserialize;

use super::models::SongResponse;

#[derive(Debug, Deserialize)]
pub struct FilterRequest {
    pub filters: Vec<Filter>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Filter {
    Float(FloatFilter),
    Number(NumberFilter),
    String(StringFilter),
}

impl Filter {
    fn matches(&self, song: &SongResponse) -> bool {
        match self {
            Self::Float(filter) => filter.matches(song),
            Self::Number(filter) => filter.matches(song),
            Self::String(filter) => filter.matches(song),
        }
    }
}

/// Returns the hashes of every song that passes all filters.
pub fn apply(songs: &[SongResponse], filters: &[Filter]) -> Vec<String> {
    let mut matched: Vec<&SongResponse> = songs.iter().collect();

    for filter in filters {
        matched.retain(|song| filter.matches(song));
    }

    matched.into_iter().map(|song| song.hash.clone()).collect()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloatFilter {
    pub field: FloatField,
    pub threshold: f64,
    #[serde(alias = "operator")]
    pub operation: Comparison,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FloatField {
    Rating,
    Bpm,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumberFilter {
    pub field: NumberField,
    pub threshold: f64,
    #[serde(alias = "operator")]
    pub operation: Comparison,
}

#[derive(Debug, Deserialize)]
pub enum NumberField {
    #[serde(rename = "DOWNLOADCOUNT")]
    DownloadCount,
    #[serde(rename = "UPVOTES")]
    Upvotes,
    #[serde(rename = "DOWNVOTES")]
    Downvotes,
    #[serde(rename = "TOTALVOTES")]
    TotalVotes,
    #[serde(rename = "UPLOADTIMEUNIX")]
    UploadTimeUnix,
    #[serde(rename = "MAPID")]
    MapId,
    #[serde(rename = "SONGDURATION")]
    SongDuration,
    #[serde(rename = "RANKEDCHANGEUNIX")]
    RankedChangeUnix,
    #[serde(rename = "RANKEDSTATUS")]
    RankedStatus,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Comparison {
    Above,
    Equals,
    Under,
}

impl Comparison {
    fn matches(self, value: f64, threshold: f64) -> bool {
        match self {
            Self::Above => value > threshold,
            Self::Equals => value == threshold,
            Self::Under => value < threshold,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StringFilter {
    pub field: StringField,
    pub equals_to: String,
}

#[derive(Debug, Deserialize)]
pub enum StringField {
    #[serde(rename = "KEY")]
    Key,
    #[serde(rename = "HASH")]
    Hash,
    #[serde(rename = "SONGNAME")]
    SongName,
    #[serde(rename = "SONGAUTHORNAME")]
    SongAuthorName,
    #[serde(rename = "LEVELAUTHORNAME")]
    LevelAuthorName,
    #[serde(rename = "UPLOADERNAME")]
    UploaderName,
}

impl FloatFilter {
    fn matches(&self, song: &SongResponse) -> bool {
        let value = match self.field {
            FloatField::Rating => rating(song),
            FloatField::Bpm => f64::from(song.bpm),
        };

        self.operation.matches(value, self.threshold)
    }
}

impl NumberFilter {
    fn matches(&self, song: &SongResponse) -> bool {
        let value = match self.field {
            NumberField::DownloadCount => f64::from(song.download_count),
            NumberField::Upvotes => f64::from(song.upvotes),
            NumberField::Downvotes => f64::from(song.downvotes),
            NumberField::TotalVotes => f64::from(song.upvotes) + f64::from(song.downvotes),
            NumberField::UploadTimeUnix => f64::from(song.upload_time_unix),
            NumberField::MapId => f64::from(song.map_id),
            NumberField::SongDuration => f64::from(song.song_duration_seconds),
            NumberField::RankedChangeUnix => f64::from(song.ranked_change_unix),
            NumberField::RankedStatus => f64::from(song.ranked_status),
        };

        self.operation.matches(value, self.threshold)
    }
}

impl StringFilter {
    fn matches(&self, song: &SongResponse) -> bool {
        let value = match self.field {
            StringField::Key => &song.key,
            StringField::Hash => &song.hash,
            StringField::SongName => &song.song_name,
            StringField::SongAuthorName => &song.song_author_name,
            StringField::LevelAuthorName => &song.level_author_name,
            StringField::UploaderName => &song.uploader_name,
        };

        value.eq_ignore_ascii_case(&self.equals_to)
    }
}

/// Upvotes divided by total votes, or `0.0` when the song has no votes.
fn rating(song: &SongResponse) -> f64 {
    let upvotes = u64::from(song.upvotes);
    let total = upvotes + u64::from(song.downvotes);
    if total == 0 {
        return 0.0;
    }

    upvotes as f64 / total as f64
}

#[cfg(test)]
mod tests {
    use super::{Filter, FilterRequest, apply};
    use crate::api::SongResponse;
    use crate::maps::SongProto;

    fn song(
        hash: &[u8],
        map_id: u32,
        bpm: f32,
        upvotes: u32,
        downvotes: u32,
        song_name: &str,
    ) -> SongResponse {
        SongResponse::from(&SongProto {
            map_id,
            hash_bytes: hash.to_vec(),
            bpm,
            upvotes,
            downvotes,
            song_name: song_name.to_owned(),
            download_count: 100,
            ..Default::default()
        })
    }

    fn parse(json: &str) -> FilterRequest {
        serde_json::from_str(json).expect("request should deserialize")
    }

    fn filtered(songs: &[SongResponse], json: &str) -> Vec<String> {
        apply(songs, &parse(json).filters)
    }

    fn sample() -> Vec<SongResponse> {
        vec![
            song(&[0x01], 1, 120.0, 90, 10, "Alpha"),
            song(&[0x02], 2, 180.0, 50, 50, "Beta"),
            song(&[0x03], 3, 240.0, 0, 0, "gamma"),
        ]
    }

    #[test]
    fn no_filters_returns_every_hash_in_order() {
        assert_eq!(
            filtered(&sample(), r#"{"filters": []}"#),
            ["01", "02", "03"]
        );
    }

    #[test]
    fn float_rating_filters_use_upvote_ratio() {
        let hashes = filtered(
            &sample(),
            r#"{"filters": [{"type": "float", "field": "RATING", "operation": "ABOVE", "threshold": 0.5}]}"#,
        );
        assert_eq!(hashes, ["01"]);
    }

    #[test]
    fn float_bpm_equality_works() {
        let hashes = filtered(
            &sample(),
            r#"{"filters": [{"type": "float", "field": "BPM", "operation": "EQUALS", "threshold": 180.0}]}"#,
        );
        assert_eq!(hashes, ["02"]);
    }

    #[test]
    fn number_filters_support_derived_and_stored_fields() {
        let hashes = filtered(
            &sample(),
            r#"{"filters": [
                {"type": "number", "field": "TOTALVOTES", "operation": "UNDER", "threshold": 101},
                {"type": "number", "field": "MAPID", "operation": "ABOVE", "threshold": 1}
            ]}"#,
        );
        assert_eq!(hashes, ["02", "03"]);
    }

    #[test]
    fn string_filters_are_case_insensitive() {
        let hashes = filtered(
            &sample(),
            r#"{"filters": [{"type": "string", "field": "SONGNAME", "equalsTo": "GAMMA"}]}"#,
        );
        assert_eq!(hashes, ["03"]);
    }

    #[test]
    fn filters_apply_in_order() {
        let hashes = filtered(
            &sample(),
            r#"{"filters": [
                {"type": "number", "field": "DOWNLOADCOUNT", "operation": "EQUALS", "threshold": 100},
                {"type": "string", "field": "KEY", "equalsTo": "2"}
            ]}"#,
        );
        assert_eq!(hashes, ["02"]);
    }

    #[test]
    fn operator_alias_is_accepted() {
        let hashes = filtered(
            &sample(),
            r#"{"filters": [{"type": "float", "field": "BPM", "operator": "UNDER", "threshold": 150}]}"#,
        );
        assert_eq!(hashes, ["01"]);
    }

    #[test]
    fn missing_type_field_or_parameters_are_rejected() {
        let requests = [
            r#"{"filters": [{"field": "BPM", "operation": "ABOVE", "threshold": 1}]}"#,
            r#"{"filters": [{"type": "float", "operation": "ABOVE", "threshold": 1}]}"#,
            r#"{"filters": [{"type": "float", "field": "BPM", "threshold": 1}]}"#,
            r#"{"filters": [{"type": "string", "field": "KEY"}]}"#,
            r#"{"filters": [{"type": "float", "field": "BPM", "threshold": 1, "operation": "SIDEWAYS"}]}"#,
            r#"{"filters": [{"type": "float", "field": "TOTALVOTES", "threshold": 1, "operation": "ABOVE"}]}"#,
            r#"{"filters": [{"type": "string", "field": "BPM", "equalsTo": "1"}]}"#,
        ];

        for request in requests {
            assert!(
                serde_json::from_str::<FilterRequest>(request).is_err(),
                "should reject {request}"
            );
        }
    }

    #[test]
    fn parsed_filter_variants_match_the_documented_types() {
        let request = parse(
            r#"{"filters": [
                {"type": "float", "field": "RATING", "operation": "ABOVE", "threshold": 0.5},
                {"type": "number", "field": "TOTALVOTES", "operation": "UNDER", "threshold": 100},
                {"type": "string", "field": "KEY", "equalsTo": "2e89c"}
            ]}"#,
        );

        assert!(matches!(request.filters[0], Filter::Float(_)));
        assert!(matches!(request.filters[1], Filter::Number(_)));
        assert!(matches!(request.filters[2], Filter::String(_)));
    }
}
