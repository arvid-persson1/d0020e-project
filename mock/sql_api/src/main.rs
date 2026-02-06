pub mod builder;
pub mod db;
pub mod schema;
pub mod models;
use diesel_derive_enum as _;
use dotenvy as _;
use serde_json as _;
use tokio::net::TcpListener;
use db::establish_connpool;
use builder::build_app;
use std::error::Error;
use tokio::runtime::Builder;

fn main() -> Result<(), Box<dyn Error>> {
  let runtime = Builder::new_multi_thread()
    .enable_all()
    .build()?;
  runtime.block_on(async_main())
}

async fn async_main() -> Result<(), Box<dyn Error>> {
  let pool = establish_connpool()?;
  let app = build_app(pool);

  let addrs = "127.0.0.1:1919";
  println!("App is listening on {addrs}");
  let listener = TcpListener::bind(addrs).await?;
  axum::serve(listener, app).await?;
  Ok(())
}
