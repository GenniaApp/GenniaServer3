#[allow(warnings, unused)]
mod game;
mod prisma;
mod routes;

use axum::{extract::Extension, Router};
use game::{handle_connection, RoomPoolStore};
use prisma::PrismaClient;
use socketioxide::SocketIo;
use std::{env, sync::Arc};
use tracing::info;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing::subscriber::set_global_default(FmtSubscriber::default())?;
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    let db_socket = Arc::new(PrismaClient::_builder().build().await.unwrap());
    let db_router = Arc::clone(&db_socket);

    let room_pool = RoomPoolStore::default();

    let (layer, io) = SocketIo::builder()
        .with_state(db_socket)
        .with_state(room_pool)
        .build_layer();

    // Register a handler for the default namespace
    io.ns("/", handle_connection);

    let app = Router::new()
        .nest("/api", routes::create_route())
        .layer(Extension(db_router))
        .layer(layer);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    info!("GenniaServer v3 running on http://0.0.0.0:{}", port);

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
