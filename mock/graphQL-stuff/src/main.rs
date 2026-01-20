//! The GraphQL api
use std::env::current_dir;

use crate::book_schema::BookFormatType;
use crate::queries::Query;
use crate::{book_schema::Book, db::DB};
use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{Router, extract::State, routing::post, serve};

mod book_schema;
mod db;
mod queries;

// I'm lazy so this is just for the handler below
type MySchema = Schema<Query, EmptyMutation, EmptySubscription>;

// The handler (that one guy that does stuff when making queries)
async fn handler(
    State(schema): State<MySchema>,
    graphql_request: GraphQLRequest,
) -> GraphQLResponse {
    let database = DB::new("./mock/graphQL-stuff/graphql_mock.db").await;
    let query = Query { db: database };
    let schema = Schema::new(query, EmptyMutation, EmptySubscription);

    let result = schema.execute(graphql_request.into_inner()).await;

    result.into()
}

#[tokio::main]
async fn main() {
    // --- Setup database (I've made a struct for this) ---
    let database = DB::new("./mock/graphQL-stuff/graphql_mock.db").await;
    let query = Query { db: database };
    let schema = Schema::new(query, EmptyMutation, EmptySubscription);

    // --- Start server ---
    let app = Router::new()
        .route("/graphql", post(handler))
        .with_state(schema);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8081")
        .await
        .unwrap();
    println!("Server's on http://127.0.0.1:8081");
    serve(listener, app).await.unwrap();
}
