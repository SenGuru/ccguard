use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

use ccguard_server::app::app;
use ccguard_server::passwords::hash_password;

#[sqlx::test(migrations = "./migrations")]
async fn login_succeeds_and_rejects_bad_password(pool: PgPool) {
    sqlx::query("insert into tenants (id,name) values ('acme','Acme')")
        .execute(&pool).await.unwrap();
    let ph = hash_password("pw12345");
    sqlx::query("insert into users (tenant_id,email,password_hash,role) values ('acme','a@acme.com',$1,'admin')")
        .bind(&ph).execute(&pool).await.unwrap();

    // correct password -> 200 + token + role
    let resp = app(pool.clone())
        .oneshot(Request::builder().method("POST").uri("/v1/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"tenant_id":"acme","email":"a@acme.com","password":"pw12345"}"#)).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v["session_token"].as_str().unwrap().starts_with("ccg_"));
    assert_eq!(v["role"], "admin");

    // wrong password -> 401
    let resp = app(pool.clone())
        .oneshot(Request::builder().method("POST").uri("/v1/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"tenant_id":"acme","email":"a@acme.com","password":"WRONG"}"#)).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
