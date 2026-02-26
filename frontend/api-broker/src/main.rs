//! HTTP API broker that aggregates book data from multiple REST sources.
//! Also maintains a local `SQLite` database for adding books independently of the broker.

use axum::{
    Json as AxumJson, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use broker::{
    Broker,
    connector::Source as _,
    encode::json::Json as BrokerJson,
    encode::xml::Xml,
    query::Queryable,
    rest::{Build as _, Builder as RestBuilder},
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;
use tokio::{net::TcpListener, sync::Mutex};
use tower_http::cors::{Any, CorsLayer};

/// Shared application state.
#[derive(Clone)]
struct AppState {
    /// Broker aggregating external REST sources.
    broker: Arc<Mutex<Broker<Book>>>,
    /// `SQLite` database connection for added books.
    db: SqlitePool,
}

/// Incoming JSON payload for `/query`.
#[derive(Deserialize)]
struct QueryRequest {
    /// Field to search by: `author`, `title`, or `isbn`.
    field: String,
    /// Value to match.
    value: String,
}

/// Representation of a book.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash, Queryable, FromRow)]
struct Book {
    /// Book title.
    title: String,
    /// Book author.
    author: String,
    /// ISBN identifier.
    isbn: String,
}

/// Entrypoint: sets up the broker, database, and HTTP server.
///
/// # Panics
/// Panics if any of the broker sources fail to initialize,
/// or if the database cannot be created/connected, or if
/// the TCP listener fails to bind.
#[tokio::main]
async fn main() {
    let mut broker = Broker::<Book>::new();

    // JSON source
    broker.add_source(Box::new(
        RestBuilder::new()
            .source_url("http://127.0.0.1:8080/books")
            .expect("Failed to create JSON broker source")
            .decoder(BrokerJson)
            .build(),
    ));

    // XML source
    broker.add_source(Box::new(
        RestBuilder::new()
            .source_url("http://127.0.0.1:1616/books")
            .expect("Failed to create XML broker source")
            .decoder(Xml)
            .build(),
    ));

    let db = SqlitePool::connect("sqlite://./books.db")
        .await
        .expect("Failed to connect to DB");

    let state = AppState {
        broker: Arc::new(Mutex::new(broker)),
        db: db.clone(),
    };

    // Create table if not exists
    let _ = sqlx::query(
        "
    CREATE TABLE IF NOT EXISTS books (
        title TEXT NOT NULL,
        author TEXT NOT NULL,
        isbn TEXT NOT NULL
    );
    ",
    )
    .execute(&db)
    .await
    .expect("Failed to create table");

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(|| async { "API Broker Running" }))
        .route("/query", post(query_handler))
        .route("/books", post(add_book))
        .layer(cors)
        .with_state(state);

    println!("Running at http://localhost:3000");

    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Failed to bind TCP listener");

    axum::serve(listener, app)
        .await
        .expect("Failed to start HTTP server");
}

/// Handles POST `/query` by fetching from broker + local DB.
async fn query_handler(
    State(state): State<AppState>,
    AxumJson(payload): AxumJson<QueryRequest>,
) -> impl IntoResponse {
    // Build broker query
    let query = match payload.field.as_str() {
        "author" => Book::author().eq(&payload.value),
        "title" => Book::title().eq(&payload.value),
        "isbn" => Book::isbn().eq(&payload.value),
        _ => return (StatusCode::BAD_REQUEST, AxumJson(Vec::<Book>::new())).into_response(),
    };

    let mut results = {
        let mut broker = state.broker.lock().await;
        broker.fetch_all(&query).await.unwrap_or_default()
    };

    // Fetch from SQLite
    let db_books = sqlx::query_as::<_, Book>(&format!(
        "SELECT title, author, isbn FROM books WHERE {} = ?",
        payload.field
    ))
    .bind(&payload.value)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    results.extend(db_books);

    (StatusCode::OK, AxumJson(results)).into_response()
}

/// Handles POST `/books` by inserting a new book into `SQLite DB`.
async fn add_book(
    State(state): State<AppState>,
    AxumJson(book): AxumJson<Book>,
) -> impl IntoResponse {
    let result = sqlx::query("INSERT INTO books (title, author, isbn) VALUES (?, ?, ?)")
        .bind(&book.title)
        .bind(&book.author)
        .bind(&book.isbn)
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => (StatusCode::CREATED, AxumJson("Book added")).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            AxumJson("Failed to add book"),
        )
            .into_response(),
    }
}
