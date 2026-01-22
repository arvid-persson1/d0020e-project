//!Libary file for the app construction logic
///Module for implementing the API handlers and types
//use tokio::net::TcpListener;
//use tokio as _;
use axum::{
    Router,
    routing::{get, post},
};

use std::sync::{Arc, Mutex};

use crate::handlers::{AppState, add_book, get_book, get_books};

///Builder function for the Router app
#[inline]
pub fn build_app() -> Router {
    let books = vec![];

    let state: Arc<AppState> = Arc::new(AppState {
        books: Arc::new(Mutex::new(books)),
    });

    let app: Router = Router::new()
        .route("/books", get(get_books))
        .route("/books{isbn}", get(get_book))
        .route("/books", post(add_book))
        .with_state(state);

    app
}
