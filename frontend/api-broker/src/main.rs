//! HTTP API broker that aggregates book data from multiple REST sources.
//! Also maintains a local `SQLite` database for adding books independently of the broker.

use axum::{
    Json as AxumJson, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use broker::connector::Source;
use broker::{
    Broker, SearchResult,
    connector::MemorySource,
    encode::json::Json as BrokerJson,
    encode::xml::Xml,
    query::Queryable,
    query::combinators::True,
    rest::{Build as _, Builder as RestBuilder},
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::collections::HashSet;
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
struct QueryCondition {
    /// Field to search by: `author`, `title`, or `isbn`.
    field: String,
    /// Value to match.
    value: String,
}

/// Request body for the `/query` endpoint.
#[derive(Deserialize)]
struct QueryRequest {
    /// List of conditions to apply.
    conditions: Vec<QueryCondition>,
    /// Logical operator used to combine conditions:
    /// `"and"` requires all conditions to match,
    /// `"or"` requires at least one.
    operator: String,
    /// Sources to search in. Empty = search all.
    sources: Vec<String>,
}

/// Representation of a book.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash, Queryable, FromRow, Clone)]
struct Book {
    /// Book title.
    title: String,
    /// Book author.
    author: String,
    /// ISBN identifier.
    isbn: String,
}

/// Payload for creating a new in-memory source.
#[derive(Deserialize)]
struct AddSourceRequest {
    /// Name of the source to create.
    name: String,
}

/// Payload for adding a book to a source.
#[derive(Deserialize)]
struct AddBookRequest {
    /// Book title
    title: String,
    /// Book author
    author: String,
    /// Book ISBN
    isbn: String,
    /// Target source name
    source: String,
}

/// Public representation of source
#[derive(Serialize)]
struct SourceInfo {
    /// Name of the source
    name: String,
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
    let memory = MemorySource::new();

    // JSON source
    broker.add_source(
        "JSON API",
        Box::new(
            RestBuilder::new()
                .source_url("http://127.0.0.1:8080/books")
                .expect("Failed to create JSON broker source")
                .decoder(BrokerJson)
                .build(),
        ),
    );

    // XML source
    broker.add_source(
        "XML API",
        Box::new(
            RestBuilder::new()
                .source_url("http://127.0.0.1:1616/books")
                .expect("Failed to create XML broker source")
                .decoder(Xml)
                .build(),
        ),
    );

    broker.add_source("Memory", Box::new(memory));

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
        .route("/sources", post(add_source_handler))
        .route("/sources", get(list_sources))
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

/// Handles POST `/query`.
///
/// # Errors
///
/// Returns:
/// - `400 BAD REQUEST` if the request is invalid.
/// - `500 INTERNAL SERVER ERROR` if database access fails.
async fn query_handler(
    State(state): State<AppState>,
    AxumJson(payload): AxumJson<QueryRequest>,
) -> Result<AxumJson<Vec<SearchResult<Book>>>, StatusCode> {
    // Get all broker books
    let mut results = {
        let mut broker = state.broker.lock().await;
        broker
            .fetch_all_with_source(&True)
            .await
            .unwrap_or_default()
    };

    // Get all SQLite books
    let db_books = sqlx::query_as::<_, Book>("SELECT title, author, isbn FROM books")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    for book in db_books {
        results.push(SearchResult {
            item: book,
            source: "Local DB".to_owned(),
        });
    }

    // Deduplicate
    let mut seen: HashSet<Book> = HashSet::new();
    results.retain(|r| seen.insert(r.item.clone()));

    // Apply filtering locally
    let filtered: Vec<SearchResult<Book>> = results
        .into_iter()
        .filter(|result| {
            // Source filtering
            if !payload.sources.is_empty() && !payload.sources.contains(&result.source) {
                return false;
            }

            let book = &result.item;

            match payload.operator.to_lowercase().as_str() {
                "and" => payload
                    .conditions
                    .iter()
                    .all(|cond| match cond.field.as_str() {
                        "author" => book
                            .author
                            .to_lowercase()
                            .contains(&cond.value.to_lowercase()),
                        "title" => book
                            .title
                            .to_lowercase()
                            .contains(&cond.value.to_lowercase()),
                        "isbn" => book
                            .isbn
                            .to_lowercase()
                            .contains(&cond.value.to_lowercase()),
                        _ => false,
                    }),
                _ => payload
                    .conditions
                    .iter()
                    .any(|cond| match cond.field.as_str() {
                        "author" => book
                            .author
                            .to_lowercase()
                            .starts_with(&cond.value.to_lowercase()),
                        "title" => book
                            .title
                            .to_lowercase()
                            .starts_with(&cond.value.to_lowercase()),
                        "isbn" => book
                            .isbn
                            .to_lowercase()
                            .starts_with(&cond.value.to_lowercase()),
                        _ => false,
                    }),
            }
        })
        .collect();

    Ok(AxumJson(filtered))
}

/// Handles POST `/books` by inserting a new book into `SQLite DB`.
async fn add_book(
    State(state): State<AppState>,
    AxumJson(payload): AxumJson<AddBookRequest>,
) -> impl IntoResponse {
    let book = Book {
        title: payload.title,
        author: payload.author,
        isbn: payload.isbn,
    };

    let mut broker = state.broker.lock().await;

    match broker.add_to_source(&payload.source, book).await {
        Ok(()) => (StatusCode::CREATED, "Book added").into_response(),
        Err(_) => (StatusCode::BAD_REQUEST, "Source not found or not writable").into_response(),
    }
}

/// Adds a new in-memory source.
///
/// # Errors
///
/// Returns `StatusCode::BAD_REQUEST` if the provided name is empty
/// or invalid.
async fn add_source_handler(
    State(state): State<AppState>,
    AxumJson(payload): AxumJson<AddSourceRequest>,
) -> Result<StatusCode, StatusCode> {
    let clean_name = payload.name.trim().to_owned();

    let source: Box<dyn Source<Book> + Send> = Box::new(MemorySource::new());

    {
        let mut broker = state.broker.lock().await;
        broker.add_source(clean_name, source);
    };

    Ok(StatusCode::CREATED)
}

/// Returns a list of all registered sources.
///
/// This endpoint is used by the frontend to populate
/// the source selection dropdown.
async fn list_sources(State(state): State<AppState>) -> AxumJson<Vec<SourceInfo>> {
    let sources = {
        let broker = state.broker.lock().await;

        broker
            .sources()
            .iter()
            .map(|(name, _)| SourceInfo { name: name.clone() })
            .collect()
    };

    AxumJson(sources)
}
