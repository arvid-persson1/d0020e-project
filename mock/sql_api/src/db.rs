use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use dotenvy::dotenv;
use std::{env, fmt::{Display, Formatter, Result as StdRes}, error::Error};

//Use db pool since diesel is synchronous and axum is asynchronous
pub(crate) type DbPool = Pool<ConnectionManager<PgConnection>>;
pub(crate) type DbConnection = PooledConnection<ConnectionManager<PgConnection>>;

// Custom error type for database operations
#[derive(Debug)]
pub(crate) enum DbError {
    Connection(String),
    Pool(String),
}

impl Display for DbError {
  fn fmt(&self, f: &mut Formatter<'_>) -> StdRes {
    match self {
      Self::Connection(msg) => write!(f, "Connection error: {msg}"),
      Self::Pool(err_msg) => write!(f, "Pool error: {err_msg}"),
    }
  }
}

impl Error for DbError {}

pub(crate) fn establish_connpool() -> Result<DbPool, DbError> {
  let _unused = dotenv().ok();

  let db_url = env::var("DB_URL")
    .map_err(|_foo| DbError::Connection(
      "DATABASE_URL must be set in .env file".to_owned()

    ))?;

  let manager = ConnectionManager::<PgConnection>::new(db_url);

  Pool::builder()
    .test_on_check_out(true)
    .build(manager)
    .map_err(|e| DbError::Pool(e.to_string()))
}
