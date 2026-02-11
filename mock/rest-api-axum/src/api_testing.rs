use crate::app_builder::build_app;
use crate::handlers::BookList;
use quick_xml::de::from_str;
use reqwest::header::CONTENT_TYPE;
use std::net::SocketAddr;
use tokio::net::TcpListener;

///# Panics
/// Panics if the TCP listener cannot bind to the requested address
/// or if the local address cannot be retrieved.
async fn spawn_app() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port");
    let addrs = listener
        .local_addr()
        .expect("Failed to get local address from listener");
    let service = build_app();
    drop(tokio::spawn(async move {
        axum::serve(listener, service)
            .await
            .expect("Server failed to start");
    }));

    addrs
}

fn get_new_book() -> &'static str {
    "<book>
    <title>Gallows Raging Fame</title>
    <author>Reighly Poams</author>
    <format>Hardcover</format>
    <isbn>1234567890123</isbn>
  </book>"
}

#[tokio::test]
///# Panics
/// Panics if the application cannot be spawned, the request fails,
/// or the response status is not 201 CREATED.
async fn book_test() {
    let addrs = spawn_app().await;
    let client = reqwest::Client::new();

    let new_book = get_new_book();

    let post_res = client
        .post(format!("http://{addrs}/books"))
        .header(CONTENT_TYPE, "application/xml")
        .body(new_book)
        .send()
        .await
        .expect("Failed to send request");

    let post_status = post_res.status();

    println!("The post status: {post_status}");

    assert_eq!(post_status, 201); // Created

    // Search by Author (Sapkowski)
    let search_author = client
        .get(format!("http://{addrs}/books?author=Andrzej+Sapkowski"))
        .send()
        .await
        .expect("Failed to send author request");

    assert_eq!(search_author.status(), 200);

    let body_author = search_author.text().await.expect("Failed to get text");
    // Verify we found the pre-filled book
    assert!(body_author.contains("The Last Wish: Introducing the Witcher"));
    assert!(body_author.contains("Andrzej Sapkowski"));

    let search_isbn = client
        .get(format!("http://{addrs}/books?isbn=9780316497541"))
        .send()
        .await
        .expect("Failed to send isbn request");

    assert_eq!(search_isbn.status(), 200);
    let body_isbn = search_isbn.text().await.expect("Failed to get text");
    assert!(body_isbn.contains("9780316497541"));

    // --- TEST 4: FETCH ALL BOOKS ---
    let list_res = client
        .get(format!("http://{addrs}/books"))
        .send()
        .await
        .expect("Failed to get list");

    assert_eq!(list_res.status(), 200);

    let body_list = list_res.text().await.expect("Failed to get text");
    // Parse the list to check contents
    let book_list: BookList = from_str(&body_list).expect("Failed to parse response");

    // We expect 5 books now (4 pre-filled + 1 added)
    assert_eq!(book_list.books.len(), 5);

    let found_sapkowski = book_list
        .books
        .iter()
        .any(|b| b.title == "The Last Wish: Introducing the Witcher");
    assert!(found_sapkowski, "List should contain pre-filled data");

    let found_new_book = book_list
        .books
        .iter()
        .any(|b| b.title == "Gallows Raging Fame");
    assert!(found_new_book, "List should contain the posted book");
}
