//! Demo of the axum rest api serving XML

#![allow(clippy::missing_panics_doc, reason = "Demo code")]

use broker::{
    Broker,
    connector::Source as _,
    encode::xml::Xml,
    query::{
        Queryable,
        combinators::{And, Or},
    },
    rest::{Build as _, Builder as RestBuilder},
};

use serde::Deserialize;
use std::fmt::{Display, Error as FmtError, Formatter};
use tokio::main;

#[derive(Debug, Deserialize, PartialEq)]
/// Type for the supported book formats
enum BookFormatType {
    /// Format for PDF
    Pdf,
    /// Format for docx (Word)
    Docx,
    /// Format for Epub
    Epub,
    /// Format for Hardcover
    Hardcover,
    /// Format for Paperback
    Paperback,
}

impl Display for BookFormatType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        // Explicitly map each variant to a string
        match self {
            Self::Pdf => write!(f, "Pdf"),
            Self::Docx => write!(f, "Docx"),
            Self::Epub => write!(f, "Epub"),
            Self::Hardcover => write!(f, "Hardcover"),
            Self::Paperback => write!(f, "Paperback"),
        }
    }
}

/// Struct for book
#[derive(Deserialize, Debug, Queryable)]
struct Book {
    /// Title
    title: String,
    /// Author
    author: String,
    /// Isbn
    isbn: String,
    /// Book format
    format: BookFormatType,
}

impl Display for Book {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self {
            title,
            author,
            isbn,
            format,
        } = self;
        write!(f, "\"{title}\" by {author} format ({format}); ISBN {isbn}")
    }
}

#[main]
async fn main() {
    let mut broker = Broker::<Book>::new();
    println!("Instantiated broker.");

    broker.add_source(Box::new(
        RestBuilder::new()
            .source_url("http://127.0.0.1:1616/books")
            .expect("Failed to parse URL.")
            .decoder(Xml)
            .build(),
    ));
    println!("Registered source (REST endpoint on 127.0.0.1:1616).");

    let query = And(
        Book::author().eq("Andrzej Sapkowski"),
        Or(
            Book::title().eq("The Last Wish: Introducing the Witcher"),
            Book::title().eq("Sword of Destiny: Tales of the Witcher"),
        ),
    );

    println!("The query: {query:#?}");

    println!("Sending query...");
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
        Err(e) => {
            println!("An error occurred: {e}");
            return;
        },
    }
}
