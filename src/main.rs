#![allow(dead_code)]

mod todonts;

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, Path},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::net::SocketAddr;
use todonts::{InputTodont, Todont};

struct DBConn(sqlx::pool::PoolConnection<sqlx::Sqlite>);

#[async_trait]
impl<S> FromRequestParts<S> for DBConn
where
    SqlitePool: axum::extract::FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let pool = SqlitePool::from_ref(state);

        let conn = pool.acquire().await.unwrap();

        Ok(Self(conn))
    }
}

async fn get_all(DBConn(mut conn): DBConn) -> (StatusCode, Json<Vec<Todont>>) {
    let todonts = sqlx::query_as!(Todont, "SELECT * FROM todonts")
        .fetch_all(&mut *conn)
        .await
        .unwrap();

    (StatusCode::OK, Json(todonts))
}

async fn get_one(
    DBConn(mut conn): DBConn,
    Path(id): Path<i64>,
) -> (StatusCode, Json<Option<Todont>>) {
    let todont = sqlx::query_as!(Todont, "SELECT * FROM todonts where id=$1", id)
        .fetch_optional(&mut *conn)
        .await
        .unwrap();

    (StatusCode::OK, Json(todont))
}

async fn create(DBConn(mut conn): DBConn, Json(payload): Json<InputTodont>) -> StatusCode {
    todonts::create_todont(&mut conn, payload).await.unwrap();
    StatusCode::OK
}

async fn delete(DBConn(mut conn): DBConn, Path(id): Path<i64>) -> StatusCode {
    todonts::delete_todont(&mut conn, id).await.unwrap();
    StatusCode::OK
}

async fn update(
    DBConn(mut conn): DBConn,
    Path(id): Path<i64>,
    Json(todont): Json<InputTodont>,
) -> StatusCode {
    todonts::update_todont(
        &mut conn,
        Todont {
            id,
            description: todont.description,
            done: todont.done,
        },
    )
    .await
    .unwrap();
    StatusCode::OK
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    // let mut connection = SqliteConnection::connect("sqlite://sqeel.db").await?;
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite://sqeel.db")
        .await?;

    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(get_all).post(create))
        .route("/:id", get(get_one).put(update).delete(delete))
        .with_state(pool);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
