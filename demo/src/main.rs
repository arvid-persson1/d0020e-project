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
    encode::pg_book::BookMapper,
    encode::xml::Xml,
    postgres::{Build as _, Builder as PostgresBuilder, models::Book},
    query::combinators::{And, Or},
    rest::{Build as _, Builder as RestBuilder},
};
use std::io::stdin;
use tokio::main;

use diesel as _;

use serde as _;

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
                .decoder(BookMapper)
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
                println!("{book:#?}");
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
                println!("{book:#?}");
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
                println!("  {book:#?}");
            }
        },
        Err(e) => println!("An error occurred: {e:#?}"),
    }
}
