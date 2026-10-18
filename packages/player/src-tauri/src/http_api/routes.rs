use std::convert::Infallible;

use axum::{
    extract::{Path, State},
    http::{header, uri::Authority, HeaderValue, Method, Request, StatusCode},
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

pub fn router(
    bridge: Bridge,
    events_tx: broadcast::Sender<RemoteEvent>,
    expected_port: u16,
) -> Router {
    let state = AppState { bridge, events_tx };

    let router = Router::new()
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
        .with_state(state);

    secure_router(router, expected_port)
}

fn request_is_allowed(request: &Request<axum::body::Body>, expected_port: u16) -> bool {
    let Some(host_header) = request.headers().get(header::HOST).and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    let Ok(authority) = host_header.parse::<Authority>() else {
        return false;
    };
    let host = authority.host();

    let expected_host = format!("{host}:{expected_port}");
    if host != "127.0.0.1"
        || authority.as_str() != expected_host.as_str()
        || authority.port_u16() != Some(expected_port)
        || request.method() == Method::OPTIONS
    {
        return false;
    }

    match request.headers().get(header::ORIGIN) {
        Some(origin) => {
            let Ok(origin) = origin.to_str() else {
                return false;
            };
            let expected_origin = format!("http://{host}:{expected_port}");
            origin == expected_origin.as_str() || is_vite_development_origin(origin)
        }
        None => matches!(request.method().as_str(), "GET" | "HEAD"),
    }
}

#[cfg(debug_assertions)]
fn is_vite_development_origin(origin: &str) -> bool {
    origin == "http://127.0.0.1:5173"
}

#[cfg(not(debug_assertions))]
fn is_vite_development_origin(_: &str) -> bool {
    false
}

async fn validate_request(
    State(expected_port): State<u16>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    if !request_is_allowed(&request, expected_port) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

fn secure_router(router: Router, expected_port: u16) -> Router {
    router.layer(middleware::from_fn_with_state(
        expected_port,
        validate_request,
    ))
}

#[cfg(test)]
mod tests {
    use std::{
        convert::Infallible,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
    };

    use axum::{
        body::Body,
        http::{header, Method, Request, StatusCode},
        response::sse::{Event, Sse},
        routing::{get, post},
        Router,
    };
    use tokio::{net::TcpListener, task::JoinHandle};

    use super::{health, request_is_allowed, secure_router};

    async fn start_server(actions: Arc<AtomicUsize>) -> (String, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let router = secure_router(
            Router::new()
                .route("/api/health", get(health))
                .route(
                    "/api/events",
                    get(|| async {
                        Sse::new(futures::stream::once(async {
                            Ok::<_, Infallible>(Event::default().event("ready").data("{}"))
                        }))
                    }),
                )
                .route(
                    "/api/playback/next",
                    post(move || {
                        let actions = actions.clone();
                        async move {
                            actions.fetch_add(1, Ordering::SeqCst);
                            StatusCode::OK
                        }
                    }),
                ),
            address.port(),
        );
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        (format!("http://127.0.0.1:{}", address.port()), task)
    }

    #[tokio::test]
    async fn local_rest_sse_and_commands_remain_available() {
        let actions = Arc::new(AtomicUsize::new(0));
        let (base_url, task) = start_server(actions.clone()).await;
        let client = reqwest::Client::new();

        let health = client.get(format!("{base_url}/api/health")).send().await.unwrap();
        assert_eq!(health.status(), StatusCode::OK);
        assert_eq!(
            health.json::<serde_json::Value>().await.unwrap(),
            serde_json::json!({ "status": "ok" })
        );

        let origin = base_url.clone();
        let command = client
            .post(format!("{base_url}/api/playback/next"))
            .header(header::ORIGIN, origin)
            .send()
            .await
            .unwrap();
        assert_eq!(command.status(), StatusCode::OK);
        assert_eq!(actions.load(Ordering::SeqCst), 1);

        let dev_proxy_command = client
            .post(format!("{base_url}/api/playback/next"))
            .header(header::ORIGIN, "http://127.0.0.1:5173")
            .send()
            .await
            .unwrap();
        assert_eq!(dev_proxy_command.status(), StatusCode::OK);
        assert_eq!(actions.load(Ordering::SeqCst), 2);

        let events = client
            .get(format!("{base_url}/api/events"))
            .send()
            .await
            .unwrap();
        assert_eq!(events.status(), StatusCode::OK);
        assert!(events.text().await.unwrap().contains("event: ready"));
        task.abort();
    }

    #[tokio::test]
    async fn rejects_hostile_host_origin_and_bodyless_cross_site_commands() {
        let actions = Arc::new(AtomicUsize::new(0));
        let (base_url, task) = start_server(actions.clone()).await;
        let client = reqwest::Client::new();
        let hostile_host = client
            .get(format!("{base_url}/api/health"))
            .header(header::HOST, "attacker.example")
            .send()
            .await
            .unwrap();
        assert_eq!(hostile_host.status(), StatusCode::FORBIDDEN);

        let wrong_port = client
            .get(format!("{base_url}/api/health"))
            .header(header::HOST, "127.0.0.1:4120")
            .send()
            .await
            .unwrap();
        assert_eq!(wrong_port.status(), StatusCode::FORBIDDEN);

        let hostile_origin = client
            .get(format!("{base_url}/api/events"))
            .header(header::ORIGIN, "https://attacker.example")
            .send()
            .await
            .unwrap();
        assert_eq!(hostile_origin.status(), StatusCode::FORBIDDEN);

        let wrong_local_origin = client
            .get(format!("{base_url}/api/health"))
            .header(header::ORIGIN, "http://127.0.0.1:5174")
            .send()
            .await
            .unwrap();
        assert_eq!(wrong_local_origin.status(), StatusCode::FORBIDDEN);

        let cross_site_command = client
            .post(format!("{base_url}/api/playback/next"))
            .header(header::ORIGIN, "https://attacker.example")
            .send()
            .await
            .unwrap();
        assert_eq!(cross_site_command.status(), StatusCode::FORBIDDEN);
        assert_eq!(actions.load(Ordering::SeqCst), 0);
        task.abort();
    }

    #[tokio::test]
    async fn rejects_originless_mutations_and_cors_preflight() {
        let (base_url, task) = start_server(Arc::new(AtomicUsize::new(0))).await;
        let client = reqwest::Client::new();

        let originless_command = client
            .post(format!("{base_url}/api/playback/next"))
            .send()
            .await
            .unwrap();
        assert_eq!(originless_command.status(), StatusCode::FORBIDDEN);

        let preflight = client
            .request(Method::OPTIONS, format!("{base_url}/api/playback/next"))
            .header(header::ORIGIN, base_url.as_str())
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .send()
            .await
            .unwrap();
        assert_eq!(preflight.status(), StatusCode::FORBIDDEN);
        assert!(!preflight.headers().contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
        task.abort();
    }

    #[test]
    fn hostile_origin_does_not_reach_the_route_handler() {
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/playback/next")
            .header(header::HOST, "127.0.0.1:4120")
            .header(header::ORIGIN, "null")
            .body(Body::empty())
            .unwrap();

        assert!(!request_is_allowed(&request, 4120));
    }
}

