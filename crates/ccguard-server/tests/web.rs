use axum::body::Body;
use axum::http::{Request, StatusCode};
use sqlx::PgPool;
use tower::ServiceExt;

use ccguard_server::app::app;
use ccguard_server::passwords::hash_password;

async fn seed_user(pool: &PgPool) {
    sqlx::query("insert into tenants (id,name) values ('acme','Acme')")
        .execute(pool)
        .await
        .unwrap();
    let ph = hash_password("pw12345");
    sqlx::query("insert into users (tenant_id,email,password_hash,role) values ('acme','boss@acme.com',$1,'owner')")
        .bind(&ph)
        .execute(pool)
        .await
        .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn login_sets_cookie_and_redirects(pool: PgPool) {
    seed_user(&pool).await;
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/web/login")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "tenant_id=acme&email=boss@acme.com&password=pw12345",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers().get("location").unwrap(), "/dashboard");
    assert!(resp
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("ccg_session="));
}

#[sqlx::test(migrations = "./migrations")]
async fn bad_login_rerenders_form(pool: PgPool) {
    seed_user(&pool).await;
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/web/login")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "tenant_id=acme&email=boss@acme.com&password=WRONG",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
