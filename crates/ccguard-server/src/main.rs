use ccguard_server::app::app;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        eprintln!("WARNING: DATABASE_URL not set — using local dev default");
        "postgres://ccguard:ccguard@localhost:5432/ccguard".into()
    });
    let pool = PgPoolOptions::new().connect(&url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Bind address is configurable so a deploy can hide the app behind a reverse
    // proxy (e.g. CCGUARD_BIND=127.0.0.1:7070 with nginx terminating TLS).
    let bind = std::env::var("CCGUARD_BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    println!("CCGuard server listening on {bind}");
    axum::serve(listener, app(pool)).await?;
    Ok(())
}
