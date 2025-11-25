//! WebSocket support for real-time progress updates
//!
//! Provides WebSocket connections for streaming job progress and results.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    /// Job progress update
    Progress(ProgressUpdate),
    /// Job completed
    Completed(CompletedUpdate),
    /// Job failed
    Failed(FailedUpdate),
    /// Ping/pong for keepalive
    Ping,
    Pong,
    /// Subscription request
    Subscribe(SubscribeRequest),
    /// Unsubscribe request
    Unsubscribe(UnsubscribeRequest),
    /// Error message
    Error(ErrorMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    pub job_id: String,
    pub percentage: u8,
    pub stage: String,
    pub message: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedUpdate {
    pub job_id: String,
    pub result_summary: String,
    pub duration_ms: u64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedUpdate {
    pub job_id: String,
    pub error: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeRequest {
    pub job_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeRequest {
    pub job_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    pub code: i32,
    pub message: String,
}

/// WebSocket state for broadcasting updates
#[derive(Clone)]
pub struct WsState {
    /// Broadcast channel for sending updates
    pub tx: broadcast::Sender<WsMessage>,
}

impl Default for WsState {
    fn default() -> Self {
        Self::new()
    }
}

impl WsState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self { tx }
    }

    /// Broadcast a progress update to all connected clients
    pub fn broadcast_progress(&self, update: ProgressUpdate) {
        let _ = self.tx.send(WsMessage::Progress(update));
    }

    /// Broadcast a completion update
    pub fn broadcast_completed(&self, update: CompletedUpdate) {
        let _ = self.tx.send(WsMessage::Completed(update));
    }

    /// Broadcast a failure update
    pub fn broadcast_failed(&self, update: FailedUpdate) {
        let _ = self.tx.send(WsMessage::Failed(update));
    }
}

/// WebSocket upgrade handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WsState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(mut socket: WebSocket, state: Arc<WsState>) {
    tracing::info!("WebSocket client connected");

    // Subscribe to broadcast channel
    let mut rx = state.tx.subscribe();

    // Subscribed job IDs (empty = all)
    let mut subscribed_jobs: Option<Vec<String>> = None;

    loop {
        tokio::select! {
            // Handle incoming messages from client
            Some(msg) = socket.recv() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        match serde_json::from_str::<WsMessage>(&text) {
                            Ok(WsMessage::Subscribe(req)) => {
                                subscribed_jobs = Some(req.job_ids);
                                tracing::debug!("Client subscribed to jobs");
                            }
                            Ok(WsMessage::Unsubscribe(_)) => {
                                subscribed_jobs = None;
                                tracing::debug!("Client unsubscribed from all jobs");
                            }
                            Ok(WsMessage::Ping) => {
                                let pong = serde_json::to_string(&WsMessage::Pong).unwrap();
                                if socket.send(Message::Text(pong)).await.is_err() {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    Ok(Message::Close(_)) => {
                        tracing::info!("WebSocket client disconnected");
                        break;
                    }
                    Err(e) => {
                        tracing::warn!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            // Handle broadcast messages
            Ok(msg) = rx.recv() => {
                // Filter by subscription
                let should_send = match &subscribed_jobs {
                    None => true, // No filter, send all
                    Some(ids) => match &msg {
                        WsMessage::Progress(p) => ids.contains(&p.job_id),
                        WsMessage::Completed(c) => ids.contains(&c.job_id),
                        WsMessage::Failed(f) => ids.contains(&f.job_id),
                        _ => true,
                    }
                };

                if should_send {
                    if let Ok(text) = serde_json::to_string(&msg) {
                        if socket.send(Message::Text(text)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    }

    tracing::info!("WebSocket client handler terminated");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_message_serialization() {
        let progress = WsMessage::Progress(ProgressUpdate {
            job_id: "job-123".to_string(),
            percentage: 50,
            stage: "analyzing".to_string(),
            message: Some("Processing partitions".to_string()),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        });

        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("Progress"));
        assert!(json.contains("job-123"));
        assert!(json.contains("50"));
    }

    #[test]
    fn test_ws_state_broadcast() {
        let state = WsState::new();
        let mut rx = state.tx.subscribe();

        state.broadcast_progress(ProgressUpdate {
            job_id: "job-456".to_string(),
            percentage: 75,
            stage: "extracting".to_string(),
            message: None,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        });

        // Should receive the message
        let msg = rx.try_recv().unwrap();
        match msg {
            WsMessage::Progress(p) => {
                assert_eq!(p.job_id, "job-456");
                assert_eq!(p.percentage, 75);
            }
            _ => panic!("Expected Progress message"),
        }
    }
}
