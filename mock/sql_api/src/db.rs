use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use dotenvy::dotenv;
use std::env;
use std::error::Error;

//Use db pool since diesel is synchronous and axum is asynchronous
pub(crate) type DbPool = Pool<ConnectionManager<PgConnection>>;
pub(crate) type DbConnection = PooledConnection<ConnectionManager<PgConnection>>;

pub(crate) fn establish_connpool() -> Result<DbPool, Box<dyn Error>> {
  let _unused = dotenv().ok();

  let db_url = env::var("DATABASE_URL")?;

  let manager = ConnectionManager::<PgConnection>::new(db_url);

  let pool = Pool::builder()
    .test_on_check_out(true)
    .build(manager)?;

  Ok(pool)

}
