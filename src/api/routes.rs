use actix_web::{web, HttpResponse, Responder};
use base64::Engine;
use serde::{Deserialize, Serialize};
use log::{info, error, warn};
use std::sync::Arc;

use crate::crypto::{TokenCrypto, ENCRYPTED_TOKEN_SIZE};
use crate::store::{TokenStore, TokenStoreStats};

#[derive(Deserialize)]
pub struct RegisterTokenRequest {
    pub trade_pubkey: String,
    pub encrypted_token: String,
}

#[derive(Deserialize)]
pub struct UnregisterTokenRequest {
    pub trade_pubkey: String,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub version: String,
    pub server_pubkey: String,
    pub tokens: TokenStoreStats,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub token_store: Arc<TokenStore>,
    pub token_crypto: Arc<TokenCrypto>,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/health", web::get().to(health_check))
            .route("/status", web::get().to(status))
            .route("/register", web::post().to(register_token))
            .route("/unregister", web::post().to(unregister_token))
            .route("/info", web::get().to(server_info))
    );
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

async fn status(
    state: web::Data<AppState>,
) -> impl Responder {
    let stats = state.token_store.get_stats().await;
    
    HttpResponse::Ok().json(StatusResponse {
        status: "running".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        server_pubkey: state.token_crypto.public_key_hex(),
        tokens: stats,
    })
}

async fn server_info(
    state: web::Data<AppState>,
) -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "server_pubkey": state.token_crypto.public_key_hex(),
        "version": env!("CARGO_PKG_VERSION"),
        "encrypted_token_size": ENCRYPTED_TOKEN_SIZE,
    }))
}

async fn register_token(
    state: web::Data<AppState>,
    req: web::Json<RegisterTokenRequest>,
) -> impl Responder {
    info!("Registering token for trade_pubkey: {}...", 
        &req.trade_pubkey[..16.min(req.trade_pubkey.len())]);

    // Validate trade_pubkey format (should be 64 hex chars)
    if req.trade_pubkey.len() != 64 || hex::decode(&req.trade_pubkey).is_err() {
        warn!("Invalid trade_pubkey format");
        return HttpResponse::BadRequest().json(RegisterResponse {
            success: false,
            message: "Invalid trade_pubkey format (expected 64 hex characters)".to_string(),
            platform: None,
        });
    }

    // Decode base64 encrypted token
    let encrypted_token = match base64::engine::general_purpose::STANDARD.decode(
        &req.encrypted_token,
    ) {
        Ok(bytes) => bytes,
        Err(e) => {
            warn!("Invalid base64 in encrypted_token: {}", e);
            return HttpResponse::BadRequest().json(RegisterResponse {
                success: false,
                message: "Invalid base64 encoding in encrypted_token".to_string(),
                platform: None,
            });
        }
    };

    // Validate token size
    if encrypted_token.len() != ENCRYPTED_TOKEN_SIZE {
        warn!(
            "Invalid encrypted token size: expected {}, got {}",
            ENCRYPTED_TOKEN_SIZE,
            encrypted_token.len()
        );
        return HttpResponse::BadRequest().json(RegisterResponse {
            success: false,
            message: format!(
                "Invalid encrypted token size (expected {} bytes, got {})",
                ENCRYPTED_TOKEN_SIZE,
                encrypted_token.len()
            ),
            platform: None,
        });
    }

    // Decrypt the token
    let decrypted = match state.token_crypto.decrypt_token(&encrypted_token) {
        Ok(token) => token,
        Err(e) => {
            error!("Failed to decrypt token: {}", e);
            return HttpResponse::BadRequest().json(RegisterResponse {
                success: false,
                message: format!("Failed to decrypt token: {}", e),
                platform: None,
            });
        }
    };

    // Store the token
    state.token_store.register(
        req.trade_pubkey.clone(),
        decrypted.device_token,
        decrypted.platform.clone(),
    ).await;

    info!(
        "Successfully registered {} token for trade_pubkey: {}...",
        decrypted.platform,
        &req.trade_pubkey[..16]
    );

    HttpResponse::Ok().json(RegisterResponse {
        success: true,
        message: "Token registered successfully".to_string(),
        platform: Some(decrypted.platform.to_string()),
    })
}

async fn unregister_token(
    state: web::Data<AppState>,
    req: web::Json<UnregisterTokenRequest>,
) -> impl Responder {
    info!("Unregistering token for trade_pubkey: {}...", 
        &req.trade_pubkey[..16.min(req.trade_pubkey.len())]);

    // Validate trade_pubkey format
    if req.trade_pubkey.len() != 64 || hex::decode(&req.trade_pubkey).is_err() {
        warn!("Invalid trade_pubkey format");
        return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "message": "Invalid trade_pubkey format (expected 64 hex characters)"
        }));
    }

    let removed = state.token_store.unregister(&req.trade_pubkey).await;

    if removed {
        HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Token unregistered successfully"
        }))
    } else {
        HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Token not found (may have already been unregistered)"
        }))
    }
}
