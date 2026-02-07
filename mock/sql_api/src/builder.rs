use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};

use crate::db::{DbConnection, DbPool};
use crate::models::Book;
use crate::schema::books as schema_books;
use diesel::prelude::*;
use diesel::result::Error;
/// Fetches a list of all books.
/// # Errors
/// Returns a `(StatusCode, String)` tuple if:
/// The application cannot acquire a connection from the pool (500).
/// An underlying SQL query error occurs (500).
pub(crate) async fn get_books_list(
    State(pool): State<DbPool>,
) -> Result<Json<Vec<Book>>, (StatusCode, String)> {
    use schema_books::dsl::books;

    let mut connection: DbConnection = pool.get().map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database Connection Error: {err}"),
        )
    })?;

    let res = books
        .load::<Book>(&mut connection)
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    Ok(Json(res))
}

/// Fetches a book by its ISBN.
/// # Errors
/// Returns a tuple `(StatusCode, String)` if:
/// -The database connection fails (500 Internal Server Error).
/// -The book does not exist (404 Not Found).
pub(crate) async fn get_book(
    State(pool): State<DbPool>,
    Path(book_isbn): Path<String>,
) -> Result<Json<Book>, (StatusCode, String)> {
    use schema_books::dsl::books;

    let mut connection: DbConnection = pool.get().map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database Connection Error: {err}"),
        )
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
/// Builder function for the Router app.
pub(crate) fn build_app(pool: DbPool) -> Router {
    Router::<DbPool>::new()
        .route("/books", get(get_books_list))
        .route("/books/:isbn", get(get_book))
        .with_state(pool)
}
