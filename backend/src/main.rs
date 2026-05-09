use tonic::transport::Server;
use sqlx::PgPool;

mod notes;

pub mod glum {
    tonic::include_proto!("glum");
}

use glum::notes_server::NotesServer;
use notes::Notes;

#[tokio::main]
async fn main() {
    let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap()).await.unwrap();

    Server::builder()
        .add_service(NotesServer::new(Notes::new(pool)))
        .serve("0.0.0.0:50051".parse().unwrap())
        .await
        .unwrap();
}
