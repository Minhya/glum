use axum::{
    Json, Router,
    extract::{Query, State},
    http::{
        HeaderMap, StatusCode,
        header::{AUTHORIZATION, COOKIE, LOCATION, SET_COOKIE},
    },
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use tonic::{Request, Status};
use uuid::Uuid;

const TOKEN_AUDIENCE: &str = "glum-app";
const TOKEN_ISSUER: &str = "glum";
const TOKEN_TTL_DAYS: i64 = 7;
const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://openidconnect.googleapis.com/v1/userinfo";
const GOOGLE_SCOPES: &str = "openid email profile https://www.googleapis.com/auth/calendar.events";
const OAUTH_STATE_COOKIE: &str = "glum_oauth_state";

#[derive(Clone)]
pub struct AuthConfig {
    pool: PgPool,
    client: Client,
    jwt_secret: Arc<String>,
    google_client_id: String,
    google_client_secret: String,
    google_redirect_uri: String,
    web_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: Option<String>,
    pub exp: usize,
    pub aud: String,
    pub iss: String,
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct GoogleUserInfo {
    sub: String,
    email: String,
    name: Option<String>,
    picture: Option<String>,
}

#[derive(Serialize)]
struct MeResponse {
    id: String,
    email: Option<String>,
}

impl AuthConfig {
    pub fn from_env(pool: PgPool, jwt_secret: Arc<String>) -> Self {
        Self {
            pool,
            client: Client::new(),
            jwt_secret,
            google_client_id: std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default(),
            google_client_secret: std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default(),
            google_redirect_uri: std::env::var("GOOGLE_REDIRECT_URI")
                .unwrap_or_else(|_| "http://localhost:8090/auth/google/callback".to_string()),
            web_url: std::env::var("WEB_URL")
                .unwrap_or_else(|_| "http://localhost:4000/".to_string()),
        }
    }
}

pub fn router(config: AuthConfig) -> Router {
    Router::new()
        .route("/auth/health", get(|| async { "ok" }))
        .route("/auth/google/login", get(google_login))
        .route("/auth/google/callback", get(google_callback))
        .route("/auth/me", get(me))
        .with_state(config)
}

pub fn verify_token(token: &str, jwt_secret: &str) -> Result<Claims, Status> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&[TOKEN_AUDIENCE]);
    validation.set_issuer(&[TOKEN_ISSUER]);

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|_| Status::unauthenticated("Invalid or expired token"))
}

pub fn current_user_id<T>(request: &Request<T>) -> Result<&str, Status> {
    request
        .extensions()
        .get::<Claims>()
        .map(|claims| claims.sub.as_str())
        .filter(|sub| !sub.is_empty())
        .ok_or_else(|| Status::unauthenticated("Missing authenticated user"))
}

pub fn extract_token<T>(request: &Request<T>) -> Result<&str, Status> {
    let metadata = request.metadata();
    let auth = metadata
        .get("authorization")
        .ok_or_else(|| Status::unauthenticated("Missing authorization header"))?
        .to_str()
        .map_err(|_| Status::unauthenticated("Invalid authorization header"))?;

    auth.strip_prefix("Bearer ")
        .ok_or_else(|| Status::unauthenticated("Invalid authorization format"))
}

async fn google_login(State(config): State<AuthConfig>) -> Response {
    if config.google_client_id.is_empty() || config.google_client_secret.is_empty() {
        return auth_error_redirect(&config.web_url, "Google OAuth is not configured.");
    }

    let state = Uuid::new_v4().to_string();
    let mut url = Url::parse(GOOGLE_AUTH_URL).expect("static Google auth URL is valid");
    url.query_pairs_mut()
        .append_pair("client_id", &config.google_client_id)
        .append_pair("redirect_uri", &config.google_redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", GOOGLE_SCOPES)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent")
        .append_pair("state", &state);

    let cookie = format!(
        "{OAUTH_STATE_COOKIE}={state}; Path=/auth/google; HttpOnly; SameSite=Lax; Max-Age=600"
    );

    let mut response = Redirect::temporary(url.as_str()).into_response();
    response
        .headers_mut()
        .insert(SET_COOKIE, cookie.parse().expect("valid state cookie"));
    response
}

async fn google_callback(
    State(config): State<AuthConfig>,
    headers: HeaderMap,
    Query(query): Query<CallbackQuery>,
) -> Response {
    if let Some(error) = query.error {
        return auth_error_redirect(&config.web_url, &format!("Google login failed: {error}"));
    }

    let Some(code) = query.code else {
        return auth_error_redirect(&config.web_url, "Google login did not return a code.");
    };
    let Some(state) = query.state else {
        return auth_error_redirect(&config.web_url, "Google login did not return state.");
    };

    if !state_cookie_matches(&headers, &state) {
        return auth_error_redirect(&config.web_url, "Google login state was invalid.");
    }

    let token = match exchange_google_code(&config, &code).await {
        Ok(token) => token,
        Err(message) => return auth_error_redirect(&config.web_url, &message),
    };

    let user = match fetch_google_user(&config.client, &token.access_token).await {
        Ok(user) => user,
        Err(message) => return auth_error_redirect(&config.web_url, &message),
    };

    if let Err(error) = upsert_user(&config.pool, &user, token.refresh_token.as_deref()).await {
        eprintln!("upsert Google user failed: {error:?}");
        return auth_error_redirect(&config.web_url, "Could not save the Google account.");
    }

    let jwt = match issue_token(&config.jwt_secret, &user.sub, Some(&user.email)) {
        Ok(jwt) => jwt,
        Err(error) => {
            eprintln!("issue app token failed: {error:?}");
            return auth_error_redirect(&config.web_url, "Could not create a Glum session.");
        }
    };

    let redirect = format!("{}#token={jwt}", trim_hash_redirect(&config.web_url));
    let mut response = Response::new(axum::body::Body::empty());
    *response.status_mut() = StatusCode::FOUND;
    response
        .headers_mut()
        .insert(LOCATION, redirect.parse().expect("valid redirect URL"));
    response.headers_mut().insert(
        SET_COOKIE,
        format!("{OAUTH_STATE_COOKIE}=; Path=/auth/google; Max-Age=0")
            .parse()
            .expect("valid expired state cookie"),
    );
    response
}

async fn me(State(config): State<AuthConfig>, headers: HeaderMap) -> impl IntoResponse {
    let Some(token) = bearer_from_headers(&headers) else {
        return (StatusCode::UNAUTHORIZED, Json(None::<MeResponse>));
    };

    match verify_token(token, &config.jwt_secret) {
        Ok(claims) => (
            StatusCode::OK,
            Json(Some(MeResponse {
                id: claims.sub,
                email: claims.email,
            })),
        ),
        Err(_) => (StatusCode::UNAUTHORIZED, Json(None::<MeResponse>)),
    }
}

async fn exchange_google_code(
    config: &AuthConfig,
    code: &str,
) -> Result<GoogleTokenResponse, String> {
    config
        .client
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("client_id", config.google_client_id.as_str()),
            ("client_secret", config.google_client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", config.google_redirect_uri.as_str()),
        ])
        .send()
        .await
        .map_err(|e| {
            eprintln!("Google token request failed: {e:?}");
            "Google login token exchange failed.".to_string()
        })?
        .error_for_status()
        .map_err(|e| {
            eprintln!("Google token request rejected: {e:?}");
            "Google rejected the login callback.".to_string()
        })?
        .json::<GoogleTokenResponse>()
        .await
        .map_err(|e| {
            eprintln!("Google token response invalid: {e:?}");
            "Google returned an invalid login response.".to_string()
        })
}

async fn fetch_google_user(client: &Client, access_token: &str) -> Result<GoogleUserInfo, String> {
    client
        .get(GOOGLE_USERINFO_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| {
            eprintln!("Google userinfo request failed: {e:?}");
            "Could not read the Google account.".to_string()
        })?
        .error_for_status()
        .map_err(|e| {
            eprintln!("Google userinfo rejected: {e:?}");
            "Google rejected the account lookup.".to_string()
        })?
        .json::<GoogleUserInfo>()
        .await
        .map_err(|e| {
            eprintln!("Google userinfo invalid: {e:?}");
            "Google returned an invalid account response.".to_string()
        })
}

async fn upsert_user(
    pool: &PgPool,
    user: &GoogleUserInfo,
    refresh_token: Option<&str>,
) -> Result<(), sqlx::Error> {
    let display_name = user
        .name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(&user.email);

    sqlx::query(
        r#"INSERT INTO users (
               id,
               email,
               display_name,
               picture_url,
               google_refresh_token,
               updated_at,
               last_login_at
           )
           VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
           ON CONFLICT (id)
           DO UPDATE SET email = EXCLUDED.email,
                         display_name = EXCLUDED.display_name,
                         picture_url = EXCLUDED.picture_url,
                         google_refresh_token = COALESCE(
                             EXCLUDED.google_refresh_token,
                             users.google_refresh_token
                         ),
                         updated_at = NOW(),
                         last_login_at = NOW()"#,
    )
    .bind(&user.sub)
    .bind(&user.email)
    .bind(display_name)
    .bind(&user.picture)
    .bind(refresh_token)
    .execute(pool)
    .await
    .map(|_| ())
}

fn issue_token(
    jwt_secret: &str,
    user_id: &str,
    email: Option<&str>,
) -> Result<String, jsonwebtoken::errors::Error> {
    let expires_at = OffsetDateTime::now_utc() + Duration::days(TOKEN_TTL_DAYS);
    let claims = Claims {
        sub: user_id.to_string(),
        email: email.map(str::to_string),
        exp: expires_at.unix_timestamp() as usize,
        aud: TOKEN_AUDIENCE.to_string(),
        iss: TOKEN_ISSUER.to_string(),
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
}

fn bearer_from_headers(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

fn state_cookie_matches(headers: &HeaderMap, state: &str) -> bool {
    headers
        .get(COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|cookie| {
                let (name, value) = cookie.trim().split_once('=')?;
                (name == OAUTH_STATE_COOKIE).then_some(value)
            })
        })
        .is_some_and(|value| value == state)
}

fn trim_hash_redirect(web_url: &str) -> String {
    let without_hash = web_url.split('#').next().unwrap_or(web_url);
    if without_hash.ends_with('/') {
        without_hash.to_string()
    } else {
        format!("{without_hash}/")
    }
}

fn auth_error_redirect(web_url: &str, message: &str) -> Response {
    let redirect = format!(
        "{}?auth_error={}",
        trim_hash_redirect(web_url),
        message.replace(' ', "+")
    );
    Redirect::temporary(&redirect).into_response()
}
