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

use cavebatsofware_site_template::entities::{access_code, access_log, admin_user};
use common::{
    build_test_server, create_verified_admin, get_csrf_token, login_as, test_email, TEST_PASSWORD,
};

use axum::http::StatusCode;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait, Set};
use uuid::Uuid;

// ==================== Helpers ====================

/// Insert an access log entry directly into the database.
async fn insert_access_log(
    db: &DatabaseConnection,
    access_code: &str,
    action: &str,
    success: bool,
    ip_address: &str,
) -> access_log::Model {
    let model = access_log::ActiveModel {
        id: Set(Uuid::new_v4()),
        access_code: Set(access_code.to_string()),
        ip_address: Set(Some(ip_address.to_string())),
        user_agent: Set(Some("test-agent".to_string())),
        tokens: Set(Some(10.0)),
        last_access_time: Set(None),
        last_delta_access: Set(None),
        action: Set(action.to_string()),
        success: Set(success),
        admin_user_id: Set(None),
        admin_user_email: Set(None),
        created_at: Set(Utc::now().into()),
    };
    model.insert(db).await.unwrap()
}

/// Insert an access code with usage data for dashboard metrics testing.
async fn insert_access_code_with_usage(
    db: &DatabaseConnection,
    code: &str,
    name: &str,
    created_by: Uuid,
    usage_count: i32,
) -> access_code::Model {
    let model = access_code::ActiveModel {
        id: Set(Uuid::new_v4()),
        code: Set(code.to_string()),
        name: Set(name.to_string()),
        description: Set(None),
        download_filename: Set(None),
        expires_at: Set(None),
        created_at: Set(Utc::now().into()),
        created_by: Set(created_by),
        usage_count: Set(usage_count),
        last_used_at: Set(Some(Utc::now().into())),
    };
    model.insert(db).await.unwrap()
}

// ==================== List access logs ====================

#[sqlx::test(migrations = false)]
async fn test_list_logs_unauthenticated_returns_401(pool: sqlx::PgPool) {
    let (server, _backend, _db) = build_test_server(pool).await;

    let response = server.get("/api/admin/access-logs").await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = false)]
async fn test_list_logs_viewer_returns_403(pool: sqlx::PgPool) {
    let (server, backend, db) = build_test_server(pool).await;
    let email = test_email("al-viewer");
    let admin = create_verified_admin(&backend, &email, TEST_PASSWORD).await;

    let mut active: admin_user::ActiveModel = admin.into();
    active.role = Set("viewer".to_string());
    active.update(&db).await.unwrap();

    login_as(&server, &email, TEST_PASSWORD).await;

    let response = server.get("/api/admin/access-logs").await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    assert_eq!(response.text(), "Insufficient permissions");
}

#[sqlx::test(migrations = false)]
async fn test_list_logs_returns_empty_paginated(pool: sqlx::PgPool) {
    let (server, backend, _db) = build_test_server(pool).await;
    let email = test_email("al-empty");
    create_verified_admin(&backend, &email, TEST_PASSWORD).await;
    login_as(&server, &email, TEST_PASSWORD).await;

    let response = server.get("/api/admin/access-logs").await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let json: serde_json::Value = response.json();
    assert!(json["data"].is_array());
    assert_eq!(json["data"].as_array().unwrap().len(), 0);
    assert_eq!(json["total"].as_u64().unwrap(), 0);
}

#[sqlx::test(migrations = false)]
async fn test_list_logs_returns_inserted_logs(pool: sqlx::PgPool) {
    let (server, backend, db) = build_test_server(pool).await;
    let email = test_email("al-inserted");
    create_verified_admin(&backend, &email, TEST_PASSWORD).await;

    insert_access_log(&db, "code-1", "GET:/access/code-1", true, "203.0.113.10").await;
    insert_access_log(&db, "code-2", "GET:/access/code-2", false, "198.51.100.42").await;

    login_as(&server, &email, TEST_PASSWORD).await;

    let response = server.get("/api/admin/access-logs").await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let json: serde_json::Value = response.json();
    let data = json["data"].as_array().unwrap();
    assert_eq!(data.len(), 2);
    assert_eq!(json["total"].as_u64().unwrap(), 2);

    // Verify response structure on first entry
    let first = &data[0];
    assert!(first["id"].is_string());
    assert!(first["access_code"].is_string());
    assert!(first["action"].is_string());
    assert!(first["success"].is_boolean());
    assert!(first["created_at"].is_string());
    assert!(first["ip_address"].is_string());
}

// ==================== Clear access logs ====================

#[sqlx::test(migrations = false)]
async fn test_clear_logs_without_csrf_returns_403(pool: sqlx::PgPool) {
    let (server, _backend, _db) = build_test_server(pool).await;

    let response = server.delete("/api/admin/access-logs").await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = false)]
async fn test_clear_logs_removes_all(pool: sqlx::PgPool) {
    let (server, backend, db) = build_test_server(pool).await;
    let email = test_email("al-clear");
    create_verified_admin(&backend, &email, TEST_PASSWORD).await;

    insert_access_log(&db, "code-1", "GET", true, "203.0.113.1").await;
    insert_access_log(&db, "code-2", "GET", true, "203.0.113.2").await;
    insert_access_log(&db, "code-3", "GET", false, "198.51.100.3").await;

    login_as(&server, &email, TEST_PASSWORD).await;

    let csrf = get_csrf_token(&server).await;
    let response = server
        .delete("/api/admin/access-logs")
        .add_header("x-csrf-token", &csrf)
        .await;

    assert_eq!(response.status_code(), StatusCode::NO_CONTENT);

    // Verify all logs deleted from DB
    let count = access_log::Entity::find().count(&db).await.unwrap();
    assert_eq!(count, 0);
}

// ==================== Dashboard metrics ====================

#[sqlx::test(migrations = false)]
async fn test_dashboard_metrics_unauthenticated_returns_401(pool: sqlx::PgPool) {
    let (server, _backend, _db) = build_test_server(pool).await;

    let response = server.get("/api/admin/dashboard/metrics").await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = false)]
async fn test_dashboard_metrics_returns_structure(pool: sqlx::PgPool) {
    let (server, backend, db) = build_test_server(pool).await;
    let email = test_email("al-dash");
    let admin = create_verified_admin(&backend, &email, TEST_PASSWORD).await;

    // Insert successful access logs (only success=true is counted in hourly metrics)
    insert_access_log(&db, "test-code", "GET:/access/test-code", true, "203.0.113.50").await;
    insert_access_log(&db, "test-code", "GET:/access/test-code", true, "198.51.100.60").await;

    // Insert an access code with recent usage
    insert_access_code_with_usage(&db, "test-code", "Test Code", admin.id, 5).await;

    login_as(&server, &email, TEST_PASSWORD).await;

    let response = server.get("/api/admin/dashboard/metrics").await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let json: serde_json::Value = response.json();

    // Verify hourly_access_rates is a 24-element array
    let hourly = json["hourly_access_rates"].as_array().unwrap();
    assert_eq!(hourly.len(), 24);
    let first_hour = &hourly[0];
    assert!(first_hour["hour"].is_string());
    assert!(first_hour["count"].is_number());

    // Verify recent_access_codes contains our code
    let recent = json["recent_access_codes"].as_array().unwrap();
    assert!(!recent.is_empty());
    let first_code = &recent[0];
    assert!(first_code["id"].is_string());
    assert!(first_code["name"].is_string());
    assert!(first_code["count"].is_number());
}
