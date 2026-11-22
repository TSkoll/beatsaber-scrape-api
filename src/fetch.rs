//! Downloading the scraped song database and the background refresh schedule.

use std::io;
use std::thread;

use async_compression::tokio::bufread::GzipDecoder;
use bytes::Bytes;
use futures_util::TryStreamExt;
use prost::Message;
use tokio::io::{AsyncReadExt, BufReader};
use tokio_cron_scheduler::{Job, JobScheduler};
use tokio_util::io::StreamReader;

use crate::api::{AppState, SongResponse, SongStore};
use crate::maps;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Every day at 00:00 and 12:00.
const REFRESH_SCHEDULE: &str = "0 0 */12 * * *";

const SONGS_URL: &str =
    "https://raw.githubusercontent.com/andruzzzhka/BeatSaberScrappedData/master/songDetails2.gz";

async fn fetch_maps() -> Result<Vec<maps::SongProto>, BoxError> {
    let response = reqwest::get(SONGS_URL).await?.error_for_status()?;

    let body = response.bytes_stream();
    let stream_reader = StreamReader::new(body.map_err(io::Error::other));
    let mut decoder = GzipDecoder::new(BufReader::new(stream_reader));

    let mut buffer = Vec::new();
    decoder.read_to_end(&mut buffer).await?;

    let container = maps::SongProtoContainer::decode(Bytes::from(buffer))?;
    Ok(container.songs)
}

/// Fills the store from a fetch at startup, then refreshes it on the cron
/// schedule. The work runs on a dedicated thread and runtime so it never
/// competes with request handling.
pub fn spawn_refresh_task(state: AppState) {
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build scheduler runtime");

        runtime.block_on(async move {
            let scheduler = JobScheduler::new()
                .await
                .expect("failed to create job scheduler");

            let job_state = state.clone();
            let job = Job::new_async(REFRESH_SCHEDULE, move |_id, _label| {
                let state = job_state.clone();
                Box::pin(async move { refresh_maps(state).await })
            })
            .expect("invalid cron schedule");

            scheduler.add(job).await.expect("failed to add fetch job");
            scheduler
                .start()
                .await
                .expect("failed to start job scheduler");

            // Run the job once on startup, then keep following the cron schedule.
            tokio::spawn(refresh_maps(state));

            // Keep the scheduler thread alive for the lifetime of the process.
            std::future::pending::<()>().await;
        });
    });
}

async fn refresh_maps(state: AppState) {
    match fetch_maps().await {
        Ok(maps) => {
            let songs: Vec<SongResponse> = maps.iter().map(SongResponse::from).collect();
            let song_count = songs.len();
            state.set_store(SongStore::new(songs)).await;
            tracing::info!(songs = song_count, "fetched map data");
        }
        Err(err) => tracing::error!(%err, "failed to fetch map data"),
    }
}
