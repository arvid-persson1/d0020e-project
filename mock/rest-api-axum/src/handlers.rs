use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};

use serde::{Deserialize, Serialize};

use axum_serde::Xml;

use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
/// Type for the supported book formats
pub(crate) enum BookFormatType {
    /// Format for PDF
    Pdf,
    /// Format for docx (Word)
    Docx,
    /// Format for Epub
    Epub,
    /// Format for Hardcover
    Hardcover,
    /// Format for Paperback
    Paperback,
    /// Pocket edition
    Pocket,
}

///Struct for book query search parameters
#[derive(Deserialize)]
pub(crate) struct BookSearch {
    /// Isbn query parameter
    isbn: Option<String>,
    /// Title query parameter
    title: Option<String>,
    /// Author query parameter
    author: Option<String>,
    /// Format query parameter.
    format: Option<BookFormatType>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename = "book")]
/// The Book type
pub(crate) struct Book {
    /// The book title
    pub(crate) title: String,
    /// The book author
    pub(crate) author: String,
    /// The book format
    pub(crate) format: BookFormatType,
    /// The book isbn
    pub(crate) isbn: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "books")]
/// A `BookList` type that creates a root element for the XML output
pub(crate) struct BookList {
    #[serde(rename = "book")]
    /// The books inside the book list
    pub(crate) books: Vec<Book>,
}

#[derive(Debug, Clone)]
/// The Appstate
pub(crate) struct AppState {
    /// The contents of `Appstate`
    pub(crate) books: Arc<Mutex<Vec<Book>>>,
}

/// Fetches a list of all books
///
///# Errors
///
/// Returns a `500 Internal Server Error` to the client if the `Appstate` mutex is poisoned.
#[inline]
pub(crate) async fn get_books(
    State(state): State<Arc<AppState>>,
    Query(params): Query<BookSearch>,
) -> Result<impl IntoResponse, StatusCode> {
    let filtered_books = {
        let books = state.books.lock().map_err(|e| {
            eprintln!("Internal Error: Mutex was poisoned: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        books
            .iter()
            .filter(|book| {
                let match_author = params
                    .author
                    .as_ref()
                    .is_none_or(|q| book.author.contains(q));

                let match_title = params.title.as_ref().is_none_or(|q| book.title.contains(q));

                let match_isbn = params.isbn.as_ref().is_none_or(|q| book.isbn == *q);

                let match_format = params.format.as_ref().is_none_or(|q| book.format == *q);

                match_author && match_title && match_isbn && match_format
            })
            .cloned()
            .collect::<Vec<Book>>()
    };

    Ok((
        StatusCode::OK,
        Xml(BookList {
            books: filtered_books,
        }),
    ))
}

/// Fetches a book by isbn (id)
///
///# Errors
///
/// Returns a `500 Internal Server Error` to the client if the `Appstate` mutex is poisoned.
/// Returns a `404 Not Found Error` to the client if it does not find a book with the given isbn.
#[inline]
pub(crate) async fn get_book(
    State(state): State<Arc<AppState>>,
    Query(params): Query<BookSearch>,
) -> Result<impl IntoResponse, StatusCode> {
    let book_option = {
        let books_guard = state
            .books
            .lock()
            .map_err(|_err| StatusCode::INTERNAL_SERVER_ERROR)?;

        books_guard
            .iter()
            .find(|book| {
                let matches_isbn = params.isbn.as_ref().is_none_or(|q| q == &book.isbn);
                let matches_author = params.author.as_ref().is_none_or(|q| q == &book.author);
                let matches_title = params.title.as_ref().is_none_or(|q| q == &book.title);
                let matches_format = params.format.as_ref().is_none_or(|q| q == &book.format);

                matches_isbn && matches_author && matches_title && matches_format
            })
            .cloned()
    };

    book_option.map_or(Err(StatusCode::NOT_FOUND), |book| Ok(Xml(book)))
}

/// Creates a new book
///
/// Returns the Statuscode CREATED when successfully creating a new book
///
///# Errors
///
/// Returns a `500 Internal Server Error` to the client if the `AppState` Mutex gets poisoned
/// (if for example another thread panics while holding the mutex lock)
#[inline]
pub(crate) async fn add_book(
    State(state): State<Arc<AppState>>,
    Xml(new_book): Xml<Book>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .books
        .lock()
        .map_err(|_err| {
            eprintln!("ERROR: Mutex poisoned while creating book");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .push(new_book.clone());

    Ok((StatusCode::CREATED, Xml(new_book)))
}
