use ccguard_server::app::app;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Config is mandatory: locate configuration/ccg.json (cwd or any ancestor),
    // resolve |ConfigPath|, and die loudly if it's missing.
    let (cfg, _config_dir) = ccguard_server::config::load().unwrap_or_else(|e| {
        eprintln!("CCGuard config error: {e}");
        std::process::exit(1);
    });

    // Logging is configured before anything else logs. Guard lives until main exits.
    let _log_guard = ccguard_server::logging::init(&cfg.logging);

    let pool = PgPoolOptions::new().connect(&cfg.database.url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Bind address stays env-configurable for the reverse-proxy deploy.
    let bind = std::env::var("CCGUARD_BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!("CCGuard server listening on {bind}");
    axum::serve(listener, app(pool)).await?;
    Ok(())
}
