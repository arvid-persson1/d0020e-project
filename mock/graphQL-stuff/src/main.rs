//! The GraphQL api
use crate::db::DB;
use crate::queries::Query;
use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{Router, routing::post, serve};

mod book_schema;
mod db;
mod queries;

// The handler
async fn handler(graphql_request: GraphQLRequest) -> GraphQLResponse {
    let query = Query { db: DB };
    let schema = Schema::new(query, EmptyMutation, EmptySubscription);

    let result = schema.execute(graphql_request.into_inner()).await;

    result.into()
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/graphql", post(handler));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8081")
        .await
        .unwrap();

    // You might wanna move this later
    println!("Server's on http://127.0.0.1:8081");

    serve(listener, app).await.unwrap()
}
