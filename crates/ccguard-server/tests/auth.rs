use axum::extract::FromRequestParts;
use axum::http::Request;
use ccguard_server::auth::AuthedTenant;
use ccguard_server::tokens::generate_token;
use sqlx::PgPool;

async fn seed_token(pool: &PgPool) -> String {
    sqlx::query("insert into tenants (id, name) values ('acme','Acme')")
        .execute(pool).await.unwrap();
    let (token, hash) = generate_token();
    sqlx::query("insert into api_tokens (tenant_id, token_hash) values ('acme', $1)")
        .bind(&hash).execute(pool).await.unwrap();
    token
}

async fn extract(pool: PgPool, header: Option<&str>) -> Result<AuthedTenant, ccguard_server::error::AppError> {
    let mut builder = Request::builder().uri("/");
    if let Some(h) = header {
        builder = builder.header("authorization", h);
    }
    let req = builder.body(()).unwrap();
    let (mut parts, _) = req.into_parts();
    AuthedTenant::from_request_parts(&mut parts, &pool).await
}

#[sqlx::test(migrations = "./migrations")]
async fn valid_token_resolves_tenant(pool: PgPool) {
    let token = seed_token(&pool).await;
    let res = extract(pool, Some(&format!("Bearer {token}"))).await;
    assert!(matches!(res, Ok(AuthedTenant(ref t)) if t == "acme"));
}

#[sqlx::test(migrations = "./migrations")]
async fn invalid_token_is_rejected(pool: PgPool) {
    seed_token(&pool).await;
    let res = extract(pool, Some("Bearer ccg_deadbeef")).await;
    assert!(matches!(res, Err(ccguard_server::error::AppError::Unauthorized(_))));
}

#[sqlx::test(migrations = "./migrations")]
async fn missing_header_is_rejected(pool: PgPool) {
    seed_token(&pool).await;
    let res = extract(pool, None).await;
    assert!(matches!(res, Err(ccguard_server::error::AppError::Unauthorized(_))));
}
