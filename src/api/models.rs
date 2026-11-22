//! JSON representations of scraped songs.

use serde::Serialize;

use crate::maps::{MapCharacteristic, MapDifficulty, SongDifficultyProto, SongProto};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongResponse {
    /// `map_id` as a lowercase hexadecimal string.
    pub key: String,
    pub map_id: u32,
    pub song_name: String,
    pub song_author_name: String,
    pub level_author_name: String,
    pub uploader_name: String,
    pub bpm: f32,
    pub download_count: u32,
    pub upvotes: u32,
    pub downvotes: u32,
    pub upload_time_unix: u32,
    pub song_duration_seconds: u32,
    pub ranked_status: i32,
    pub ranked_change_unix: u32,
    /// `hash_bytes` as an uppercase hexadecimal string.
    pub hash: String,
    pub difficulties: Vec<SongDifficultyResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongDifficultyResponse {
    pub characteristic: i32,
    pub difficulty: i32,
    pub stars_t100: u32,
    pub njs_t100: u32,
    pub bombs: u32,
    pub notes: u32,
    pub obstacles: u32,
    pub mods: i32,
    pub characteristic_human: String,
    pub difficulty_human: String,
}

impl From<&SongProto> for SongResponse {
    fn from(song: &SongProto) -> Self {
        Self {
            key: format!("{:x}", song.map_id),
            map_id: song.map_id,
            song_name: song.song_name.clone(),
            song_author_name: song.song_author_name.clone(),
            level_author_name: song.level_author_name.clone(),
            uploader_name: song.uploader_name.clone(),
            bpm: song.bpm,
            download_count: song.download_count,
            upvotes: song.upvotes,
            downvotes: song.downvotes,
            upload_time_unix: song.upload_time_unix,
            song_duration_seconds: song.song_duration_seconds,
            ranked_status: song.ranked_state,
            ranked_change_unix: song.ranked_change_unix,
            hash: song.hash(),
            difficulties: song
                .difficulties
                .iter()
                .map(SongDifficultyResponse::from)
                .collect(),
        }
    }
}

impl From<&SongDifficultyProto> for SongDifficultyResponse {
    fn from(difficulty: &SongDifficultyProto) -> Self {
        Self {
            characteristic: difficulty.characteristic,
            difficulty: difficulty.difficulty,
            stars_t100: difficulty.stars_t100,
            njs_t100: difficulty.njs_t100,
            bombs: difficulty.bombs,
            notes: difficulty.notes,
            obstacles: difficulty.obstacles,
            mods: difficulty.mods,
            characteristic_human: characteristic_name(difficulty.characteristic),
            difficulty_human: difficulty_name(difficulty.difficulty),
        }
    }
}

fn characteristic_name(value: i32) -> String {
    match MapCharacteristic::try_from(value) {
        Ok(MapCharacteristic::NinetyDegree) => "90Degree".to_owned(),
        Ok(MapCharacteristic::ThreeSixtyDegree) => "360Degree".to_owned(),
        Ok(characteristic) => format!("{characteristic:?}"),
        Err(_) => "Unknown".to_owned(),
    }
}

fn difficulty_name(value: i32) -> String {
    match MapDifficulty::try_from(value) {
        Ok(difficulty) => format!("{difficulty:?}"),
        Err(_) => "Unknown".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{SongResponse, characteristic_name, difficulty_name};
    use crate::maps::{SongDifficultyProto, SongProto};

    #[test]
    fn serialises_hash_as_uppercase_hex() {
        let song = SongProto {
            map_id: 42,
            hash_bytes: vec![0x0a, 0xbc, 0xde, 0xf1],
            song_name: "Test Song".to_owned(),
            ranked_state: 1,
            difficulties: vec![SongDifficultyProto {
                characteristic: 5,
                difficulty: 3,
                ..Default::default()
            }],
            ..Default::default()
        };

        let response = SongResponse::from(&song);
        assert_eq!(response.hash, "0ABCDEF1");
        assert_eq!(response.key, "2a");

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["hash"], "0ABCDEF1");
        assert_eq!(json["key"], "2a");
        assert_eq!(json["mapId"], 42);
        assert_eq!(json["songName"], "Test Song");
        assert_eq!(json["rankedStatus"], 1);
        assert!(json.get("rankedState").is_none());
        assert!(json.get("hashBytes").is_none());
        assert_eq!(json["difficulties"][0]["characteristicHuman"], "360Degree");
        assert_eq!(json["difficulties"][0]["difficultyHuman"], "Expert");
    }

    #[test]
    fn human_names_match_the_api_contract() {
        assert_eq!(characteristic_name(0), "Custom");
        assert_eq!(characteristic_name(1), "Standard");
        assert_eq!(characteristic_name(2), "OneSaber");
        assert_eq!(characteristic_name(3), "NoArrows");
        assert_eq!(characteristic_name(4), "90Degree");
        assert_eq!(characteristic_name(5), "360Degree");
        assert_eq!(characteristic_name(6), "Lightshow");
        assert_eq!(characteristic_name(7), "Lawless");
        assert_eq!(characteristic_name(99), "Unknown");

        assert_eq!(difficulty_name(0), "Easy");
        assert_eq!(difficulty_name(1), "Normal");
        assert_eq!(difficulty_name(2), "Hard");
        assert_eq!(difficulty_name(3), "Expert");
        assert_eq!(difficulty_name(4), "ExpertPlus");
        assert_eq!(difficulty_name(99), "Unknown");
    }
}
