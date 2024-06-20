// Stops the client from outputting a huge number of warnings during compilation.
mod game;
#[allow(warnings, unused)]
mod prisma;

use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use prisma::PrismaClient;
use prisma_client_rust::NewClientError;
use socketioxide::{extract::SocketRef, SocketIo};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    let client: Result<PrismaClient, NewClientError> = PrismaClient::_builder().build().await;

    let (layer, io) = SocketIo::new_layer();

    // Register a handler for the default namespace
    io.ns("/", |s: SocketRef| {
        // For each "message" event received, send a "message-back" event with the "Hello World!" event
        s.on("message", |s: SocketRef| {
            s.emit("message-back", "Hello World!").ok();
        });
    });

    let app = axum::Router::new()
        .route("/ping", get(|| async { StatusCode::OK }))
        .route("/", get(|| async { "Hello, World!" }))
        .layer(layer);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
