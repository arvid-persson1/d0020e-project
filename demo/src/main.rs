#![expect(missing_docs, reason = "Demo code.")]
#![allow(dead_code, reason = "Demo code.")]
#![allow(clippy::missing_panics_doc, reason = "Demo code.")]
#![allow(clippy::use_debug, reason = "Demo code.")]
#![allow(clippy::shadow_unrelated, reason = "Demo code.")]
#![allow(clippy::missing_docs_in_private_items, reason = "Demo code.")]

use broker::postgres::PgDecode;
use broker::postgres::models::Book as Pgbook;
use broker::{
    Broker,
    connector::Source as _,
    encode::json::Json,
    encode::xml::Xml,
    postgres::{Build as _, Builder as PostgresBuilder},
    query::{
        Queryable,
        combinators::{And, Or},
    },
    rest::{Build as _, Builder as RestBuilder},
};
use diesel::RunQueryDsl as _;
use diesel::pg::PgConnection;
use diesel::result::Error as dslError;
use serde::Deserialize;
use std::{
    //env::var,
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

#[derive(Debug, Clone)]
pub struct DemoBookDecoder;

// Implements PgDecode for the local Book struct
impl PgDecode<Book> for DemoBookDecoder {
    #[inline]
    fn decode_all(
        &self,
        conn: &mut PgConnection,
        sql_text: &str,
    ) -> Result<Vec<Book>, dslError> {
        let full_query = if sql_text.trim().is_empty() {
            "SELECT * FROM books".to_owned()
        } else {
            format!("SELECT * FROM books WHERE {sql_text}")
        };

        let db_books = diesel::sql_query(full_query).load::<Pgbook>(conn)?;

        let unified_books = db_books
            .into_iter()
            .map(|db_model| Book {
                title: db_model.title,
                author: db_model.author,
                isbn: db_model.isbn,
            })
            .collect();

        Ok(unified_books)
    }
}

#[main]
async fn main() {
    let mut buf = String::new();

    let mut broker = Broker::<Book>::new();
    println!("Instantiated broker.");

    broker.add_source(
        "Bookery".into(),
        Box::new(
            RestBuilder::new()
                .source_url("http://127.0.0.1:8080/books")
                .expect("Failed to parse URL.")
                .decoder(Json)
                .build(),
        ),
    );
    println!("Registered source (REST endpoint on 127.0.0.1:8080).");
    broker.add_source(
        "Axum XML".into(),
        Box::new(
            RestBuilder::new()
                .source_url("http://127.0.0.1:1616/books")
                .expect("Failed to parse URL.")
                .decoder(Xml)
                .build(),
        ),
    );

    println!("Registered source (REST endpoint on 127.0.0.1:1616).");

    //let pg_url = var("postgres://mock_reader@localhost:5632/bookery_db").expect("Failed to parse URL.");

    broker.add_source(
        "PostgresDB".into(),
        Box::new(
            PostgresBuilder::<Book>::new()
                .url("postgres://mock_reader@localhost:5632/bookery_db")
                .decoder(DemoBookDecoder) // <--- Use your custom adapter!
                .build()
                .expect("Failed to build Postgres connector"),
        ),
    );

    println!("Registered source postgres");

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
        Err(e) => {
            println!("An error occurred: {e:#?}");
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
        Err(e) => {
            println!("An error occurred: {e:#?}");
            return;
        },
    }

    let query_sapkowski = Book::author().eq("Andrzej Sapkowski");
    println!("Defined query: {{query: {query_sapkowski:#?}}}");
    println!("Press ENTER to fetch Andrzej Sapkowski (Expected from Postgres DB)...");
    let _unused = stdin().read_line(&mut buf).expect("Failed to read line.");
    buf.clear();

    match broker.fetch_all(&query_sapkowski).await {
        Ok(books) if books.is_empty() => println!("No books matching query."),
        Ok(books) => {
            println!("Found books:");
            for book in books {
                println!("  {book}");
            }
        },
        Err(e) => println!("An error occurred: {e:#?}"),
    }
}
