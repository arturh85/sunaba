//! Native desktop OAuth implementation using local HTTP server for callback
//! Based on Authorization Code Flow with PKCE (RFC 7636)
//!
//! This is a lightweight, synchronous implementation optimized for minimal dependencies.
//! Uses ureq (sync HTTP) instead of reqwest, manual JWT parsing instead of jsonwebtoken.

use anyhow::{Context, Result};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tiny_http::{Response, Server};

/// Google OAuth Client ID (shared with WASM web client)
const GOOGLE_CLIENT_ID: &str =
    "1055019721589-7gh63ujmm7fekedmdnquo1f2fh9l5p3g.apps.googleusercontent.com";

/// Google OAuth endpoints
const GOOGLE_AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const OAUTH_SCOPE: &str = "openid email profile";

/// Local HTTP server port for OAuth callback (randomized to avoid conflicts)
const CALLBACK_PORT_MIN: u16 = 8000;
const CALLBACK_PORT_MAX: u16 = 9000;

/// OAuth claims extracted from JWT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthClaims {
    pub email: Option<String>,
    pub name: Option<String>,
    pub sub: String, // Google user ID
}

/// PKCE state for OAuth flow
struct PkceState {
    code_verifier: String,
    code_challenge: String,
    state: String,
}

impl PkceState {
    fn new() -> Self {
        let code_verifier = generate_random_string(64);
        let code_challenge = generate_code_challenge(&code_verifier);
        let state = generate_random_string(16);

        Self {
            code_verifier,
            code_challenge,
            state,
        }
    }
}

/// Generate cryptographically secure random string
fn generate_random_string(length: usize) -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..length).map(|_| rng.r#gen()).collect();
    URL_SAFE_NO_PAD.encode(&bytes)
}

/// Generate PKCE code challenge (SHA256 hash of verifier)
fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(&hash)
}

/// Start OAuth login flow (blocking)
/// Returns JWT token on success
pub fn oauth_login() -> Result<String> {
    log::info!("[OAuth] Starting native OAuth login flow...");

    // Find available port for local HTTP server
    let port = find_available_port(CALLBACK_PORT_MIN, CALLBACK_PORT_MAX)
        .context("Failed to find available port for OAuth callback")?;

    // Use 127.0.0.1 (loopback IP) instead of localhost per RFC 8252
    // This allows Google to accept any port dynamically without pre-registration
    let redirect_uri = format!("http://127.0.0.1:{}/callback", port);
    log::info!("[OAuth] Using redirect URI: {}", redirect_uri);

    // Generate PKCE state
    let pkce = PkceState::new();

    // Build Google OAuth authorization URL
    let auth_url = build_auth_url(GOOGLE_CLIENT_ID, &redirect_uri, &pkce);
    log::info!("[OAuth] Opening browser to: {}", auth_url);

    // Open browser to authorization URL
    if let Err(e) = webbrowser::open(&auth_url) {
        log::warn!("Failed to open browser automatically: {}", e);
        log::info!("Please open this URL manually: {}", auth_url);
    }

    log::info!("[OAuth] Waiting for user authorization...");

    // Start local HTTP server and wait for callback (blocking)
    let auth_code =
        receive_auth_code(port, &pkce.state).context("Failed to receive authorization code")?;

    log::info!("[OAuth] Authorization code received, exchanging for token...");

    // Exchange authorization code for JWT token (blocking)
    let token = exchange_code_for_token(
        GOOGLE_CLIENT_ID,
        &auth_code,
        &redirect_uri,
        &pkce.code_verifier,
    )
    .context("Failed to exchange authorization code for token")?;

    log::info!("[OAuth] OAuth login successful!");
    Ok(token)
}

/// Build Google OAuth authorization URL with PKCE
fn build_auth_url(client_id: &str, redirect_uri: &str, pkce: &PkceState) -> String {
    format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
        GOOGLE_AUTH_ENDPOINT,
        urlencoding::encode(client_id),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(OAUTH_SCOPE),
        urlencoding::encode(&pkce.code_challenge),
        urlencoding::encode(&pkce.state)
    )
}

/// Find an available port in the given range
fn find_available_port(min: u16, max: u16) -> Result<u16> {
    for port in min..=max {
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return Ok(port);
        }
    }
    anyhow::bail!("No available ports in range {}-{}", min, max)
}

/// Start local HTTP server and wait for OAuth callback (blocking)
fn receive_auth_code(port: u16, expected_state: &str) -> Result<String> {
    let server = Server::http(("127.0.0.1", port))
        .map_err(|e| anyhow::anyhow!("Failed to start local HTTP server: {}", e))?;

    let expected_state = expected_state.to_string();

    for request in server.incoming_requests() {
        log::debug!(
            "[OAuth] Received request: {:?} {}",
            request.method(),
            request.url()
        );

        // Parse query parameters from URL
        let url = request.url();
        let params = parse_query_params(url);

        // Check for error
        if let Some(error) = params.get("error") {
            let _ = request.respond(Response::from_string(format!(
                "❌ OAuth Error: {}\n\nYou can close this window.",
                error
            )));
            anyhow::bail!("OAuth error: {}", error);
        }

        // Validate state (CSRF protection)
        if let Some(state) = params.get("state") {
            if state != &expected_state {
                let _ = request.respond(Response::from_string(
                    "❌ State mismatch - possible CSRF attack\n\nYou can close this window.",
                ));
                anyhow::bail!("State mismatch - possible CSRF attack");
            }
        } else {
            continue; // Not the callback we're looking for
        }

        // Extract authorization code
        if let Some(code) = params.get("code") {
            log::info!("[OAuth] Authorization code received from callback");

            // Send success response to browser
            let _ = request.respond(Response::from_string(
                "✅ Login successful!\n\nYou can close this window and return to Sunaba.",
            ));

            return Ok(code.clone());
        }
    }

    anyhow::bail!("HTTP server stopped without receiving authorization code")
}

/// Parse query parameters from URL
fn parse_query_params(url: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();

    if let Some(query_start) = url.find('?') {
        let query = &url[query_start + 1..];
        for pair in query.split('&') {
            if let Some(eq_pos) = pair.find('=') {
                let key = &pair[..eq_pos];
                let value = &pair[eq_pos + 1..];
                params.insert(
                    urlencoding::decode(key).unwrap_or_default().to_string(),
                    urlencoding::decode(value).unwrap_or_default().to_string(),
                );
            }
        }
    }

    params
}

/// Exchange authorization code for JWT token (blocking, uses ureq)
fn exchange_code_for_token(
    client_id: &str,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<String> {
    let params = [
        ("client_id", client_id),
        ("code", code),
        ("code_verifier", code_verifier),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_uri),
    ];

    let response = ureq::post(GOOGLE_TOKEN_ENDPOINT)
        .send_form(&params)
        .context("Failed to send token exchange request")?;

    let token_response: TokenResponse = response
        .into_json()
        .context("Failed to parse token response")?;

    Ok(token_response.id_token)
}

#[derive(Deserialize)]
struct TokenResponse {
    id_token: String,
    // access_token and refresh_token also available if needed
}

/// Parse OAuth claims from JWT token (manual parsing, no verification)
/// Note: Verification is handled by SpacetimeDB server
///
/// JWT format: base64url(header).base64url(payload).signature
/// We only need the payload (claims)
pub fn parse_jwt_claims(token: &str) -> Result<OAuthClaims> {
    // Split JWT into parts
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        anyhow::bail!("Invalid JWT format: expected 3 parts, got {}", parts.len());
    }

    // Decode the payload (second part)
    let payload_base64 = parts[1];

    // JWT uses base64url encoding, need to add padding if missing
    let payload_padded = match payload_base64.len() % 4 {
        2 => format!("{}==", payload_base64),
        3 => format!("{}=", payload_base64),
        _ => payload_base64.to_string(),
    };

    // Decode base64
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_padded.as_bytes())
        .context("Failed to decode JWT payload base64")?;

    // Parse JSON
    let payload: serde_json::Value =
        serde_json::from_slice(&payload_bytes).context("Failed to parse JWT payload JSON")?;

    // Extract claims
    let claims = OAuthClaims {
        email: payload
            .get("email")
            .and_then(|v: &serde_json::Value| v.as_str())
            .map(|s: &str| s.to_string()),
        name: payload
            .get("name")
            .and_then(|v: &serde_json::Value| v.as_str())
            .map(|s: &str| s.to_string()),
        sub: payload
            .get("sub")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
    };

    Ok(claims)
}

/// Save OAuth token to file
pub fn save_oauth_token(token: &str) -> Result<()> {
    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?
        .join("sunaba");

    std::fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

    let token_path = config_dir.join("oauth_credentials.json");

    let credentials = serde_json::json!({
        "id_token": token,
        "saved_at": chrono::Utc::now().to_rfc3339()
    });

    std::fs::write(&token_path, serde_json::to_string_pretty(&credentials)?)
        .context("Failed to write OAuth credentials")?;

    log::info!("[OAuth] Saved credentials to: {}", token_path.display());
    Ok(())
}

/// Load OAuth token from file
pub fn load_oauth_token() -> Option<String> {
    let config_dir = dirs::config_dir()?.join("sunaba");
    let token_path = config_dir.join("oauth_credentials.json");

    if !token_path.exists() {
        return None;
    }

    let contents = std::fs::read_to_string(&token_path).ok()?;
    let credentials: serde_json::Value = serde_json::from_str(&contents).ok()?;

    credentials
        .get("id_token")
        .and_then(|v: &serde_json::Value| v.as_str())
        .map(|s: &str| s.to_string())
}

/// Delete OAuth token file (logout)
pub fn delete_oauth_token() -> Result<()> {
    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?
        .join("sunaba");

    let token_path = config_dir.join("oauth_credentials.json");

    if token_path.exists() {
        std::fs::remove_file(&token_path).context("Failed to delete OAuth credentials")?;
        log::info!("[OAuth] Deleted credentials from: {}", token_path.display());
    }

    Ok(())
}
