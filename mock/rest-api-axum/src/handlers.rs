use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use serde::{Deserialize, Serialize};

use axum_serde::Xml;

use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize, Clone)]
///Type for the supported book formats
enum BookFormatType {
    ///Format for PDF
    Pdf,
    ///Format for docx (Word)
    Docx,
    ///Format for Epub
    Epub,
    ///Format for Hardcover
    Hardcover,
    ///Format for Paperback
    Paperback,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename = "book")]
///The Book type
pub struct Book {
    ///The book title
    title: String,
    ///The book author
    author: String,
    ///The book format
    format: BookFormatType,
    ///The book isbn
    isbn: String,
}

#[derive(Debug, Serialize)]
#[serde(rename = "books")]
///A `BookList` type that creates a root element for the XML output
pub struct BookList {
    #[serde(rename = "book")]
    ///The books inside the book list
    pub books: Vec<Book>,
}

#[derive(Debug, Clone)]
///The Appstate
pub struct AppState {
    ///The contents of `Appstate`
    pub books: Arc<Mutex<Vec<Book>>>,
}

///Fetches a list of all books
///
/// # Errors
///
///Returns a `500 Internal Server Error` to the client if the `Appstate` mutex is poisoned.
#[inline]
pub async fn get_books(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, StatusCode> {
    // map_err catches the "poison" error and converts it to a 500 code
    let books_vector = state
        .books
        .lock()
        .map_err(|_err| StatusCode::INTERNAL_SERVER_ERROR)?
        .clone();

    // We wrap the response in BookList to satisfy the XML root element requirement
    Ok(Xml(BookList {
        books: books_vector,
    }))
}

///Fetches a book by isbn (id)
///
/// # Errors
///
///Returns a `500 Internal Server Error` to the client if the `Appstate` mutex is poisoned.
///Returns a `404 Not Found Error` to the client if it does not find a book with the given isbn.
#[inline]
pub async fn get_book(
    State(state): State<Arc<AppState>>,
    Path(isbn): Path<String>,
) -> impl IntoResponse {
    let book_option = {
        // If the lock fails, this returns Err(500) immediately.
        let books_guard = state
            .books
            .lock()
            .map_err(|_err| StatusCode::INTERNAL_SERVER_ERROR)?;

        books_guard.iter().find(|b| b.isbn == isbn).cloned()
    };

    book_option.map_or(
        Err(StatusCode::NOT_FOUND), // If None (Not Found)
        |book| Ok(Xml(book)),       // If Some (Found)
    )
}

///Creates a new book
///
///Returns the Statuscode CREATED when successfully creating a new book
///
/// # Errors
///
///Returns a `500 Internal Server Error` to the client if the `AppState` Mutex gets poisoned
///(if for example another thread panics while holding the mutex lock)
#[inline]
pub async fn add_book(
    State(state): State<Arc<AppState>>,
    Xml(new_book): Xml<Book>,
) -> impl IntoResponse {
    state.books.lock().map_or_else(
        |_| {
            // The Mutex is poisoned (another thread panicked while holding it).
            // We log the error (optional) and return a 500 error to the client.
            eprintln!("ERROR: Mutex is poisoned!");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        },
        |mut books_guard| {
            // Success! We have the guard.
            books_guard.push(new_book.clone());
            // Return the success tuple wrapped in Ok()
            Ok((StatusCode::CREATED, Xml(new_book)))
        },
    )
}
