use std::convert::Infallible;

use axum::{
    extract::{Path, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use futures::Stream;
use serde_json::{json, Value};
use tokio::sync::broadcast;

use super::{actions, lyrics, search, RemoteEvent};
use crate::bridge::{bridge::Bridge, types::BridgeError};

#[derive(Clone)]
pub struct AppState {
    pub(super) bridge: Bridge,
    events_tx: broadcast::Sender<RemoteEvent>,
}

pub(super) struct BridgeErrorResponse(pub(super) BridgeError);

fn local_request_allowed(headers: &HeaderMap) -> bool {
    let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) else { return false; };
    let Some((hostname, port)) = host.rsplit_once(':') else { return false; };
    let Ok(port) = port.parse::<u16>() else { return false; };
    if !(4120..=4129).contains(&port) || !(hostname == "127.0.0.1" || hostname.eq_ignore_ascii_case("localhost")) { return false; }
    match headers.get(header::ORIGIN) {
        None => true,
        Some(value) => value.to_str().map(|origin| {
            origin.eq_ignore_ascii_case(&format!("http://{host}"))
                || (cfg!(debug_assertions) && (origin == "http://localhost:5173"
                    || origin == "http://127.0.0.1:5173"))
        }).unwrap_or(false),
    }
}

async fn guard_local_request(request: Request, next: Next) -> Response {
    if !local_request_allowed(request.headers()) { return StatusCode::FORBIDDEN.into_response(); }
    let mut response = next.run(request).await;
    response.headers_mut().insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    response.headers_mut().insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    response
}


impl IntoResponse for BridgeErrorResponse {
    fn into_response(self) -> Response {
        let message = self.0.to_string();
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": message })),
        )
            .into_response()
    }
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn get_queue(State(state): State<AppState>) -> Result<Json<Value>, BridgeErrorResponse> {
    state
        .bridge
        .call("Queue.getQueue", json!({}))
        .await
        .map(Json)
        .map_err(BridgeErrorResponse)
}

async fn get_playback(State(state): State<AppState>) -> Result<Json<Value>, BridgeErrorResponse> {
    state
        .bridge
        .call("Playback.getState", json!({}))
        .await
        .map(Json)
        .map_err(BridgeErrorResponse)
}

async fn get_volume(State(state): State<AppState>) -> Result<Json<Value>, BridgeErrorResponse> {
    state
        .bridge
        .call("Playback.getVolume", json!({}))
        .await
        .map(|volume| Json(json!({ "volume": volume })))
        .map_err(BridgeErrorResponse)
}

async fn get_settings(State(state): State<AppState>) -> Result<Json<Value>, BridgeErrorResponse> {
    let bridge = &state.bridge;
    let (shuffle, repeat, discovery, language, dark, theme_id, lyrics_text_size) =
        tokio::try_join!(
            bridge.call("Settings.getGlobal", json!({"id": "core.playback.shuffle"})),
            bridge.call("Settings.getGlobal", json!({"id": "core.playback.repeat"})),
            bridge.call(
                "Settings.getGlobal",
                json!({"id": "core.playback.discovery"})
            ),
            bridge.call("Settings.getGlobal", json!({"id": "core.general.language"})),
            bridge.call("Settings.getGlobal", json!({"id": "core.theme.dark"})),
            bridge.call("Settings.getGlobal", json!({"id": "core.theme.active.id"})),
            bridge.call("Settings.getGlobal", json!({"id": "core.lyrics.textSize"})),
        )
        .map_err(BridgeErrorResponse)?;

    Ok(Json(json!({
        "shuffle": shuffle,
        "repeat": repeat,
        "discovery": discovery,
        "language": language,
        "dark": dark,
        "themeId": theme_id,
        "lyricsTextSize": lyrics_text_size,
    })))
}

async fn get_setting(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, BridgeErrorResponse> {
    state
        .bridge
        .call("Settings.getGlobal", json!({"id": id}))
        .await
        .map(Json)
        .map_err(BridgeErrorResponse)
}

async fn set_setting(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Result<StatusCode, BridgeErrorResponse> {
    state
        .bridge
        .call("Settings.setGlobal", json!({"id": id, "value": body}))
        .await
        .map(|_| StatusCode::OK)
        .map_err(BridgeErrorResponse)
}

fn events_stream(
    mut receiver: broadcast::Receiver<RemoteEvent>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    async_stream::stream! {
        yield Ok(Event::default().comment("connected"));

        loop {
            match receiver.recv().await {
                Ok(remote_event) => {
                    let event = Event::default()
                        .event(remote_event.kind.as_str())
                        .data(remote_event.data);
                    yield Ok(event);
                }
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    log::warn!("SSE client lagged, skipped {count} events");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    }
}

async fn get_events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = state.events_tx.subscribe();
    Sse::new(events_stream(receiver)).keep_alive(KeepAlive::default())
}

pub fn router(bridge: Bridge, events_tx: broadcast::Sender<RemoteEvent>) -> Router {
    let state = AppState { bridge, events_tx };

    Router::new()
        .route("/api/health", get(health))
        .route("/api/queue", get(get_queue))
        .route("/api/playback", get(get_playback))
        .route("/api/settings", get(get_settings))
        .route("/api/settings/{id}", get(get_setting).post(set_setting))
        .route("/api/events", get(get_events))
        .route("/api/playback/play", post(actions::play))
        .route("/api/playback/pause", post(actions::pause))
        .route("/api/playback/toggle", post(actions::toggle_playback))
        .route("/api/playback/next", post(actions::next_track))
        .route("/api/playback/previous", post(actions::previous_track))
        .route("/api/playback/seek", post(actions::seek))
        .route("/api/playback/shuffle", post(actions::set_shuffle))
        .route("/api/playback/repeat", post(actions::set_repeat))
        .route(
            "/api/playback/volume",
            get(get_volume).post(actions::set_volume),
        )
        .route("/api/queue/add", post(actions::add_to_queue))
        .route("/api/queue/remove", post(actions::remove_from_queue))
        .route("/api/queue/go-to", post(actions::go_to_index))
        .route("/api/queue/clear", post(actions::clear_queue))
        .route(
            "/api/favorites/tracks/add",
            post(actions::add_favorite_track),
        )
        .route(
            "/api/favorites/tracks/remove",
            post(actions::remove_favorite_track),
        )
        .route(
            "/api/playlists/from-queue",
            post(actions::save_queue_as_playlist),
        )
        .route("/api/search", post(search::search))
        .route("/api/lyrics/current", get(lyrics::get_current_lyrics))
        .fallback(super::frontend::serve_frontend)
        .with_state(state)
        .layer(middleware::from_fn(guard_local_request))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_foreign_hosts_and_origins() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("127.0.0.1:4120"));
        assert!(local_request_allowed(&headers));
        headers.insert(header::ORIGIN, HeaderValue::from_static("https://evil.example"));
        assert!(!local_request_allowed(&headers));
        headers.remove(header::ORIGIN);
        headers.insert(header::HOST, HeaderValue::from_static("evil.example:4120"));
        assert!(!local_request_allowed(&headers));
    }
}

