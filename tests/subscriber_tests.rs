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

use cavebatsofware_site_template::entities::subscriber;
use common::{build_test_server, get_csrf_token};

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use uuid::Uuid;

// ==================== Helpers ====================

/// Insert a subscriber directly into the database.
async fn insert_subscriber(
    db: &DatabaseConnection,
    email: &str,
    verified: bool,
    verification_token: Option<String>,
) -> subscriber::Model {
    let now = Utc::now();
    let model = subscriber::ActiveModel {
        id: Set(Uuid::new_v4()),
        email: Set(email.to_string()),
        verified: Set(verified),
        verification_token: Set(verification_token),
        verified_at: Set(if verified { Some(now.into()) } else { None }),
        active: Set(true),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
    };
    model.insert(db).await.unwrap()
}

/// Insert a subscriber with a custom created_at timestamp (for expiry tests).
async fn insert_subscriber_with_created_at(
    db: &DatabaseConnection,
    email: &str,
    verification_token: String,
    created_at: chrono::DateTime<Utc>,
) -> subscriber::Model {
    let model = subscriber::ActiveModel {
        id: Set(Uuid::new_v4()),
        email: Set(email.to_string()),
        verified: Set(false),
        verification_token: Set(Some(verification_token)),
        verified_at: Set(None),
        active: Set(true),
        created_at: Set(created_at.into()),
        updated_at: Set(created_at.into()),
    };
    model.insert(db).await.unwrap()
}

// ==================== Subscribe ====================

#[sqlx::test(migrations = false)]
async fn test_subscribe_without_csrf_returns_403(pool: sqlx::PgPool) {
    let (server, _backend, _db) = build_test_server(pool).await;

    let response = server
        .post("/api/subscribe")
        .json(&serde_json::json!({"email": "test@example.com"}))
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = false)]
async fn test_subscribe_invalid_email_returns_400(pool: sqlx::PgPool) {
    let (server, _backend, _db) = build_test_server(pool).await;

    let csrf = get_csrf_token(&server).await;
    let response = server
        .post("/api/subscribe")
        .add_header("x-csrf-token", &csrf)
        .json(&serde_json::json!({"email": ""}))
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
    let json: serde_json::Value = response.json();
    assert_eq!(json["success"].as_bool().unwrap(), false);
    assert!(json["message"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("invalid"));
}

#[sqlx::test(migrations = false)]
async fn test_subscribe_already_verified_returns_message(pool: sqlx::PgPool) {
    let (server, _backend, db) = build_test_server(pool).await;

    let email = "verified-subscriber@example.com";
    insert_subscriber(&db, email, true, None).await;

    let csrf = get_csrf_token(&server).await;
    let response = server
        .post("/api/subscribe")
        .add_header("x-csrf-token", &csrf)
        .json(&serde_json::json!({"email": email}))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let json: serde_json::Value = response.json();
    assert_eq!(json["success"].as_bool().unwrap(), true);
    assert!(json["message"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("already subscribed"));
}

// ==================== Verify subscription ====================

#[sqlx::test(migrations = false)]
async fn test_verify_valid_token_redirects_success(pool: sqlx::PgPool) {
    let (server, _backend, db) = build_test_server(pool).await;

    let token = Uuid::new_v4().to_string();
    insert_subscriber(
        &db,
        "verify-success@example.com",
        false,
        Some(token.clone()),
    )
    .await;

    let response = server
        .get(&format!("/api/subscribe/verify?token={}", token))
        .await;

    assert_eq!(response.status_code(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(location, "/blog?verified=success");
}

#[sqlx::test(migrations = false)]
async fn test_verify_invalid_token_redirects_invalid(pool: sqlx::PgPool) {
    let (server, _backend, _db) = build_test_server(pool).await;

    let response = server
        .get("/api/subscribe/verify?token=nonexistent-token")
        .await;

    assert_eq!(response.status_code(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(location, "/blog?verified=invalid");
}

#[sqlx::test(migrations = false)]
async fn test_verify_already_verified_redirects(pool: sqlx::PgPool) {
    let (server, _backend, db) = build_test_server(pool).await;

    let token = Uuid::new_v4().to_string();
    insert_subscriber(
        &db,
        "already-verified@example.com",
        true,
        Some(token.clone()),
    )
    .await;

    let response = server
        .get(&format!("/api/subscribe/verify?token={}", token))
        .await;

    assert_eq!(response.status_code(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(location, "/blog?verified=already");
}

#[sqlx::test(migrations = false)]
async fn test_verify_expired_token_redirects_expired(pool: sqlx::PgPool) {
    let (server, _backend, db) = build_test_server(pool).await;

    let token = Uuid::new_v4().to_string();
    let eight_days_ago = Utc::now() - Duration::days(8);
    insert_subscriber_with_created_at(
        &db,
        "expired-subscriber@example.com",
        token.clone(),
        eight_days_ago,
    )
    .await;

    let response = server
        .get(&format!("/api/subscribe/verify?token={}", token))
        .await;

    assert_eq!(response.status_code(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(location, "/blog?verified=expired");
}
