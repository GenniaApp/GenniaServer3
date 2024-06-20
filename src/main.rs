#[allow(warnings, unused)]
mod game;
mod prisma;
mod routes;

use axum::{extract::Extension, Router};
use game::RoomPool;
use prisma::PrismaClient;
use socketioxide::{extract::SocketRef, SocketIo};
use std::{collections::BTreeMap, env, sync::Arc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    let prisma_client = Arc::new(PrismaClient::_builder().build().await.unwrap());

    let room_pool = RoomPool {
        pool: BTreeMap::new(),
        max_room_count: env::var("MAX_ROOM_COUNT")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .unwrap(),
    };

    let (layer, io) = SocketIo::new_layer();

    // Register a handler for the default namespace
    io.ns("/", |s: SocketRef| {
        // For each "message" event received, send a "message-back" event with the "Hello World!" event
        s.on("message", |s: SocketRef| {
            s.emit("message-back", "Hello World!").ok();
        });
    });

    let app = Router::new()
        .nest("/api", routes::create_route())
        .layer(Extension(prisma_client))
        .layer(Extension(room_pool))
        .layer(layer);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    println!("GenniaServer v3 running on http://0.0.0.0:{}", port);

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
