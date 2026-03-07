/*  This file is part of a basic website template project - cavebatsofware-site-template
 *  Copyright (C) 2025 Grant DeFayette & Cavebatsoftware LLC
 *
 *  cavebatsofware-site-template is free software: you can redistribute it and/or modify
 *  it under the terms of either the GNU General Public License as published by
 *  the Free Software Foundation, version 3 of the License (GPL-3.0-only), OR under
 *  the 3 clause BSD License (BSD-3-Clause).
 *
 *  If you wish to use this software under the GPL-3.0-only license, remove
 *  references to BSD-3-Clause and copies of the BSD-3-Clause license from copies you distribute,
 *  unless you would like to dual-license your modifications to the software.
 *
 *  If you wish to use this software under the BSD-3-Clause license, remove
 *  references to GPL-3.0-only and copies of the GPL-3.0-only License from copies you distribute,
 *  unless you would like to dual-license your modifications to the software.
 *
 *  cavebatsofware-site-template is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License and BSD 3 Clause License
 *  along with cavebatsofware-site-template.  If not, see <https://www.gnu.org/licenses/gpl-3.0.html>.
 *  For BSD-3-Clause terms, see <https://opensource.org/licenses/BSD-3-Clause>
 */

mod common;

use cavebatsofware_site_template::admin::auth::AdminAuthBackend;
use cavebatsofware_site_template::admin::Credentials;
use cavebatsofware_site_template::entities::admin_user;
use cavebatsofware_site_template::middleware::admin_auth::{
    require_admin_auth, require_administrator, AdminAuthSession,
};
use common::{test_db_from_pool, test_email};

use axum::{
    http::StatusCode,
    middleware::from_fn,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_login::AuthManagerLayerBuilder;
use axum_test::TestServer;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use tower_sessions::{Session, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;

const TEST_PASSWORD: &str = "MyStr0ng!Password123";

// ==================== Test-only handlers ====================

async fn test_login_handler(
    mut auth_session: AdminAuthSession,
    Json(creds): Json<Credentials>,
) -> impl IntoResponse {
    match auth_session.authenticate(creds).await {
        Ok(Some(user)) => {
            auth_session.login(&user).await.unwrap();
            StatusCode::OK
        }
        Ok(None) => StatusCode::UNAUTHORIZED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn test_set_mfa_handler(session: Session) -> impl IntoResponse {
    session.insert("mfa_verified", true).await.unwrap();
    StatusCode::OK
}

async fn protected_handler() -> impl IntoResponse {
    (StatusCode::OK, "protected content")
}

async fn admin_only_handler() -> impl IntoResponse {
    (StatusCode::OK, "admin only content")
}

// ==================== Helpers ====================

async fn create_verified_admin(
    backend: &AdminAuthBackend,
    email: &str,
    password: &str,
) -> admin_user::Model {
    let (_admin, token) = backend.create_admin(email, password).await.unwrap();
    backend.verify_email(&token).await.unwrap()
}

async fn build_test_server(
    pool: sqlx::PgPool,
) -> (TestServer, AdminAuthBackend, DatabaseConnection) {
    dotenvy::dotenv().ok();

    let db = test_db_from_pool(pool.clone()).await;

    // Session store using the same pool as SeaORM
    let session_store = PostgresStore::new(pool);
    session_store
        .migrate()
        .await
        .expect("session table migration should succeed");

    let session_layer = SessionManagerLayer::new(session_store);
    let backend = AdminAuthBackend::new(db.clone());
    let auth_layer = AuthManagerLayerBuilder::new(backend.clone(), session_layer).build();

    // Routes behind require_admin_auth only
    let protected_routes = Router::new()
        .route("/protected", get(protected_handler))
        .route("/api/admin/change-password", post(protected_handler))
        .route("/api/admin/logout", post(protected_handler))
        .layer(from_fn(require_admin_auth));

    // Routes behind require_administrator + require_admin_auth
    let admin_only_routes = Router::new()
        .route("/admin-only", get(admin_only_handler))
        .layer(from_fn(require_administrator))
        .layer(from_fn(require_admin_auth));

    // Intentionally misconfigured: require_administrator WITHOUT require_admin_auth
    let misconfigured_routes = Router::new()
        .route("/misconfigured", get(admin_only_handler))
        .layer(from_fn(require_administrator));

    // Test-only routes (no auth middleware, but auth_layer provides session)
    let test_routes = Router::new()
        .route("/test-login", post(test_login_handler))
        .route("/test-set-mfa", post(test_set_mfa_handler));

    let app = Router::new()
        .merge(protected_routes)
        .merge(admin_only_routes)
        .merge(misconfigured_routes)
        .merge(test_routes)
        .layer(auth_layer);

    let server = TestServer::builder().save_cookies().build(app).unwrap();

    (server, backend, db)
}

async fn login_as(server: &TestServer, email: &str, password: &str) {
    let response = server
        .post("/test-login")
        .json(&serde_json::json!({
            "email": email,
            "password": password,
        }))
        .await;
    assert_eq!(
        response.status_code(),
        StatusCode::OK,
        "Login should succeed for {}",
        email
    );
}

// ==================== require_admin_auth tests ====================

#[sqlx::test(migrations = false)]
async fn test_unauthenticated_request_returns_401(pool: sqlx::PgPool) {
    let (server, _backend, _db) = build_test_server(pool).await;

    let response = server.get("/protected").await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    assert_eq!(response.text(), "Not authenticated");
}

#[sqlx::test(migrations = false)]
async fn test_authenticated_user_access_succeeds(pool: sqlx::PgPool) {
    let (server, backend, _db) = build_test_server(pool).await;
    let email = test_email("mw-basic");
    create_verified_admin(&backend, &email, TEST_PASSWORD).await;
    login_as(&server, &email, TEST_PASSWORD).await;

    let response = server.get("/protected").await;

    assert_eq!(response.status_code(), StatusCode::OK);
    assert_eq!(response.text(), "protected content");
}

#[sqlx::test(migrations = false)]
async fn test_mfa_enabled_not_verified_returns_403(pool: sqlx::PgPool) {
    let (server, backend, _db) = build_test_server(pool).await;
    let email = test_email("mw-mfa-unverified");
    let admin = create_verified_admin(&backend, &email, TEST_PASSWORD).await;

    backend
        .update_totp(admin.id, Some("JBSWY3DPEHPK3PXP".to_string()), true)
        .await
        .unwrap();

    login_as(&server, &email, TEST_PASSWORD).await;

    let response = server.get("/protected").await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    assert_eq!(response.text(), "MFA verification required");
}

#[sqlx::test(migrations = false)]
async fn test_mfa_enabled_and_verified_returns_200(pool: sqlx::PgPool) {
    let (server, backend, _db) = build_test_server(pool).await;
    let email = test_email("mw-mfa-verified");
    let admin = create_verified_admin(&backend, &email, TEST_PASSWORD).await;

    backend
        .update_totp(admin.id, Some("JBSWY3DPEHPK3PXP".to_string()), true)
        .await
        .unwrap();

    login_as(&server, &email, TEST_PASSWORD).await;

    // Set MFA verified in session
    let mfa_response = server.post("/test-set-mfa").await;
    assert_eq!(mfa_response.status_code(), StatusCode::OK);

    let response = server.get("/protected").await;

    assert_eq!(response.status_code(), StatusCode::OK);
    assert_eq!(response.text(), "protected content");
}

#[sqlx::test(migrations = false)]
async fn test_force_password_change_blocks_normal_routes(pool: sqlx::PgPool) {
    let (server, backend, db) = build_test_server(pool).await;
    let email = test_email("mw-forcepw");
    let admin = create_verified_admin(&backend, &email, TEST_PASSWORD).await;

    let mut active: admin_user::ActiveModel = admin.into();
    active.force_password_change = Set(true);
    active.update(&db).await.unwrap();

    login_as(&server, &email, TEST_PASSWORD).await;

    let response = server.get("/protected").await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    assert_eq!(response.text(), "Password change required");
}

#[sqlx::test(migrations = false)]
async fn test_force_password_change_allows_change_password_endpoint(pool: sqlx::PgPool) {
    let (server, backend, db) = build_test_server(pool).await;
    let email = test_email("mw-forcepw-cp");
    let admin = create_verified_admin(&backend, &email, TEST_PASSWORD).await;

    let mut active: admin_user::ActiveModel = admin.into();
    active.force_password_change = Set(true);
    active.update(&db).await.unwrap();

    login_as(&server, &email, TEST_PASSWORD).await;

    let response = server.post("/api/admin/change-password").await;

    assert_eq!(response.status_code(), StatusCode::OK);
}

#[sqlx::test(migrations = false)]
async fn test_force_password_change_allows_logout_endpoint(pool: sqlx::PgPool) {
    let (server, backend, db) = build_test_server(pool).await;
    let email = test_email("mw-forcepw-lo");
    let admin = create_verified_admin(&backend, &email, TEST_PASSWORD).await;

    let mut active: admin_user::ActiveModel = admin.into();
    active.force_password_change = Set(true);
    active.update(&db).await.unwrap();

    login_as(&server, &email, TEST_PASSWORD).await;

    let response = server.post("/api/admin/logout").await;

    assert_eq!(response.status_code(), StatusCode::OK);
}

// ==================== require_administrator tests ====================

#[sqlx::test(migrations = false)]
async fn test_administrator_role_access_succeeds(pool: sqlx::PgPool) {
    let (server, backend, _db) = build_test_server(pool).await;
    let email = test_email("mw-admin-role");
    create_verified_admin(&backend, &email, TEST_PASSWORD).await;
    login_as(&server, &email, TEST_PASSWORD).await;

    let response = server.get("/admin-only").await;

    assert_eq!(response.status_code(), StatusCode::OK);
    assert_eq!(response.text(), "admin only content");
}

#[sqlx::test(migrations = false)]
async fn test_non_admin_role_returns_403(pool: sqlx::PgPool) {
    let (server, backend, db) = build_test_server(pool).await;
    let email = test_email("mw-viewer-role");
    let admin = create_verified_admin(&backend, &email, TEST_PASSWORD).await;

    // Change role to "viewer" in DB before login
    let mut active: admin_user::ActiveModel = admin.into();
    active.role = Set("viewer".to_string());
    active.update(&db).await.unwrap();

    login_as(&server, &email, TEST_PASSWORD).await;

    let response = server.get("/admin-only").await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    assert_eq!(response.text(), "Insufficient permissions");
}

#[sqlx::test(migrations = false)]
async fn test_require_administrator_without_auth_middleware_returns_500(pool: sqlx::PgPool) {
    let (server, backend, _db) = build_test_server(pool).await;
    let email = test_email("mw-misconfig");
    create_verified_admin(&backend, &email, TEST_PASSWORD).await;
    login_as(&server, &email, TEST_PASSWORD).await;

    let response = server.get("/misconfigured").await;

    assert_eq!(response.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(response.text(), "Internal server error");
}
