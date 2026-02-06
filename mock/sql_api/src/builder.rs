use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json,
    Router,
};

use crate::models::Book;
use crate::db::{DbPool, DbConnection};
use diesel::prelude::*;
use crate::schema::books as schema_books;
use diesel::result::Error;

pub(crate) async fn get_books_list(State(pool): State<DbPool>) -> Result<Json<Vec<Book>>, (StatusCode, String)> {
  use schema_books::dsl::books;

  let mut connection: DbConnection = pool.get().map_err(|err| {
  (StatusCode::INTERNAL_SERVER_ERROR, format!("Database Connection Error: {err}"))
  })?;

  let res = books
    .load::<Book>(&mut connection)
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

  Ok(Json(res))
}

pub(crate) async fn get_book(State(pool): State<DbPool>, Path(book_isbn): Path<String>) -> Result<Json<Book>, (StatusCode, String)> {
  use schema_books::dsl::books;

  let mut connection: DbConnection = pool.get().map_err(|err| {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("Database Connection Error: {err}"))
  })?;

  let res = books
    .find(book_isbn)
    .first::<Book>(&mut connection)
    .map_err(|err| {
        if err == Error::NotFound {
            (StatusCode::NOT_FOUND, "Could not find book".to_owned())
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
        }
    })?;

  Ok(Json(res))
}

pub(crate) fn build_app(pool: DbPool) -> Router {
  Router::<DbPool>::new()
    .route("/books", get(get_books_list))
    .route("/books/:isbn", get(get_book))
    .with_state(pool)
}
