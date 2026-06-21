use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

type Rooms = Arc<Mutex<HashMap<String, Room>>>;

struct Room {
    agent_tx: Option<tokio::sync::mpsc::UnboundedSender<Message>>,
    viewers: Vec<tokio::sync::mpsc::UnboundedSender<Message>>,
}

#[derive(Deserialize)]
struct ServerMsg {
    #[serde(rename = "type")]
    msg_type: String,
    pin: Option<String>,
    event: Option<String>,
    x: Option<u32>,
    y: Option<u32>,
    button: Option<u32>,
    key: Option<String>,
    pressed: Option<bool>,
    dx: Option<i32>,
    dy: Option<i32>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let rooms: Rooms = Arc::new(Mutex::new(HashMap::new()));

    let app = Router::new()
        .route("/", get(index))
        .route("/ws", get(ws_handler))
        .with_state(rooms);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    info!("Relay listening on :8080");
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> impl IntoResponse {
    Html(include_str!("web/index.html"))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(rooms): State<Rooms>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, rooms))
}

async fn handle_socket(socket: WebSocket, rooms: Rooms) {
    let (mut sender, mut receiver) = socket.split();

    let mut current_pin: Option<String> = None;
    let mut is_agent = false;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(server_msg) = serde_json::from_str::<ServerMsg>(&text) {
                    match server_msg.msg_type.as_str() {
                        "register" => {
                            if let Some(pin) = server_msg.pin {
                                let mut rooms = rooms.lock().await;
                                if rooms.contains_key(&pin) {
                                    let _ = tx.send(Message::Text(
                                        serde_json::json!({"type":"error","msg":"PIN already in use"}).to_string().into(),
                                    ));
                                    continue;
                                }
                                rooms.insert(pin.clone(), Room {
                                    agent_tx: Some(tx.clone()),
                                    viewers: Vec::new(),
                                });
                                current_pin = Some(pin.clone());
                                is_agent = true;
                                let _ = tx.send(Message::Text(
                                    serde_json::json!({"type":"registered","pin":pin}).to_string().into(),
                                ));
                                info!("Agent registered with PIN: {}", pin);
                            }
                        }
                        "join" => {
                            if let Some(pin) = server_msg.pin {
                                let mut rooms = rooms.lock().await;
                                if let Some(room) = rooms.get_mut(&pin) {
                                    room.viewers.push(tx.clone());
                                    current_pin = Some(pin.clone());
                                    let _ = tx.send(Message::Text(
                                        serde_json::json!({"type":"joined","pin":pin}).to_string().into(),
                                    ));
                                    info!("Viewer joined room: {}", pin);
                                } else {
                                    let _ = tx.send(Message::Text(
                                        serde_json::json!({"type":"error","msg":"Room not found"}).to_string().into(),
                                    ));
                                }
                            }
                        }
                        "input" => {
                            if let Some(ref pin) = current_pin {
                                let rooms = rooms.lock().await;
                                if let Some(room) = rooms.get(pin) {
                                    if let Some(ref agent_tx) = room.agent_tx {
                                        let _ = agent_tx.send(Message::Text(text.clone()));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Message::Binary(data) => {
                if let Some(ref pin) = current_pin {
                    let rooms = rooms.lock().await;
                    if let Some(room) = rooms.get(pin) {
                        let frame = Message::Binary(data);
                        for viewer in &room.viewers {
                            let _ = viewer.send(frame.clone());
                        }
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    if let Some(pin) = current_pin {
        let mut rooms = rooms.lock().await;
        if is_agent {
            rooms.remove(&pin);
            info!("Agent disconnected, room {} removed", pin);
        } else if let Some(room) = rooms.get_mut(&pin) {
            room.viewers.retain(|v| !v.is_closed());
        }
    }

    send_task.abort();
}
