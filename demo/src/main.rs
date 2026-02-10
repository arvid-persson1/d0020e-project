#![expect(missing_docs, reason = "Demo code.")]
#![allow(dead_code, reason = "Demo code.")]

use broker::{
    Broker,
    connector::Source,
    encode::json::Json,
    query::{
        Queryable,
        combinators::{And, Or},
    },
    rest::{Build as _, Builder as RestBuilder},
};
use serde::Deserialize;
use std::fmt::{Display, Error as FmtError, Formatter};
use tokio::main;

#[derive(Deserialize, Debug, Queryable)]
struct Book {
    title: String,
    author: String,
    isbn: String,
}

impl Display for Book {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self {
            title,
            author,
            isbn,
        } = self;
        write!(f, "\"{title}\" by {author}; ISBN {isbn}")
    }
}

#[main]
async fn main() {
    let mut broker = Broker::<Book>::new();
    println!("Instantiated broker.");

    broker.add_source(Box::new(
        RestBuilder::new()
            .source_url("http://127.0.0.1:8080")
            .expect("Failed to parse URL.")
            .decoder(Json)
            .build(),
    ));
    println!("Registered source (REST endpoint on 127.0.0.1:8080).");

    println!();

    let query = Book::author().eq("Jane Austen");
    println!("Defined query: {query:#?}\n");

    let results = broker.fetch_all(&query).await;
    match results {
        Ok(v) if v.is_empty() => {
            println!("No books matching query.");
        },
        Ok(v) => {
            for book in v {
                println!("{book}");
            }
        },
        Err(_) => {
            println!("An error occurred.");
            return;
        },
    }

    let query = And(
        Book::author().eq("George Orwell"),
        Or(Book::title().eq("1984"), Book::title().eq("Animal Farm")),
    );
    println!("Defined query: {query:#?} [1 residue]\n");

    let results = broker.fetch_all(&query).await;
    match results {
        Ok(v) if v.is_empty() => {
            println!("No books matching query.");
        },
        Ok(v) => {
            for book in v {
                println!("{book}");
            }
        },
        Err(_) => {
            println!("An error occurred.");
            return;
        },
    }
}
