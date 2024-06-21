#[allow(warnings, unused)]
use axum::{body::Body, extract::Query};
use axum::{
    extract::{Json, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Router,
};
use axum_macros::debug_handler;
use custom_map_data::{map_tiles_data, views};
use player;
use players_starred_maps;
use prisma_client_rust::{
    prisma_errors::query_engine::{RecordNotFound, UniqueKeyViolation},
    Direction, QueryError,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, vec};
use uuid::Uuid;

use crate::prisma::*;

type PrismaState = Extension<Arc<PrismaClient>>;
type AppResult<T> = Result<T, AppError>;
type AppJsonResult<T> = AppResult<Json<T>>;

/*

/api/register => POST
/api/rooms => GET
/api/create_room => POST
/api/replays/:replay_id => GET
/api/maps => GET
/api/maps/new => GET
/api/maps/best => GET
/api/maps/hot => GET
/api/maps/search => GET
/api/maps/starred => GET
/api/map/:map_id => GET, PUT, DELETE
/api/map/:map_id/toggle_star => POST

*/
pub fn create_route() -> Router {
    Router::new()
        .route("/api/register", post(handle_player_register))
        .route("/replays/:replay_id", get(handle_replays_get))
        .route("/maps", get(handle_all_maps_get))
        .route("/maps/new", get(handle_new_maps_get))
        .route("/maps/best", get(handle_best_maps_get))
        .route("/maps/hot", get(handle_hot_maps_get))
        .route("/maps/search", get(handle_search_maps))
        .route("/maps/starred", get(handle_starred_maps_get))
        .route(
            "/map/:map_id",
            get(handle_map_get)
                .put(handle_map_put)
                .delete(handle_map_delete),
        )
        .route("/map/:map_id/toggle_star", post(handle_star_map))
}

#[derive(Deserialize)]
struct RegisterRequest {
    username: String,
    email: String,
}

#[derive(Serialize)]
struct RegisterResponse {
    success: bool,
    player_id: String,
    reason: String,
}

#[debug_handler]
async fn handle_player_register(
    Extension(db): PrismaState,
    Json(RegisterRequest { username, email }): Json<RegisterRequest>,
) -> Response<Body> {
    match db
        .player()
        .find_many(vec![player::username::equals(username.clone())])
        .exec()
        .await
    {
        Ok(_) => {
            return Json(RegisterResponse {
                success: false,
                player_id: "".to_string(),
                reason: "The username was token.".to_string(),
            })
            .into_response()
        }
        Err(_) => match db.player().create(username, email, vec![]).exec().await {
            Ok(data) => {
                return Json(RegisterResponse {
                    success: true,
                    player_id: data.id,
                    reason: "".to_string(),
                })
                .into_response()
            }
            Err(err) => {
                return Json(RegisterResponse {
                    success: false,
                    player_id: "".to_string(),
                    reason: err.to_string(),
                })
                .into_response()
            }
        },
    }
}

#[debug_handler]
async fn handle_replays_get(
    Extension(db): PrismaState,
    Path(replay_id): Path<Uuid>,
) -> AppJsonResult<replay::Data> {
    let replay = db
        .replay()
        .find_unique(replay::id::equals(replay_id.to_string()))
        .exec()
        .await?;

    Ok(Json::from(replay.unwrap()))
}

custom_map_data::select!(map_selected {
    id
    name
    width
    height
    creator
    description
    created_at
    views
    star_count
});

#[debug_handler]
async fn handle_all_maps_get(Extension(db): PrismaState) -> AppJsonResult<Vec<map_selected::Data>> {
    let maps = db
        .custom_map_data()
        .find_many(vec![])
        .select(map_selected::select())
        .exec()
        .await?;

    Ok(Json::from(maps))
}

#[debug_handler]
async fn handle_new_maps_get(Extension(db): PrismaState) -> AppJsonResult<Vec<map_selected::Data>> {
    let maps = db
        .custom_map_data()
        .find_many(vec![])
        .order_by(custom_map_data::created_at::order(Direction::Asc))
        .select(map_selected::select())
        .exec()
        .await?;

    Ok(Json::from(maps))
}

#[debug_handler]
async fn handle_best_maps_get(
    Extension(db): PrismaState,
) -> AppJsonResult<Vec<map_selected::Data>> {
    let maps = db
        .custom_map_data()
        .find_many(vec![])
        .order_by(custom_map_data::star_count::order(Direction::Desc))
        .select(map_selected::select())
        .exec()
        .await?;

    Ok(Json::from(maps))
}

#[debug_handler]
async fn handle_hot_maps_get(Extension(db): PrismaState) -> AppJsonResult<Vec<map_selected::Data>> {
    let maps = db
        .custom_map_data()
        .find_many(vec![])
        .order_by(custom_map_data::views::order(Direction::Desc))
        .select(map_selected::select())
        .exec()
        .await?;

    Ok(Json::from(maps))
}

#[debug_handler]
async fn handle_search_maps(
    Extension(db): PrismaState,
    Query(params): Query<HashMap<String, String>>,
) -> AppJsonResult<Vec<map_selected::Data>> {
    let search_term = params.get("q").unwrap();

    let maps = db
        .custom_map_data()
        .find_many(vec![
            custom_map_data::name::contains(search_term.to_string()),
            custom_map_data::id::equals(search_term.to_string()),
        ])
        .select(map_selected::select())
        .exec()
        .await?;

    Ok(Json::from(maps))
}

#[debug_handler]
async fn handle_starred_maps_get(
    Extension(db): PrismaState,
    Query(params): Query<HashMap<String, String>>,
) -> AppJsonResult<Vec<map_selected::Data>> {
    let user_id = params.get("user_id").unwrap().to_string();

    let starred_map_relations = db
        .player()
        .find_unique(player::id::equals(user_id))
        .select(player::select!({ star_maps }))
        .exec()
        .await?
        .unwrap()
        .star_maps;
    let starred_map_ids = starred_map_relations
        .iter()
        .map(|x| x.map_id.clone())
        .collect();
    let starred_maps = db
        .custom_map_data()
        .find_many(vec![custom_map_data::id::in_vec(starred_map_ids)])
        .select(map_selected::select())
        .exec()
        .await?;

    Ok(Json::from(starred_maps))
}

#[debug_handler]
async fn handle_map_get(
    Extension(db): PrismaState,
    Path(map_id): Path<Uuid>,
) -> AppJsonResult<custom_map_data::Data> {
    let map = db
        .custom_map_data()
        .find_unique(custom_map_data::id::equals(map_id.to_string()))
        .exec()
        .await?
        .unwrap();
    db.custom_map_data()
        .update(
            custom_map_data::id::equals(map_id.to_string()),
            vec![views::increment(1)],
        )
        .exec()
        .await?;

    Ok(Json::from(map))
}

#[derive(Deserialize)]
struct MapRequest {
    map_tile_data: String,
}

#[debug_handler]
async fn handle_map_put(
    Extension(db): PrismaState,
    Path(map_id): Path<Uuid>,
    Json(input): Json<MapRequest>,
) -> AppJsonResult<custom_map_data::Data> {
    let updated_map = db
        .custom_map_data()
        .update(
            custom_map_data::id::equals(map_id.to_string()),
            vec![map_tiles_data::set(input.map_tile_data)],
        )
        .exec()
        .await?;

    Ok(Json::from(updated_map))
}

#[debug_handler]
async fn handle_map_delete(
    Extension(db): PrismaState,
    Path(map_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    db.custom_map_data()
        .delete(custom_map_data::id::equals(map_id.to_string()))
        .exec()
        .await?;

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
struct StarRequest {
    user_id: String,
    map_id: String,
}

#[debug_handler]
async fn handle_star_map(
    Extension(db): PrismaState,
    Json(StarRequest { user_id, map_id }): Json<StarRequest>,
) -> AppResult<StatusCode> {
    match db
        .players_starred_maps()
        .find_unique(players_starred_maps::player_id_map_id(
            user_id.clone(),
            map_id.clone(),
        ))
        .exec()
        .await
    {
        Ok(_) => {
            db._transaction()
                .run(|db| async move {
                    db.players_starred_maps()
                        .delete(players_starred_maps::player_id_map_id(
                            user_id.clone(),
                            map_id.clone(),
                        ))
                        .exec()
                        .await?;
                    db.custom_map_data()
                        .update(
                            custom_map_data::id::equals(map_id.clone()),
                            vec![custom_map_data::star_count::decrement(1)],
                        )
                        .exec()
                        .await
                })
                .await?;
            return Ok(StatusCode::OK);
        }
        Err(_) => {
            db._transaction()
                .run(|db| async move {
                    db.players_starred_maps()
                        .create(
                            player::id::equals(user_id.clone()),
                            custom_map_data::id::equals(map_id.clone()),
                            vec![],
                        )
                        .exec()
                        .await?;
                    db.custom_map_data()
                        .update(
                            custom_map_data::id::equals(map_id.clone()),
                            vec![custom_map_data::star_count::increment(1)],
                        )
                        .exec()
                        .await
                })
                .await?;
            return Ok(StatusCode::OK);
        }
    }
}

enum AppError {
    PrismaError(QueryError),
    NotFound,
}

impl From<QueryError> for AppError {
    fn from(error: QueryError) -> Self {
        match error {
            e if e.is_prisma_error::<RecordNotFound>() => AppError::NotFound,
            e => AppError::PrismaError(e),
        }
    }
}

// This centralizes all different errors from our app in one place
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::PrismaError(error) if error.is_prisma_error::<UniqueKeyViolation>() => {
                StatusCode::CONFLICT
            }
            AppError::PrismaError(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound => StatusCode::NOT_FOUND,
        };

        status.into_response()
    }
}
