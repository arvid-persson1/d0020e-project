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

fn get_book1() -> &'static str {
  "<book>
    <title>Nineteen Eighty-Four</title>
    <author>George Orwell</author>
    <format>Hardcover</format>
    <isbn>9780198185215</isbn>
  </book>"
}

fn get_book2() -> &'static str {
  "<book>
    <title>The Last Wish: Introducing the Witcher</title>
    <author>Andrzej Sapkowski</author>
    <format>Hardcover</format>
    <isbn>9780316497541</isbn>
  </book>"
}

#[tokio::test]
///# Panics
/// Panics if the application cannot be spawned, the request fails,
/// or the response status is not 201 CREATED.
async fn book_test() {
    let addrs = spawn_app().await;
    let client = reqwest::Client::new();

    let test_data = get_book1();

    let post_reqst = client
        .post(format!("http://{addrs}/books"))
        .header(CONTENT_TYPE, "application/xml")
        .body(test_data)
        .send()
        .await
        .expect("Failed to send request");

    let status1 = post_reqst.status();

    assert_eq!(status1, 201);

    let body1 = post_reqst
        .text()
        .await
        .expect("Failed to retrieve response text");

    assert!(body1.contains("<title>Nineteen Eighty-Four</title>"));
    assert!(body1.contains("<isbn>9780198185215</isbn>"));

    let other_test_data = get_book2();

    let other_post_reqst = client
        .post(format!("http://{addrs}/books"))
        .header(CONTENT_TYPE, "application/xml")
        .body(other_test_data)
        .send()
        .await
        .expect("Failed to send request");

    let status2 = other_post_reqst.status();
    assert_eq!(status2, 201);

    let body2 = other_post_reqst
        .text()
        .await
        .expect("Failed to retrieve response text");
    assert!(body2.contains("<title>The Last Wish: Introducing the Witcher</title>"));
    assert!(body2.contains("<isbn>9780316497541</isbn>"));

    let isbn_target = "9780316497541";

    let get_reqst = client
        .get(format!("http://{addrs}/books/{isbn_target}"))
        .send()
        .await
        .expect("Failed to send request");

    let status3 = get_reqst.status();

    assert_eq!(status3, 200);

    let body3 = get_reqst
        .text()
        .await
        .expect("Failed to retrieve response text");

    assert!(body3.contains("<title>The Last Wish: Introducing the Witcher</title>"));
    assert!(body3.contains("<isbn>9780316497541</isbn>"));

    let get_list_reqst = client
        .get(format!("http://{addrs}/books"))
        .send()
        .await
        .expect("Failed to send request");

    let status4 = get_list_reqst.status();

    assert_eq!(status4, 200);

    let body4 = get_list_reqst
        .text()
        .await
        .expect("Failed to retrieve response text");
    let book_list: BookList = from_str(&body4).expect("Failed to parse the response");

    assert_eq!(book_list.books[0].get_title(), "Nineteen Eighty-Four");
    assert_eq!(book_list.books[0].get_isbn(), "9780198185215");

    assert_eq!(
        book_list.books[1].get_title(),
        "The Last Wish: Introducing the Witcher"
    );
    assert_eq!(book_list.books[1].get_isbn(), "9780316497541");
}
