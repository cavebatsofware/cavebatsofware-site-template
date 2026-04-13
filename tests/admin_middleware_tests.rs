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

use cavebatsofware_site_template::entities::admin_user;
use common::{
    build_test_server, create_verified_admin, get_csrf_token, login_as, test_email, TEST_PASSWORD,
};

use axum::http::StatusCode;
use sea_orm::{ActiveModelTrait, Set};
use totp_rs::{Algorithm, Secret, TOTP};

const TEST_TOTP_SECRET: &str = "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP";

// ==================== Helpers ====================

fn generate_totp_code(secret: &str, email: &str) -> String {
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        Secret::Encoded(secret.to_string())
            .to_bytes()
            .unwrap(),
        None,
        email.to_string(),
    )
    .unwrap();
    totp.generate_current().unwrap()
}

// ==================== require_admin_auth tests ====================

#[sqlx::test(migrations = false)]
async fn test_unauthenticated_request_returns_401(pool: sqlx::PgPool) {
    let (server, _backend, _db) = build_test_server(pool).await;

    let response = server.get("/api/admin/access-codes").await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    assert_eq!(response.text(), "Not authenticated");
}

#[sqlx::test(migrations = false)]
async fn test_authenticated_admin_access_succeeds(pool: sqlx::PgPool) {
    let (server, backend, _db) = build_test_server(pool).await;
    let email = test_email("mw-basic");
    create_verified_admin(&backend, &email, TEST_PASSWORD).await;
    login_as(&server, &email, TEST_PASSWORD).await;

    let response = server.get("/api/admin/access-codes").await;

    assert_eq!(response.status_code(), StatusCode::OK);
}

#[sqlx::test(migrations = false)]
async fn test_mfa_enabled_not_verified_returns_403(pool: sqlx::PgPool) {
    let (server, backend, _db) = build_test_server(pool).await;
    let email = test_email("mw-mfa-unverified");
    let admin = create_verified_admin(&backend, &email, TEST_PASSWORD).await;

    backend
        .update_totp(admin.id, Some(TEST_TOTP_SECRET.to_string()), true)
        .await
        .unwrap();

    login_as(&server, &email, TEST_PASSWORD).await;

    let response = server.get("/api/admin/access-codes").await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    assert_eq!(response.text(), "MFA verification required");
}

#[sqlx::test(migrations = false)]
async fn test_mfa_enabled_and_verified_returns_200(pool: sqlx::PgPool) {
    let (server, backend, _db) = build_test_server(pool).await;
    let email = test_email("mw-mfa-verified");
    let admin = create_verified_admin(&backend, &email, TEST_PASSWORD).await;

    backend
        .update_totp(admin.id, Some(TEST_TOTP_SECRET.to_string()), true)
        .await
        .unwrap();

    login_as(&server, &email, TEST_PASSWORD).await;

    // Verify MFA via the real endpoint
    let code = generate_totp_code(TEST_TOTP_SECRET, &email);
    let csrf = get_csrf_token(&server).await;
    let mfa_response = server
        .post("/api/admin/mfa/verify")
        .add_header("x-csrf-token", &csrf)
        .json(&serde_json::json!({ "code": code }))
        .await;
    assert_eq!(
        mfa_response.status_code(),
        StatusCode::OK,
        "MFA verify should succeed: {}",
        mfa_response.text()
    );

    let response = server.get("/api/admin/access-codes").await;

    assert_eq!(response.status_code(), StatusCode::OK);
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

    let response = server.get("/api/admin/access-codes").await;

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

    let csrf = get_csrf_token(&server).await;
    let response = server
        .post("/api/admin/change-password")
        .add_header("x-csrf-token", &csrf)
        .json(&serde_json::json!({
            "current_password": TEST_PASSWORD,
            "new_password": "NewStr0ng!Password456"
        }))
        .await;

    // Should NOT be blocked by force_password_change middleware
    assert_ne!(
        response.text(),
        "Password change required",
        "change-password should be allowed even with force_password_change"
    );
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

    let csrf = get_csrf_token(&server).await;
    let response = server
        .post("/api/admin/logout")
        .add_header("x-csrf-token", &csrf)
        .await;

    // Should NOT be blocked by force_password_change middleware
    assert_ne!(
        response.text(),
        "Password change required",
        "logout should be allowed even with force_password_change"
    );
}

// ==================== require_administrator tests ====================

#[sqlx::test(migrations = false)]
async fn test_administrator_role_access_succeeds(pool: sqlx::PgPool) {
    let (server, backend, _db) = build_test_server(pool).await;
    let email = test_email("mw-admin-role");
    create_verified_admin(&backend, &email, TEST_PASSWORD).await;
    login_as(&server, &email, TEST_PASSWORD).await;

    let response = server.get("/api/admin/access-codes").await;

    assert_eq!(response.status_code(), StatusCode::OK);
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

    let response = server.get("/api/admin/access-codes").await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    assert_eq!(response.text(), "Insufficient permissions");
}
