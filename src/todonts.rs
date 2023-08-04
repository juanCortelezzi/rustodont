use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteConnection;

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct Todont {
    pub id: i64,
    pub description: String,
    pub done: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InputTodont {
    pub description: String,
    pub done: bool,
}

pub async fn update_todont(conn: &mut SqliteConnection, todont: Todont) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update todonts set description=$1, done=$2 where id=$3",
        todont.description,
        todont.done,
        todont.id,
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn create_todont(
    conn: &mut SqliteConnection,
    input_todont: InputTodont,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "insert into todonts (description, done) values ($1, $2)",
        input_todont.description,
        input_todont.done,
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn delete_todont(conn: &mut SqliteConnection, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!("delete from todonts where id=$1", id)
        .execute(conn)
        .await?;

    Ok(())
}
