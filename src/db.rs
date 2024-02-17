use std::env;
use deadpool_postgres::{ManagerConfig, RecyclingMethod, Runtime};
use deadpool_postgres::Pool;
use postgres::NoTls;

fn env_or(key: &str, default: &str) -> Option<String> {
    Some(
        env::var(key).unwrap_or(default.to_string())
    )
}

fn get_db_config() -> deadpool_postgres::Config {
    let mut config = deadpool_postgres::Config::new();

    config.user = env_or("DB_USER", "admin");
    config.password = env_or("DB_PASSWORD", "1234");
    config.dbname = env_or("DB_NAME", "rinha");
    config.host = env_or("DB_HOST", "localhost");
    config.port = Some(5432);

    config.manager =
        Some(ManagerConfig { recycling_method: RecyclingMethod::Fast });

    config
}

pub fn create_pool() -> Result<Pool, String> {
    Ok(get_db_config().create_pool(Some(Runtime::Tokio1), NoTls).map_err(|err| err.to_string())?)
}