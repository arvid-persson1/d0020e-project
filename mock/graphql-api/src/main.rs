//! A simple GraphQL api, that has the ability to insert and fetch data.
//! Note that the data is persistent and a database needs to be removed to clear it.
use crate::{
    db::Db,
    queries::{Mutation, Query},
};
use async_graphql::{EmptySubscription, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{Router, extract::State, routing::post, serve};
use tokio::net::TcpListener;

pub mod book_schema;
pub mod db;
pub mod queries;

/// A type introduced just to make the handler a bit more readable.
type MySchema = Schema<Query, Mutation, EmptySubscription>;

/// The handler. It's the function that's run when there's a GraphQL request.
async fn handler(
    State(schema): State<MySchema>,
    graphql_request: GraphQLRequest,
) -> GraphQLResponse {
    let result = schema.execute(graphql_request.into_inner()).await;
    result.into()
}

/// # Panics
/// Panics if the server couldn't bind to the provided url
#[tokio::main]
async fn main() {
    // --- Setup database (I've made a struct for this) ---
    let database = Db::new("./mock/graphql-api/graphql_mock.db").await;
    // Please note that the clone is needed for ownership
    let query = Query {
        db: database.clone(),
    };
    let mutation = Mutation { db: database };
    let schema = Schema::new(query, mutation, EmptySubscription);

    // --- Start server ---
    let app = Router::new()
        .route("/graphql", post(handler))
        .with_state(schema);
    let listener = TcpListener::bind("127.0.0.1:8081")
        .await
        .expect("Unable to bind ip address");
    println!("Server's on http://127.0.0.1:8081");
    serve(listener, app).await.expect("Unable to start server");
}
