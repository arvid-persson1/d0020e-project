#![expect(missing_docs, reason = "Demo code.")]
#![allow(dead_code, reason = "Demo code.")]

use reqwest::*;
use serde_json::json;

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
async fn main() -> Result<()> {
    // let mut broker = Broker::<Book>::new();
    //
    // broker.add_source(Box::new(
    //     RestBuilder::new()
    //         .source_url("http://127.0.0.1:8081/graphql")
    //         .expect("Failed to parse URL.")
    //         .source_method(Method::POST)
    //         .decoder(Json)
    //         .build(),
    // ));
    //
    // let query = Book::author().eq("Jane Austen");
    // println!("query: {query:#?}");
    // let results = broker.fetch_all(&query).await;
    // println!("results: {results:#?}");

    // -- Initial test to see if things work at all --
    let client = Client::new();

    let query = r#"
        query {
            getBooks(
                filter: {format: PDF}
            ) 
            {title, author, isbn}
        }
    "#;

    let variables = json!({ "id": "123" });

    // NOTE: I think this is the simplest way to convert between the str arrays and json
    let body = json!({
        "query": query,
        "variables": variables,
    });

    let response = client
        .post("http://127.0.0.1:8081/graphql")
        .json(&body) // This automatically sets Content-Type to application/json
        .send()
        .await?;

    let response_text = response.text().await?;
    println!("Response: {}", response_text);

    Ok(())
}
