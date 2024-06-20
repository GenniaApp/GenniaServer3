use axum::extract::Query;
#[allow(warnings, unused)]
use axum::{
    extract::{Json, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Extension, Router,
};
use axum_macros::debug_handler;
use map_diff::replay_id;
use prisma_client_rust::{
    chrono::{DateTime, Local},
    prisma_errors::query_engine::{RecordNotFound, UniqueKeyViolation},
    serde_json::Number,
    Direction, QueryError,
};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    vec,
};
use tokio::{select, sync::mpsc::error};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    game::{Room, RoomPool},
    prisma::*,
};

type RoomPoolState = Extension<RoomPool>;
type PrismaState = Extension<Arc<PrismaClient>>;
type AppResult<T> = Result<T, AppError>;
type AppJsonResult<T> = AppResult<Json<T>>;

/*

/api/rooms => GET
/api/create_room => POST
/api/replays/:replayId => GET
/api/maps => GET
/api/maps/new => GET
/api/maps/best => GET
/api/maps/hot => GET
/api/maps/search => GET
/api/maps/starred => GET
/api/map/:mapId => GET, PUT, DELETE
/api/map/:mapId/toggleStar => POST

*/
pub fn create_route() -> Router {
    Router::new()
        .route("/rooms", get(handle_rooms_get))
        .route("/create_room", post(handle_create_room))
        .route("/replays/:replayId", get(handle_replays_get))
        .route("/maps", get(handle_all_maps_get))
        .route("/maps/new", get(handle_new_maps_get))
        .route("/maps/best", get(handle_best_maps_get))
        .route("/maps/hot", get(handle_hot_maps_get))
        .route("/maps/search", get(handle_search_maps))
    // .route(
    //     "/user/:username",
    //     put(handle_user_put).delete(handle_user_delete),
    // )
    // .route("/comment", post(handle_comment_post))
}

async fn handle_rooms_get(
    Extension(room_pool): RoomPoolState,
) -> AppJsonResult<BTreeMap<String, Room>> {
    Ok(Json::from(room_pool.pool.clone()))
}

#[derive(Serialize)]
struct RoomCreateResult {
    success: bool,
    room_id: String,
    reason: &'static str,
}

#[debug_handler]
async fn handle_create_room(
    Extension(room_pool): RoomPoolState,
) -> AppJsonResult<RoomCreateResult> {
    match room_pool.create_room() {
        Ok(room_id) => {
            return Ok(Json(RoomCreateResult {
                success: true,
                room_id,
                reason: "",
            }))
        }
        Err(reason) => {
            return Ok(Json(RoomCreateResult {
                success: false,
                room_id: "".to_string(),
                reason,
            }))
        }
    };
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

/*
async fn handle_user_put(
    prisma: State,
    Path(username): Path<String>,
    Json(input): Json<UserRequest>,
) -> AppJsonResult<user::Data> {
    let updated_user = prisma
        .user()
        .update(
            user::username::equals(username),
            vec![
                user::username::set(input.username),
                user::email::set(input.email),
            ],
        )
        .exec()
        .await?;

    Ok(Json::from(updated_user))
}

async fn handle_user_delete(
    prisma: Database,
    Path(username): Path<String>,
) -> AppResult<StatusCode> {
    prisma
        .user()
        .delete(user::username::equals(username))
        .exec()
        .await?;

    Ok(StatusCode::OK)
}

async fn handle_comment_post(
    prisma: Database,
    Json(req): Json<CommentRequest>,
) -> AppJsonResult<comments::Data> {
    let comment = prisma
        .comments()
        .create(req.message, user::id::equals(req.user), vec![])
        .exec()
        .await?;

    Ok(Json::from(comment))
}
 */
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
