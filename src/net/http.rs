use bevy::prelude::*;
use std::sync::Arc;
use super::{NetworkEvent, TokioRuntime};
use serde::{Deserialize, Serialize};

/// Request types for the HTTP API calls.
#[derive(Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct FriendRequestBody {
    pub username: String,
}

#[derive(Serialize)]
pub struct FriendActionBody {
    pub request_id: uuid::Uuid,
}

#[derive(Serialize)]
pub struct MatchmakingQueueBody {
    pub game_mode: String,
}

/// Response types from the server.
#[derive(Deserialize, Debug)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: uuid::Uuid,
    pub username: String,
}

#[derive(Deserialize, Debug)]
pub struct ErrorResponse {
    pub message: String,
}

#[derive(Deserialize, Debug)]
pub struct FriendRequestsResponse {
    pub incoming: Vec<noctyrn_shared::player::FriendRequestInfo>,
    pub outgoing: Vec<noctyrn_shared::player::FriendRequestInfo>,
}

/// Resource that holds pending async results via a shared buffer.
/// Async tasks push completed NetworkEvents here; the poll system drains them.
#[derive(Resource, Clone)]
pub struct PendingRequests {
    pub results: Arc<std::sync::Mutex<Vec<NetworkEvent>>>,
}

impl Default for PendingRequests {
    fn default() -> Self {
        Self {
            results: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
}

/// System that polls pending HTTP requests and forwards results as messages.
pub fn poll_pending_requests(
    pending: Res<PendingRequests>,
    mut events: MessageWriter<NetworkEvent>,
) {
    let mut results = pending.results.lock().unwrap();
    for event in results.drain(..) {
        events.write(event);
    }
}

/// Helper to spawn an async HTTP request and track it.
pub fn spawn_http_request(
    rt: &TokioRuntime,
    pending: &PendingRequests,
    future: impl std::future::Future<Output = NetworkEvent> + Send + 'static,
) {
    let results = pending.results.clone();
    rt.0.spawn(async move {
        let event = future.await;
        results.lock().unwrap().push(event);
    });
}

/// Async function: login request
pub async fn async_login(base_url: String, email: String, password: String) -> NetworkEvent {
    let client = reqwest::Client::new();
    let url = format!("{}/auth/login", base_url);

    match client.post(&url).json(&LoginRequest { email, password }).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<AuthResponse>().await {
                Ok(auth) => NetworkEvent::LoginSuccess {
                    token: auth.token,
                    user_id: auth.user_id,
                    username: auth.username,
                },
                Err(e) => NetworkEvent::LoginError {
                    message: format!("Failed to parse response: {}", e),
                },
            }
        }
        Ok(resp) => {
            let msg = resp.json::<ErrorResponse>().await
                .map(|e| e.message)
                .unwrap_or_else(|_| "Login failed".to_string());
            NetworkEvent::LoginError { message: msg }
        }
        Err(e) => NetworkEvent::ConnectionError {
            message: format!("Could not connect to server. Please check that the server is running.\n{}", e),
        },
    }
}

/// Async function: register request
pub async fn async_register(base_url: String, username: String, email: String, password: String) -> NetworkEvent {
    let client = reqwest::Client::new();
    let url = format!("{}/auth/register", base_url);

    match client.post(&url).json(&RegisterRequest { username, email, password }).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<AuthResponse>().await {
                Ok(auth) => NetworkEvent::RegisterSuccess {
                    token: auth.token,
                    user_id: auth.user_id,
                    username: auth.username,
                },
                Err(e) => NetworkEvent::RegisterError {
                    message: format!("Failed to parse response: {}", e),
                },
            }
        }
        Ok(resp) => {
            let msg = resp.json::<ErrorResponse>().await
                .map(|e| e.message)
                .unwrap_or_else(|_| "Registration failed".to_string());
            NetworkEvent::RegisterError { message: msg }
        }
        Err(e) => NetworkEvent::ConnectionError {
            message: format!("Could not connect to server. Please check that the server is running.\n{}", e),
        },
    }
}

/// Async function: fetch profile
pub async fn async_get_profile(base_url: String, token: String) -> NetworkEvent {
    let client = reqwest::Client::new();
    let url = format!("{}/profile", base_url);

    match client.get(&url).bearer_auth(&token).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<noctyrn_shared::player::PlayerProfile>().await {
                Ok(profile) => NetworkEvent::ProfileLoaded { profile },
                Err(e) => NetworkEvent::ProfileError {
                    message: format!("Failed to parse profile: {}", e),
                },
            }
        }
        Ok(resp) => {
            let msg = resp.json::<ErrorResponse>().await
                .map(|e| e.message)
                .unwrap_or_else(|_| "Failed to load profile".to_string());
            NetworkEvent::ProfileError { message: msg }
        }
        Err(e) => NetworkEvent::ConnectionError {
            message: format!("Could not connect to server: {}", e),
        },
    }
}

/// Async function: fetch friends list
pub async fn async_get_friends(base_url: String, token: String) -> NetworkEvent {
    let client = reqwest::Client::new();
    let url = format!("{}/friends", base_url);

    match client.get(&url).bearer_auth(&token).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<Vec<noctyrn_shared::player::FriendEntry>>().await {
                Ok(friends) => NetworkEvent::FriendsLoaded { friends },
                Err(e) => NetworkEvent::FriendError {
                    message: format!("Failed to parse friends: {}", e),
                },
            }
        }
        Ok(resp) => {
            let msg = resp.json::<ErrorResponse>().await
                .map(|e| e.message)
                .unwrap_or_else(|_| "Failed to load friends".to_string());
            NetworkEvent::FriendError { message: msg }
        }
        Err(e) => NetworkEvent::ConnectionError {
            message: format!("Could not connect to server: {}", e),
        },
    }
}

/// Async function: fetch friend requests
pub async fn async_get_friend_requests(base_url: String, token: String) -> NetworkEvent {
    let client = reqwest::Client::new();
    let url = format!("{}/friends/requests", base_url);

    match client.get(&url).bearer_auth(&token).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<FriendRequestsResponse>().await {
                Ok(data) => NetworkEvent::FriendRequestsLoaded {
                    incoming: data.incoming,
                    outgoing: data.outgoing,
                },
                Err(e) => NetworkEvent::FriendError {
                    message: format!("Failed to parse requests: {}", e),
                },
            }
        }
        Ok(resp) => {
            let msg = resp.json::<ErrorResponse>().await
                .map(|e| e.message)
                .unwrap_or_else(|_| "Failed to load friend requests".to_string());
            NetworkEvent::FriendError { message: msg }
        }
        Err(e) => NetworkEvent::ConnectionError {
            message: format!("Could not connect to server: {}", e),
        },
    }
}

/// Async function: send friend request
pub async fn async_send_friend_request(base_url: String, token: String, target_username: String) -> NetworkEvent {
    let client = reqwest::Client::new();
    let url = format!("{}/friends/request", base_url);

    match client.post(&url).bearer_auth(&token).json(&FriendRequestBody { username: target_username }).send().await {
        Ok(resp) if resp.status().is_success() => NetworkEvent::FriendRequestSent,
        Ok(resp) => {
            let msg = resp.json::<ErrorResponse>().await
                .map(|e| e.message)
                .unwrap_or_else(|_| "Failed to send friend request".to_string());
            NetworkEvent::FriendError { message: msg }
        }
        Err(e) => NetworkEvent::ConnectionError {
            message: format!("Could not connect to server: {}", e),
        },
    }
}

/// Async function: accept friend request
pub async fn async_accept_friend_request(base_url: String, token: String, request_id: uuid::Uuid) -> NetworkEvent {
    let client = reqwest::Client::new();
    let url = format!("{}/friends/accept", base_url);

    match client.post(&url).bearer_auth(&token).json(&FriendActionBody { request_id }).send().await {
        Ok(resp) if resp.status().is_success() => NetworkEvent::FriendRequestAccepted,
        Ok(resp) => {
            let msg = resp.json::<ErrorResponse>().await
                .map(|e| e.message)
                .unwrap_or_else(|_| "Failed to accept request".to_string());
            NetworkEvent::FriendError { message: msg }
        }
        Err(e) => NetworkEvent::ConnectionError {
            message: format!("Could not connect to server: {}", e),
        },
    }
}

/// Async function: decline friend request
pub async fn async_decline_friend_request(base_url: String, token: String, request_id: uuid::Uuid) -> NetworkEvent {
    let client = reqwest::Client::new();
    let url = format!("{}/friends/decline", base_url);

    match client.post(&url).bearer_auth(&token).json(&FriendActionBody { request_id }).send().await {
        Ok(resp) if resp.status().is_success() => NetworkEvent::FriendRequestDeclined,
        Ok(resp) => {
            let msg = resp.json::<ErrorResponse>().await
                .map(|e| e.message)
                .unwrap_or_else(|_| "Failed to decline request".to_string());
            NetworkEvent::FriendError { message: msg }
        }
        Err(e) => NetworkEvent::ConnectionError {
            message: format!("Could not connect to server: {}", e),
        },
    }
}

/// Async function: remove friend
pub async fn async_remove_friend(base_url: String, token: String, friend_id: uuid::Uuid) -> NetworkEvent {
    let client = reqwest::Client::new();
    let url = format!("{}/friends/{}", base_url, friend_id);

    match client.delete(&url).bearer_auth(&token).send().await {
        Ok(resp) if resp.status().is_success() => NetworkEvent::FriendRemoved,
        Ok(resp) => {
            let msg = resp.json::<ErrorResponse>().await
                .map(|e| e.message)
                .unwrap_or_else(|_| "Failed to remove friend".to_string());
            NetworkEvent::FriendError { message: msg }
        }
        Err(e) => NetworkEvent::ConnectionError {
            message: format!("Could not connect to server: {}", e),
        },
    }
}
