#![allow(clippy::result_large_err)] // tonic services conventionally return tonic::Status.

use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use std::sync::Arc;
use tokio::net::TcpListener;
use tonic::transport::Server;
use tonic_reflection::server::Builder as ReflectionBuilder;

mod auth;
mod calendar;
mod interceptor;
mod notes;
mod todos;
mod users;

pub mod glum {
    tonic::include_proto!("glum");

    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("glum_descriptor");
}

use glum::notes_server::NotesServer;
use glum::todos_server::TodosServer;
use glum::users_server::UsersServer;
use interceptor::auth_interceptor;
use notes::Notes;
use todos::Todos;
use users::Users;

#[tokio::main]
async fn main() {
    let pool = connect_pool().await;
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let jwt_secret = Arc::new(
        std::env::var("APP_JWT_SECRET")
            .unwrap_or_else(|_| "dev-only-change-me-at-deploy-time".to_string()),
    );

    let interceptor = auth_interceptor(jwt_secret.clone());
    let users = Users::new(pool.clone());
    let auth_router = auth::router(auth::AuthConfig::from_env(pool.clone(), jwt_secret));
    let auth_listener = TcpListener::bind("0.0.0.0:3001").await.unwrap();

    let reflection = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(glum::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    let grpc_server = Server::builder()
        .add_service(reflection)
        .add_service(NotesServer::with_interceptor(
            Notes::new(pool.clone()),
            interceptor.clone(),
        ))
        .add_service(TodosServer::with_interceptor(
            Todos::new(pool),
            interceptor.clone(),
        ))
        .add_service(UsersServer::with_interceptor(users, interceptor))
        .serve("0.0.0.0:3000".parse().unwrap());
    let auth_server = axum::serve(auth_listener, auth_router);

    tokio::select! {
        result = grpc_server => result.unwrap(),
        result = auth_server => result.unwrap(),
    }
}

async fn connect_pool() -> PgPool {
    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        return PgPool::connect(&database_url).await.unwrap();
    }

    let host = std::env::var("PGHOST").unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("PGPORT")
        .ok()
        .and_then(|port| port.parse::<u16>().ok())
        .unwrap_or(5432);
    let user = std::env::var("PGUSER").unwrap_or_else(|_| "glum".to_string());
    let password = std::env::var("PGPASSWORD").unwrap_or_default();
    let database = std::env::var("PGDATABASE").unwrap_or_else(|_| "glum".to_string());

    let options = PgConnectOptions::new()
        .host(&host)
        .port(port)
        .username(&user)
        .password(&password)
        .database(&database)
        .ssl_mode(PgSslMode::Disable);

    PgPoolOptions::new().connect_with(options).await.unwrap()
}
