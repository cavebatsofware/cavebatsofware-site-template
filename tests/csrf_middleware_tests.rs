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

use axum::http::StatusCode;
use common::{build_test_server, get_csrf_token};

// ==================== Tests ====================

#[sqlx::test(migrations = false)]
async fn test_get_request_exempt_from_csrf(pool: sqlx::PgPool) {
    let (server, _backend, _db) = build_test_server(pool).await;

    let response = server.get("/api/admin/auth-config").await;

    assert_eq!(response.status_code(), StatusCode::OK);
}

#[sqlx::test(migrations = false)]
async fn test_post_without_token_returns_403(pool: sqlx::PgPool) {
    let (server, _backend, _db) = build_test_server(pool).await;

    let response = server
        .post("/api/admin/login")
        .json(&serde_json::json!({"email": "x@x.com", "password": "pass"}))
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    assert!(response.text().contains("CSRF"));
}

#[sqlx::test(migrations = false)]
async fn test_post_with_invalid_token_returns_403(pool: sqlx::PgPool) {
    let (server, _backend, _db) = build_test_server(pool).await;

    let response = server
        .post("/api/admin/login")
        .add_header("x-csrf-token", "bogus-token")
        .json(&serde_json::json!({"email": "x@x.com", "password": "pass"}))
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = false)]
async fn test_post_with_valid_token_passes_csrf(pool: sqlx::PgPool) {
    let (server, _backend, _db) = build_test_server(pool).await;

    let token = get_csrf_token(&server).await;

    // POST with valid CSRF token — will fail auth (401) but should NOT be 403/CSRF
    let response = server
        .post("/api/admin/login")
        .add_header("x-csrf-token", &token)
        .json(&serde_json::json!({"email": "x@x.com", "password": "wrong"}))
        .await;

    assert_ne!(
        response.status_code(),
        StatusCode::FORBIDDEN,
        "CSRF should pass; expected non-403 (got {})",
        response.status_code()
    );
}

#[sqlx::test(migrations = false)]
async fn test_put_without_token_returns_403(pool: sqlx::PgPool) {
    let (server, _backend, _db) = build_test_server(pool).await;

    let response = server
        .put("/api/admin/access-codes/00000000-0000-0000-0000-000000000000")
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    assert!(response.text().contains("CSRF"));
}

#[sqlx::test(migrations = false)]
async fn test_delete_without_token_returns_403(pool: sqlx::PgPool) {
    let (server, _backend, _db) = build_test_server(pool).await;

    let response = server
        .delete("/api/admin/access-codes/00000000-0000-0000-0000-000000000000")
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    assert!(response.text().contains("CSRF"));
}

#[sqlx::test(migrations = false)]
async fn test_cross_session_token_rejected(pool: sqlx::PgPool) {
    // Server A — fetch a CSRF token bound to its session
    let (server_a, _backend_a, _db_a) = build_test_server(pool.clone()).await;
    let token_a = get_csrf_token(&server_a).await;

    // Server B — separate cookie jar = separate session
    let (server_b, _backend_b, _db_b) = build_test_server(pool).await;
    // Establish server B's own session
    get_csrf_token(&server_b).await;

    // Use server A's token on server B — should fail
    let response = server_b
        .post("/api/admin/login")
        .add_header("x-csrf-token", &token_a)
        .json(&serde_json::json!({"email": "x@x.com", "password": "pass"}))
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = false)]
async fn test_contact_route_csrf_enforced(pool: sqlx::PgPool) {
    let (server, _backend, _db) = build_test_server(pool).await;

    let response = server
        .post("/api/contact")
        .json(&serde_json::json!({"name": "Test", "email": "t@t.com", "message": "hi"}))
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    assert!(response.text().contains("CSRF"));
}

#[sqlx::test(migrations = false)]
async fn test_subscribe_route_csrf_enforced(pool: sqlx::PgPool) {
    let (server, _backend, _db) = build_test_server(pool).await;

    let response = server
        .post("/api/subscribe")
        .json(&serde_json::json!({"email": "t@t.com"}))
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    assert!(response.text().contains("CSRF"));
}
