use ccguard_server::app::app;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ccguard:ccguard@localhost:5432/ccguard".into());
    let pool = PgPoolOptions::new().connect(&url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    println!("CCGuard server listening on :8080");
    axum::serve(listener, app(pool)).await?;
    Ok(())
}
