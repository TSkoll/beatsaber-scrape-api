//! In-memory song store and the shared application state built around it.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::models::SongResponse;

/// Fetched songs plus lookup indexes by hash and map id.
#[derive(Debug, Default)]
pub struct SongStore {
    songs: Vec<SongResponse>,
    by_hash: HashMap<String, usize>,
    by_key: HashMap<u32, usize>,
}

impl SongStore {
    pub fn new(songs: Vec<SongResponse>) -> Self {
        let mut by_hash = HashMap::with_capacity(songs.len());
        let mut by_key = HashMap::with_capacity(songs.len());

        for (index, song) in songs.iter().enumerate() {
            by_hash.entry(song.hash.clone()).or_insert(index);
            by_key.entry(song.map_id).or_insert(index);
        }

        Self {
            songs,
            by_hash,
            by_key,
        }
    }

    pub fn songs(&self) -> &[SongResponse] {
        &self.songs
    }

    /// Looks up a song by its uppercase hex hash.
    pub fn song_by_hash(&self, hash: &str) -> Option<&SongResponse> {
        self.by_hash.get(hash).map(|&index| &self.songs[index])
    }

    pub fn song_by_key(&self, key: u32) -> Option<&SongResponse> {
        self.by_key.get(&key).map(|&index| &self.songs[index])
    }
}

#[derive(Clone, Default)]
pub struct AppState {
    songs: Arc<RwLock<Arc<SongStore>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Cheap snapshot of the current store; the lock is only held for the clone.
    pub async fn store(&self) -> Arc<SongStore> {
        self.songs.read().await.clone()
    }

    /// Swaps in freshly fetched data without blocking readers for long.
    pub async fn set_store(&self, store: SongStore) {
        *self.songs.write().await = Arc::new(store);
    }
}

#[cfg(test)]
mod tests {
    use super::SongStore;
    use crate::api::SongResponse;
    use crate::maps::SongProto;

    #[test]
    fn looks_up_songs_by_hash_and_key() {
        let song = SongProto {
            map_id: 42,
            hash_bytes: vec![0x0a, 0xbc, 0xde, 0xf1],
            song_name: "Test Song".to_owned(),
            ..Default::default()
        };

        let store = SongStore::new(vec![SongResponse::from(&song)]);

        assert_eq!(store.song_by_hash("0ABCDEF1").unwrap().map_id, 42);
        assert!(store.song_by_hash("0abcdef1").is_none());
        assert_eq!(store.song_by_key(42).unwrap().hash, "0ABCDEF1");
        assert!(store.song_by_key(7).is_none());
    }
}
