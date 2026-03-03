#![expect(missing_docs, reason = "Demo code.")]
#![allow(dead_code, reason = "Demo code.")]
#![allow(clippy::missing_panics_doc, reason = "Demo code.")]
#![allow(clippy::use_debug, reason = "Demo code.")]
#![allow(clippy::shadow_unrelated, reason = "Demo code.")]
#![allow(clippy::missing_docs_in_private_items, reason = "Demo code.")]

use broker::{
    Broker,
    connector::Source as _,
    encode::json::Json,
    encode::xml::Xml,
    query::{
        Queryable,
        combinators::{And, Or},
    },
    rest::{Build as _, Builder as RestBuilder},
};
use serde::Deserialize;
use std::{
    fmt::{Display, Error as FmtError, Formatter},
    io::stdin,
};
use tokio::main;

/// Struct for book
#[derive(Deserialize, Debug, PartialEq, Eq, Hash, Queryable)]
struct Book {
    /// Title
    title: String,
    /// Author
    author: String,
    /// Isbn
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
    let mut buf = String::new();

    let mut broker = Broker::<Book>::new();
    println!("Instantiated broker.");

    broker.add_source(
        "JSON API",
        Box::new(
        RestBuilder::new()
            .source_url("http://127.0.0.1:8080/books")
            .expect("Failed to parse URL.")
            .decoder(Json)
            .build(),
    ));
    println!("Registered source (REST endpoint on 127.0.0.1:8080).");
    broker.add_source(
        "XML API",
        Box::new(
        RestBuilder::new()
            .source_url("http://127.0.0.1:1616/books")
            .expect("Failed to parse URL.")
            .decoder(Xml)
            .build(),
    ));
    println!("Registered source (REST endpoint on 127.0.0.1:1616).");
    println!();

    let query = Book::author().eq("Jane Austen");
    println!("Defined query: {query:#?}\nPress ENTER to run.");

    drop(stdin().read_line(&mut buf));
    buf.clear();

    let results = broker.fetch_all(&query).await;
    match results {
        Ok(v) if v.is_empty() => {
            println!("No books matching query.");
        },
        Ok(v) => {
            println!("Found books:");
            for book in v {
                println!("{book}");
            }
        },
        Err(_) => {
            println!("An error occurred.");
            return;
        },
    }
    println!();

    let query1 = And(
        Book::author().eq("George Orwell"),
        Or(Book::title().eq("1984"), Book::title().eq("Animal Farm")),
    );
    println!("Defined query: {query1:#?} [1 residue]\nPress ENTER to run.");

    drop(stdin().read_line(&mut buf));
    buf.clear();

    let results1 = broker.fetch_all(&query1).await;
    match results1 {
        Ok(v) if v.is_empty() => {
            println!("No books matching query.");
        },
        Ok(v) => {
            println!("Found books:");
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
