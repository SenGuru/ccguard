use axum::body::Body;
use axum::http::{Request, StatusCode};
use sqlx::{PgPool, Row};
use tower::ServiceExt;

use ccguard_server::app::app;

#[sqlx::test(migrations = "./migrations")]
async fn admin_creates_user(pool: PgPool) {
    std::env::set_var("ADMIN_TOKEN", "secret-admin");
    sqlx::query("insert into tenants (id,name) values ('acme','Acme')")
        .execute(&pool).await.unwrap();

    let body = r#"{"tenant_id":"acme","email":"a@acme.com","password":"pw12345","role":"owner"}"#;
    let resp = app(pool.clone())
        .oneshot(Request::builder().method("POST").uri("/v1/users")
            .header("content-type", "application/json")
            .header("x-admin-token", "secret-admin")
            .body(Body::from(body)).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let row = sqlx::query("select count(*) as c from users where tenant_id='acme'")
        .fetch_one(&pool).await.unwrap();
    let c: i64 = row.get("c");
    assert_eq!(c, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn create_user_rejects_bad_admin_token(pool: PgPool) {
    std::env::set_var("ADMIN_TOKEN", "secret-admin");
    sqlx::query("insert into tenants (id,name) values ('acme','Acme')")
        .execute(&pool).await.unwrap();

    let body = r#"{"tenant_id":"acme","email":"a@acme.com","password":"pw12345","role":"owner"}"#;
    let resp = app(pool.clone())
        .oneshot(Request::builder().method("POST").uri("/v1/users")
            .header("content-type", "application/json")
            .header("x-admin-token", "WRONG")
            .body(Body::from(body)).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
