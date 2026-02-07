use crate::{builder, db, models, schema};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use diesel::prelude::*; // Connection and insert helpers
use tower::ServiceExt as _; // trait for calling the app
/// # Panics
/// Panics if:
/// -Creating the connection pool fails
/// -Fetching of the connection fails
/// -Insertion of the test book fails
fn init_test_db() -> db::DbPool {
    let pool = db::establish_connpool().expect("Failed to establish pool");

    let mut connection = pool.get().expect("Failed to get connection");

    let _unused = diesel::delete(schema::books::table).execute(&mut connection);

    let _ = diesel::insert_into(schema::books::table)
        .values((
            schema::books::isbn.eq("9780316497541"),
            schema::books::title.eq("The Last Wish: Introducing the Witcher"),
            schema::books::author.eq("Andrzej Sapkowski"),
            schema::books::format.eq(models::BookFormatType::Hardcover),
        ))
        .execute(&mut connection)
        .expect("Failed to insert book");

    pool
}
/// # Panics
/// Panics if tests fail
#[tokio::test]
async fn books_test() {
    let pool = init_test_db();
    let app = builder::build_app(pool);

    let response_list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/books")
                .body(Body::empty())
                .expect("Failed to build list request"),
        )
        .await
        .expect("Failed to execute list request");

    assert_eq!(response_list.status(), StatusCode::OK);

    let body_bytes_list = to_bytes(response_list.into_body(), usize::MAX)
        .await
        .expect("Failed to read list body");

    let books: Vec<models::Book> =
        serde_json::from_slice(&body_bytes_list).expect("Failed to deserialize list");

    assert!(!books.is_empty(), "List should not be empty");
    assert_eq!(
        books[0].get_title(),
        "The Last Wish: Introducing the Witcher"
    );
    assert_eq!(books[0].get_author(), "Andrzej Sapkowski");
    assert_eq!(books[0].get_format(), models::BookFormatType::Hardcover);
    assert_eq!(books[0].get_isbn(), "9780316497541");

    let response_single = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/books/9780316497541")
                .body(Body::empty())
                .expect("Failed to build list request"),
        )
        .await
        .expect("Failed to execute single item request");

    assert_eq!(response_single.status(), StatusCode::OK);

    let body_bytes_single = to_bytes(response_single.into_body(), usize::MAX)
        .await
        .expect("Failed to read single item body");

    let book: models::Book =
        serde_json::from_slice(&body_bytes_single).expect("Failed to deserialize single book");

    assert_eq!(book.get_isbn(), "9780316497541");
    assert_eq!(book.get_title(), "The Last Wish: Introducing the Witcher");
    assert_eq!(book.get_author(), "Andrzej Sapkowski");
    assert_eq!(book.get_format(), models::BookFormatType::Hardcover);
}
