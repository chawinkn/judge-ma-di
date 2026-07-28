use anyhow::Result;
use deadpool_postgres::Pool;
use openssl::ssl::{SslConnector, SslMethod};
use postgres_openssl::MakeTlsConnector;
use std::{str::FromStr, time::Duration};

const POOL_SIZE: usize = 16;
const KEEPALIVES_IDLE: Duration = Duration::from_secs(240);

pub fn create_pool(url: &str) -> Result<Pool> {
    let mut pg_config = tokio_postgres::Config::from_str(url)?;
    pg_config.keepalives(true);
    pg_config.keepalives_idle(KEEPALIVES_IDLE);

    let mgr_config = deadpool_postgres::ManagerConfig {
        recycling_method: deadpool_postgres::RecyclingMethod::Verified,
    };

    let builder: openssl::ssl::SslConnectorBuilder =
        SslConnector::builder(SslMethod::tls()).unwrap();
    let connector = MakeTlsConnector::new(builder.build());

    let mgr = deadpool_postgres::Manager::from_config(pg_config, connector, mgr_config);

    let pool = deadpool_postgres::Pool::builder(mgr)
        .max_size(POOL_SIZE)
        .build()?;

    Ok(pool)
}
