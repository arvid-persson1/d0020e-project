//! Libary file for the app construction logic
/// Module for implementing the API handlers and types
// use tokio::net::TcpListener;
// use tokio as _;
use axum::{
    Router,
    routing::{get, post},
};

use crate::handlers::{AppState, Book, BookFormatType, add_book, get_book, get_books};
use std::sync::{Arc, Mutex};

/// Builder function for the Router app
#[inline]
pub fn build_app() -> Router {
    let books: Vec<Book> = vec![
        Book {
            title: "Pride and Prejudice".to_owned(),
            author: "Jane Austen".to_owned(),
            isbn: "9780141439518".to_owned(),
            format: BookFormatType::Paperback,
        },
        Book {
            title: "Moby Dick".to_owned(),
            author: "Herman Melville".to_owned(),
            isbn: "9781503280786".to_owned(),
            format: BookFormatType::Hardcover,
        },
        Book {
            title: "Nineteen Eighty-Four".to_owned(),
            author: "George Orwell".to_owned(),
            isbn: "9780141036144".to_owned(),
            format: BookFormatType::Paperback,
        },
        Book {
            title: "The Last Wish: Introducing the Witcher".to_owned(),
            author: "Andrzej Sapkowski".to_owned(),
            isbn: "9780316497541".to_owned(),
            format: BookFormatType::Paperback,
        },
    ];

    let state: Arc<AppState> = Arc::new(AppState {
        books: Arc::new(Mutex::new(books)),
    });

    let app: Router = Router::new()
        .route("/books", get(get_books))
        .route("/book", get(get_book))
        .route("/books", post(add_book))
        .with_state(state);

    app
}
