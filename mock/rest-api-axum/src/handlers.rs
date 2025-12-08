use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use serde::{Deserialize, Serialize};

use axum_serde::Xml;

use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize, Clone)]
enum BookFormatType {
    AudiobookFormat,
    EBook,
    GraphicNovel,
    Hardcover,
    Paperback,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename = "book")]
pub struct Book {
    title: String,
    author: String,
    format: BookFormatType,
    isbn: String,
}

#[derive(Debug, Serialize)]
#[serde(rename = "books")]
pub struct BookList {
    #[serde(rename = "book")]
    pub books: Vec<Book>,
}

#[derive(Debug, Clone)]
pub struct APPState {
    pub books: Arc<Mutex<Vec<Book>>>,
}

//Get handler for all books
pub async fn get_books(State(state): State<Arc<APPState>>) -> impl IntoResponse {
    let books_guard = state.books.lock().expect("Mutex poisoned");
    let books_copy = books_guard.clone();

    (StatusCode::OK, Xml(BookList { books: books_copy }))
}

//Get handler for a book by isbn (id)
pub async fn get_book(
    State(state): State<Arc<APPState>>,
    Path(isbn): Path<String>,
) -> impl IntoResponse {
    let books_guard = state.books.lock().expect("Mutex poisoned");

    let book_ref = books_guard.iter().find(|b| b.isbn == isbn);

    match book_ref {
        Some(book) => Ok(Xml(book.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn add_book(
    State(state): State<Arc<APPState>>,
    Xml(new_book): Xml<Book>,
) -> impl IntoResponse {
    let mut books_guard = state.books.lock().expect("Mutex poisoned");

    books_guard.push(new_book.clone());

    (StatusCode::CREATED, Xml(new_book))
}
