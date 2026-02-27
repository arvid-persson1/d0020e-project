use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::get,
};

use crate::{db::{DbConnection, DbPool}, models::BookSearch};
use crate::models::Book;
use crate::schema::books as schema_books;
use diesel::prelude::*;
use diesel::result::Error;
/// Fetches a list of all books.
/// # Errors
/// Returns a `(StatusCode, String)` tuple if:
/// The application cannot acquire a connection from the pool (500).
/// An underlying SQL query error occurs (500).
pub(crate) async fn get_books(
    State(pool): State<DbPool>,
    Query(params): Query<BookSearch>,
) -> Result<Json<Vec<Book>>, (StatusCode, String)> {
    use schema_books::dsl::{books, isbn, title, author, format};

    let mut connection: DbConnection = pool.get().map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database Connection Error: {err}"),
        )
    })?;

    let mut query = books.into_boxed();

    if let Some(req_isbn) = params.isbn {
      query = query.filter(isbn.eq(req_isbn));
    }


    if let Some(req_title) = params.title {
      let search_pattern = format!("%{req_title}%");
      query = query.filter(title.ilike(search_pattern));
    }

    if let Some(req_author) = params.author {
      let search_pattern = format!("%{req_author}%");
      query = query.filter(author.ilike(search_pattern));
    }

    if let Some(req_format) = params.format {
      query = query.filter(format.eq(req_format));
    }

    let res = query
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
    Query(params): Query<BookSearch>,
) -> Result<Json<Book>, (StatusCode, String)> {
    use schema_books::dsl::{books, title, isbn, author, format};

    let mut connection: DbConnection = pool.get().map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database Connection Error: {err}"),
        )
    })?;

    let mut query = books.into_boxed();

    if let Some(req_isbn) = params.isbn {
      query = query.filter(isbn.eq(req_isbn));
    }

    if let Some(req_title) = params.title {
      let search_pattern = format!("%{req_title}%");
      query = query.filter(title.ilike(search_pattern));
    }

    if let Some(req_author) = params.author {
      let search_pattern = format!("%{req_author}%");
      query = query.filter(author.ilike(search_pattern));
    }

    if let Some(req_format) = params.format {
      query = query.filter(format.eq(req_format));
    }

    let res = query.first::<Book>(&mut connection);

    println!("--> HANDLER: get_book was called!");

    match res {
      Ok(book) => Ok(Json(book)),
      Err(Error::NotFound) => {
        Err((StatusCode::NOT_FOUND, "Could not find book".to_owned()))
      }
    Err(err) => Err((StatusCode::INTERNAL_SERVER_ERROR, err.to_string())),
    }

}
/// Builder function for the Router app.
pub(crate) fn build_app(pool: DbPool) -> Router {
    Router::<DbPool>::new()
        .route("/books", get(get_books))
        .route("/book", get(get_book))
        .with_state(pool)
}
