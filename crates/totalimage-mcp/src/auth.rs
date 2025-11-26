//! JWT Authentication for MCP Server
//!
//! Provides JWT token validation and API key authentication for the HTTP transport.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Expiration timestamp
    pub exp: u64,
    /// Issued at timestamp
    pub iat: u64,
    /// Issuer
    #[serde(default)]
    pub iss: Option<String>,
    /// Audience
    #[serde(default)]
    pub aud: Option<String>,
    /// Custom roles
    #[serde(default)]
    pub roles: Vec<String>,
    /// Project ID (optional)
    #[serde(default)]
    pub project_id: Option<String>,
}

/// Authentication configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Enable authentication (if false, all requests are allowed)
    pub enabled: bool,
    /// JWT secret key (for HMAC algorithms)
    pub jwt_secret: Option<String>,
    /// JWT public key (for RSA/EC algorithms)
    pub jwt_public_key: Option<String>,
    /// JWT algorithm
    pub jwt_algorithm: Algorithm,
    /// API keys (simple authentication)
    pub api_keys: Vec<String>,
    /// JWT issuer to validate
    pub jwt_issuer: Option<String>,
    /// JWT audience to validate
    pub jwt_audience: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            jwt_secret: None,
            jwt_public_key: None,
            jwt_algorithm: Algorithm::HS256,
            api_keys: vec![],
            jwt_issuer: None,
            jwt_audience: None,
        }
    }
}

impl AuthConfig {
    /// Create config from environment variables
    pub fn from_env() -> Self {
        let enabled = std::env::var("MCP_AUTH_ENABLED")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(false);

        let jwt_secret = std::env::var("MCP_JWT_SECRET").ok();
        let jwt_public_key = std::env::var("MCP_JWT_PUBLIC_KEY").ok();

        let jwt_algorithm = std::env::var("MCP_JWT_ALGORITHM")
            .ok()
            .and_then(|alg| match alg.to_uppercase().as_str() {
                "HS256" => Some(Algorithm::HS256),
                "HS384" => Some(Algorithm::HS384),
                "HS512" => Some(Algorithm::HS512),
                "RS256" => Some(Algorithm::RS256),
                "RS384" => Some(Algorithm::RS384),
                "RS512" => Some(Algorithm::RS512),
                "ES256" => Some(Algorithm::ES256),
                "ES384" => Some(Algorithm::ES384),
                _ => None,
            })
            .unwrap_or(Algorithm::HS256);

        let api_keys: Vec<String> = std::env::var("MCP_API_KEYS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let jwt_issuer = std::env::var("MCP_JWT_ISSUER").ok();
        let jwt_audience = std::env::var("MCP_JWT_AUDIENCE").ok();

        Self {
            enabled,
            jwt_secret,
            jwt_public_key,
            jwt_algorithm,
            api_keys,
            jwt_issuer,
            jwt_audience,
        }
    }

    /// Check if config is valid for authentication
    pub fn is_valid(&self) -> bool {
        if !self.enabled {
            return true; // Disabled auth is "valid"
        }
        // Must have either JWT secret/key or API keys
        self.jwt_secret.is_some() || self.jwt_public_key.is_some() || !self.api_keys.is_empty()
    }
}

/// Authentication result
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub roles: Vec<String>,
    pub project_id: Option<String>,
    pub auth_method: AuthMethod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod {
    Jwt,
    ApiKey,
    None,
}

/// Authentication error
#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken(String),
    ExpiredToken,
    InvalidApiKey,
    ConfigError(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authentication token"),
            AuthError::InvalidToken(_) => (StatusCode::UNAUTHORIZED, "Invalid authentication token"),
            AuthError::ExpiredToken => (StatusCode::UNAUTHORIZED, "Token has expired"),
            AuthError::InvalidApiKey => (StatusCode::UNAUTHORIZED, "Invalid API key"),
            AuthError::ConfigError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Authentication configuration error"),
        };

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": null,
            "error": {
                "code": -32000,
                "message": message
            }
        });

        (status, axum::Json(body)).into_response()
    }
}

/// Validate a JWT token
pub fn validate_jwt(token: &str, config: &AuthConfig) -> Result<Claims, AuthError> {
    let decoding_key = if let Some(ref secret) = config.jwt_secret {
        DecodingKey::from_secret(secret.as_bytes())
    } else if let Some(ref public_key) = config.jwt_public_key {
        DecodingKey::from_rsa_pem(public_key.as_bytes())
            .map_err(|e| AuthError::ConfigError(format!("Invalid public key: {}", e)))?
    } else {
        return Err(AuthError::ConfigError("No JWT key configured".to_string()));
    };

    let mut validation = Validation::new(config.jwt_algorithm);

    // Set issuer validation if configured (otherwise skip issuer validation)
    if let Some(ref issuer) = config.jwt_issuer {
        validation.set_issuer(&[issuer]);
    } else {
        validation.iss = None;
    }

    // Set audience validation if configured (otherwise skip audience validation)
    if let Some(ref audience) = config.jwt_audience {
        validation.set_audience(&[audience]);
    } else {
        validation.aud = None;
    }

    let token_data = decode::<Claims>(token, &decoding_key, &validation)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::ExpiredToken,
            _ => AuthError::InvalidToken(e.to_string()),
        })?;

    Ok(token_data.claims)
}

/// Validate an API key
pub fn validate_api_key(key: &str, config: &AuthConfig) -> bool {
    config.api_keys.iter().any(|k| k == key)
}

/// Extract bearer token from Authorization header
pub fn extract_bearer_token(auth_header: &str) -> Option<&str> {
    if auth_header.starts_with("Bearer ") || auth_header.starts_with("bearer ") {
        Some(&auth_header[7..])
    } else {
        None
    }
}

/// Authentication middleware for Axum
pub async fn auth_middleware(
    State(config): State<Arc<AuthConfig>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    // If auth is disabled, pass through
    if !config.enabled {
        return Ok(next.run(request).await);
    }

    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let auth_user = match auth_header {
        Some(header) => {
            // Try Bearer token (JWT)
            if let Some(token) = extract_bearer_token(header) {
                // First try as JWT
                match validate_jwt(token, &config) {
                    Ok(claims) => AuthUser {
                        user_id: claims.sub,
                        roles: claims.roles,
                        project_id: claims.project_id,
                        auth_method: AuthMethod::Jwt,
                    },
                    Err(_) => {
                        // Fall back to API key check
                        if validate_api_key(token, &config) {
                            AuthUser {
                                user_id: "api-user".to_string(),
                                roles: vec!["api".to_string()],
                                project_id: None,
                                auth_method: AuthMethod::ApiKey,
                            }
                        } else {
                            return Err(AuthError::InvalidToken("Invalid token or API key".to_string()));
                        }
                    }
                }
            } else {
                return Err(AuthError::MissingToken);
            }
        }
        None => {
            // Check for X-API-Key header
            let api_key = request
                .headers()
                .get("X-API-Key")
                .and_then(|h| h.to_str().ok());

            match api_key {
                Some(key) if validate_api_key(key, &config) => AuthUser {
                    user_id: "api-user".to_string(),
                    roles: vec!["api".to_string()],
                    project_id: None,
                    auth_method: AuthMethod::ApiKey,
                },
                Some(_) => return Err(AuthError::InvalidApiKey),
                None => return Err(AuthError::MissingToken),
            }
        }
    };

    // Store auth user in request extensions
    let mut request = request;
    request.extensions_mut().insert(auth_user);

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config_from_env() {
        // Test default (disabled)
        let config = AuthConfig::default();
        assert!(!config.enabled);
        assert!(config.is_valid());
    }

    #[test]
    fn test_extract_bearer_token() {
        assert_eq!(extract_bearer_token("Bearer abc123"), Some("abc123"));
        assert_eq!(extract_bearer_token("bearer abc123"), Some("abc123"));
        assert_eq!(extract_bearer_token("Basic abc123"), None);
        assert_eq!(extract_bearer_token("abc123"), None);
    }

    #[test]
    fn test_validate_api_key() {
        let config = AuthConfig {
            api_keys: vec!["key1".to_string(), "key2".to_string()],
            ..Default::default()
        };

        assert!(validate_api_key("key1", &config));
        assert!(validate_api_key("key2", &config));
        assert!(!validate_api_key("key3", &config));
        assert!(!validate_api_key("", &config));
    }
}
