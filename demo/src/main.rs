#![expect(missing_docs, reason = "Demo code.")]
#![allow(dead_code, reason = "Demo code.")]

use reqwest::*;
use serde_json::{Value, json};

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

// Testing it here first!
fn body_builder(
    (method, resolver, resolver_input, value, fields): (&str, &str, &str, &str, Vec<&str>),
) -> Value {
    let fields_formatted = fields.join(", ");
    let mut query;
    if resolver_input == "" {
        query = format!("{method} {{ {resolver} {{ {fields_formatted} }} }}");
    } else {
        query = format!(
            "{method} {{ {resolver}({resolver_input}: {value}) {{ {fields_formatted} }} }}"
        );
    }
    json!({"query": query})
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

    // This is my home for testing for now
    let client = Client::new();
    let temp = (
        "query",
        "getAllBooks",
        "",
        "",
        vec!["title", "author", "isbn"],
    );

    let body = body_builder(temp);
    println!("Body: {}", body);

    let response = client
        .post("http://127.0.0.1:8081/graphql")
        .json(&body) // This automatically sets Content-Type to application/json
        .send()
        .await?;

    let response_text = response.text().await?;
    println!("Response: {}", response_text);

    Ok(())
}
