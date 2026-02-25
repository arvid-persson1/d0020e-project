//! HTTP API broker that aggregates book data from multiple REST sources.

use broker::connector::Source;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
    Json as AxumJson,
};
use broker::{
    Broker,
    query::Queryable,
    rest::{Build as _, Builder as RestBuilder},
    encode::xml::Xml,
};
use broker::encode::json::Json as BrokerJson;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

/// Shared application state.
#[derive(Clone)]
struct AppState {
    broker: Arc<Mutex<Broker<Book>>>,
}

/// Incoming JSON payload for `/query`.
#[derive(Deserialize)]
struct QueryRequest {
    field: String,
    value: String,
}

/// Representation of a book.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash, Queryable)]
struct Book {
    title: String,
    author: String,
    isbn: String,
}

#[tokio::main]
async fn main() {
    let mut broker = Broker::<Book>::new();

    // JSON source
    broker.add_source(Box::new(
        RestBuilder::new()
            .source_url("http://127.0.0.1:8080/books")
            .unwrap()
            .decoder(BrokerJson)
            .build(),
    ));

    // XML source
    broker.add_source(Box::new(
        RestBuilder::new()
            .source_url("http://127.0.0.1:1616/books")
            .unwrap()
            .decoder(Xml)
            .build(),
    ));

    let state = AppState {
        broker: Arc::new(Mutex::new(broker)),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(|| async { "API Broker Running" }))
        .route("/query", post(query_handler))
        .layer(cors)
        .with_state(state);

    println!("Running at http://localhost:3000");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}

/// Handles POST `/query`
async fn query_handler(
    State(state): State<AppState>,
    AxumJson(payload): AxumJson<QueryRequest>,
) -> impl IntoResponse {
    let query = match payload.field.as_str() {
        "author" => Book::author().eq(&payload.value),
        "title" => Book::title().eq(&payload.value),
        "isbn" => Book::isbn().eq(&payload.value),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                AxumJson(Vec::<Book>::new()),
            ).into_response();
        }
    };

    let mut broker = state.broker.lock().await;

    match broker.fetch_all(&query).await {
        Ok(results) => (StatusCode::OK, AxumJson(results)).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            AxumJson(Vec::<Book>::new()),
        ).into_response(),
    }
}
