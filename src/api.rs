//! HTTP API: routes, handlers, and error responses.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;

pub mod filter;
pub mod models;
pub mod store;

pub use filter::FilterRequest;
pub use models::{SongDifficultyResponse, SongResponse};
pub use store::{AppState, SongStore};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/hash/{hash}", get(song_by_hash))
        .route("/api/v1/key/{key}", get(song_by_key))
        .route("/api/v1/hashes", get(list_hashes))
        .route("/api/v1/keys", get(list_keys))
        .route("/api/v1/filter", post(filter_songs))
        .fallback(route_not_found)
        .method_not_allowed_fallback(method_not_allowed)
        .with_state(state)
}

async fn route_not_found() -> ApiError {
    ApiError::NotFound("no route matches this path".to_owned())
}

async fn method_not_allowed() -> ApiError {
    ApiError::MethodNotAllowed
}

async fn song_by_hash(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> Result<Json<SongResponse>, ApiError> {
    state
        .store()
        .await
        .song_by_hash(&hash.to_uppercase())
        .cloned()
        .map(Json)
        .ok_or_else(|| ApiError::NotFound("song not found".to_owned()))
}

async fn song_by_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<SongResponse>, ApiError> {
    let map_id = u32::from_str_radix(&key, 16).map_err(|_| {
        ApiError::BadRequest("invalid key: expected a hexadecimal map id".to_owned())
    })?;

    state
        .store()
        .await
        .song_by_key(map_id)
        .cloned()
        .map(Json)
        .ok_or_else(|| ApiError::NotFound("song not found".to_owned()))
}

async fn list_hashes(State(state): State<AppState>) -> Json<Vec<String>> {
    let store = state.store().await;
    Json(store.songs().iter().map(|song| song.hash.clone()).collect())
}

async fn list_keys(State(state): State<AppState>) -> Json<Vec<String>> {
    let store = state.store().await;
    Json(store.songs().iter().map(|song| song.key.clone()).collect())
}

/// `POST /api/v1/filter` — returns the hashes of all songs matching every
/// filter, in store order.
async fn filter_songs(
    State(state): State<AppState>,
    payload: Result<Json<FilterRequest>, JsonRejection>,
) -> Result<Json<Vec<String>>, ApiError> {
    let Json(request) = payload.map_err(|rejection| ApiError::BadRequest(rejection.body_text()))?;

    let store = state.store().await;
    Ok(Json(filter::apply(store.songs(), &request.filters)))
}

#[derive(Debug)]
enum ApiError {
    NotFound(String),
    BadRequest(String),
    MethodNotAllowed,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::NotFound(message) => (StatusCode::NOT_FOUND, message),
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::MethodNotAllowed => (
                StatusCode::METHOD_NOT_ALLOWED,
                "this route does not support the request method".to_owned(),
            ),
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    use super::ApiError;

    async fn assert_error_response(error: ApiError, status: StatusCode, message: &str) {
        let response = error.into_response();
        assert_eq!(response.status(), status);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("error body should be readable");
        let json: serde_json::Value =
            serde_json::from_slice(&body).expect("error body should be JSON");

        assert_eq!(json, serde_json::json!({ "error": message }));
    }

    #[tokio::test]
    async fn not_found_has_json_error_message() {
        assert_error_response(
            ApiError::NotFound("song not found".to_owned()),
            StatusCode::NOT_FOUND,
            "song not found",
        )
        .await;
    }

    #[tokio::test]
    async fn bad_request_has_json_error_message() {
        assert_error_response(
            ApiError::BadRequest("invalid key".to_owned()),
            StatusCode::BAD_REQUEST,
            "invalid key",
        )
        .await;
    }

    #[tokio::test]
    async fn method_not_allowed_has_json_error_message() {
        assert_error_response(
            ApiError::MethodNotAllowed,
            StatusCode::METHOD_NOT_ALLOWED,
            "this route does not support the request method",
        )
        .await;
    }
}
