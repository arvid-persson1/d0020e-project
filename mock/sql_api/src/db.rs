use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use dotenvy::dotenv;
use std::env;
use std::error::Error;

/// Represents the database pool.
/// Uses a db pool since diesel is synchronous and axum is asynchronous
pub(crate) type DbPool = Pool<ConnectionManager<PgConnection>>;
/// Represents the connection to the database to avoid long type signatures.
pub(crate) type DbConnection = PooledConnection<ConnectionManager<PgConnection>>;

/// Function that establishes the database connection pool.
/// Reads the `DATABASE_URL` environment variable and creates
/// a managed pool of postgres connections.
/// # Errors
/// Returns an error if:
/// -The `DATABASE_URL` environment variable is not set.
/// -The connection pool cannot be created
pub(crate) fn establish_connpool() -> Result<DbPool, Box<dyn Error>> {
    let _unused = dotenv().ok();

    let db_url = env::var("DATABASE_URL")?;

    let manager = ConnectionManager::<PgConnection>::new(db_url);

    let pool = Pool::builder().test_on_check_out(true).build(manager)?;

    Ok(pool)
}
