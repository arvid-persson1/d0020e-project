#![expect(missing_docs, reason = "Demo code.")]
#![allow(dead_code, reason = "Demo code.")]

use broker::{
    Broker,
    connector::Source,
    encode::json::Json,
    query::Queryable,
    rest::{Build as _, Builder as RestBuilder},
};
use serde::Deserialize;
use tokio::main;

#[derive(Deserialize, Debug, Queryable)]
struct Book {
    title: String,
    author: String,
    format: BookFormatType,
    isbn: String,
}

#[derive(Deserialize, Debug)]
enum BookFormatType {
    Epub,
    Hardcover,
    Paperback,
}

#[main]
async fn main() {
    let mut broker = Broker::<Book>::new();

    broker.add_source(Box::new(
        RestBuilder::new()
            .source_url("http://127.0.0.1:8080")
            .expect("Failed to parse URL.")
            .decoder(Json)
            .build(),
    ));

    let query = Book::author().eq("Jane Austen");
    println!("query: {query:#?}");
    let results = broker.fetch_all(&query).await;
    println!("results: {results:#?}");
}
