#![allow(dead_code)]

mod todonts;

use anyhow::Result;
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
use tower_http::{classify::ServerErrorsFailureClass, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

#[tracing::instrument]
async fn get_all(DBConn(mut conn): DBConn) -> (StatusCode, Json<Vec<Todont>>) {
    let todonts = sqlx::query_as!(Todont, "SELECT * FROM todonts")
        .fetch_all(&mut *conn)
        .await
        .unwrap();

    (StatusCode::OK, Json(todonts))
}

#[tracing::instrument]
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

#[tracing::instrument]
async fn create(DBConn(mut conn): DBConn, Json(payload): Json<InputTodont>) -> StatusCode {
    todonts::create_todont(&mut conn, payload).await.unwrap();
    StatusCode::OK
}

#[tracing::instrument]
async fn delete(DBConn(mut conn): DBConn, Path(id): Path<i64>) -> StatusCode {
    todonts::delete_todont(&mut conn, id).await.unwrap();
    StatusCode::OK
}

#[tracing::instrument]
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
async fn main() -> Result<()> {
    dotenvy::dotenv()?;

    let database_url = std::env::var("DATABASE_URL").unwrap_or(String::from("sqlite://default.db"));
    let environment_mode = std::env::var("ENV").unwrap_or(String::from("DEV"));
    let addr = SocketAddr::from((
        [0, 0, 0, 0],
        std::env::var("PORT")
            .unwrap_or(String::from("3000"))
            .parse::<u16>()?,
    ));

    println!(
        "hello there running the server: {}, {}",
        database_url, environment_mode,
    );

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                "rustodont=debug,tower_http=debug,axum::rejection=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // let mut connection = SqliteConnection::connect("sqlite://sqeel.db").await?;
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        // .connect("sqlite://sqeel.db")
        .connect(&database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let app = Router::new()
        .route("/", get(get_all).post(create))
        .route("/:id", get(get_one).put(update).delete(delete))
        .with_state(pool)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &axum::http::Request<_>| {
                    // Log the matched route's path (with placeholders not filled in).
                    // Use request.uri() or OriginalUri if you want the real path.
                    let matched_path = request
                        .extensions()
                        .get::<axum::extract::MatchedPath>()
                        .map(axum::extract::MatchedPath::as_str);

                    tracing::info_span!(
                        "http_request",
                        method = ?request.method(),
                        matched_path,
                        some_other_field = tracing::field::Empty,
                    )
                })
                .on_request(|_request: &axum::http::Request<_>, _span: &tracing::Span| {
                    // You can use `_span.record("some_other_field", value)` in one of these
                    // closures to attach a value to the initially empty field in the info_span
                    // created above.
                })
                .on_response(
                    |_response: &axum::response::Response,
                     _latency: std::time::Duration,
                     _span: &tracing::Span| {
                        // ...
                    },
                )
                .on_failure(
                    |_error: ServerErrorsFailureClass,
                     _latency: std::time::Duration,
                     _span: &tracing::Span| {
                        // ...
                    },
                ),
        );

    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
