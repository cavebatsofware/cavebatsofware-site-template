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

use crate::docx::process_docx_template;
use crate::entities::{access_code, AccessCode};
use crate::errors::{AppError, AppResult};
use crate::middleware::AuthenticatedUser;
use crate::s3::S3Service;
use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get},
    Router,
};
use chrono::{NaiveDate, TimeZone, Utc};

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, QueryFilter, QueryOrder,
    Set,
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone)]
pub struct AccessCodeState {
    pub db: DatabaseConnection,
    pub s3: S3Service,
}

pub fn access_code_routes() -> Router<AccessCodeState> {
    Router::new()
        .route(
            "/api/admin/access-codes",
            get(list_codes).post(create_code_multipart),
        )
        .route("/api/admin/access-codes/{id}", delete(delete_code))
}

/// Replace {{ACCESS_CODE}} placeholder in HTML content with the actual access code
fn process_html_template(html_content: &[u8], access_code: &str) -> Result<Vec<u8>, AppError> {
    let html_string = String::from_utf8(html_content.to_vec())
        .map_err(|e| AppError::AuthError(format!("Invalid UTF-8 in HTML file: {}", e)))?;

    let processed_html = html_string.replace("{{ACCESS_CODE}}", access_code);

    Ok(processed_html.into_bytes())
}

#[derive(Serialize)]
struct AccessCodeResponse {
    id: Uuid,
    code: String,
    name: String,
    description: Option<String>,
    download_filename: Option<String>,
    expires_at: Option<String>,
    created_at: String,
    is_expired: bool,
    usage_count: i32,
}

impl From<access_code::Model> for AccessCodeResponse {
    fn from(model: access_code::Model) -> Self {
        let now = Utc::now();
        let is_expired = model
            .expires_at
            .as_ref()
            .map(|exp| exp.with_timezone(&Utc) < now)
            .unwrap_or(false);

        Self {
            id: model.id,
            code: model.code,
            name: model.name,
            description: model.description,
            download_filename: model.download_filename,
            expires_at: model
                .expires_at
                .map(|dt| dt.with_timezone(&Utc).to_rfc3339()),
            created_at: model.created_at.with_timezone(&Utc).to_rfc3339(),
            is_expired,
            usage_count: model.usage_count,
        }
    }
}

async fn list_codes(
    State(state): State<AccessCodeState>,
    _user: AuthenticatedUser,
) -> AppResult<Json<Vec<AccessCodeResponse>>> {
    let codes = AccessCode::find()
        .order_by(access_code::Column::CreatedAt, Order::Desc)
        .all(&state.db)
        .await?;
    let response: Vec<AccessCodeResponse> = codes.into_iter().map(Into::into).collect();
    Ok(Json(response))
}

async fn create_code_multipart(
    State(state): State<AccessCodeState>,
    user: AuthenticatedUser,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<AccessCodeResponse>)> {
    let mut code_value = String::new();
    let mut name_value = String::new();
    let mut description_value: Option<String> = None;
    let mut download_filename_value: Option<String> = None;
    let mut expires_at_value: Option<String> = None;
    let mut index_html: Option<Vec<u8>> = None;
    let mut document_docx: Option<Vec<u8>> = None;

    // Parse multipart form data
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::AuthError(format!("Failed to parse multipart form: {}", e)))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "code" => {
                code_value = field.text().await.map_err(|e| {
                    AppError::AuthError(format!("Failed to read code field: {}", e))
                })?;
            }
            "name" => {
                name_value = field.text().await.map_err(|e| {
                    AppError::AuthError(format!("Failed to read name field: {}", e))
                })?;
            }
            "description" => {
                let text = field.text().await.map_err(|e| {
                    AppError::AuthError(format!("Failed to read description field: {}", e))
                })?;
                description_value = if text.is_empty() { None } else { Some(text) };
            }
            "download_filename" => {
                let text = field.text().await.map_err(|e| {
                    AppError::AuthError(format!("Failed to read download_filename field: {}", e))
                })?;
                download_filename_value = if text.is_empty() { None } else { Some(text) };
            }
            "expires_at" => {
                let text = field.text().await.map_err(|e| {
                    AppError::AuthError(format!("Failed to read expires_at field: {}", e))
                })?;
                expires_at_value = if text.is_empty() { None } else { Some(text) };
            }
            "index_html" => {
                let data = field.bytes().await.map_err(|e| {
                    AppError::AuthError(format!("Failed to read index.html file: {}", e))
                })?;
                if !data.is_empty() {
                    index_html = Some(data.to_vec());
                }
            }
            "document_docx" => {
                let data = field.bytes().await.map_err(|e| {
                    AppError::AuthError(format!("Failed to read Document.docx file: {}", e))
                })?;
                if !data.is_empty() {
                    document_docx = Some(data.to_vec());
                }
            }
            _ => {
                // Unknown field, skip it
                tracing::warn!("Unknown multipart field: {}", field_name);
            }
        }
    }

    // Validate required fields
    if code_value.trim().is_empty() {
        return Err(AppError::AuthError(
            "Access code cannot be empty".to_string(),
        ));
    }

    if name_value.trim().is_empty() {
        return Err(AppError::AuthError("Name cannot be empty".to_string()));
    }

    // Validate that both files are provided
    if index_html.is_none() || document_docx.is_none() {
        return Err(AppError::AuthError(
            "Both index.html and Document.docx files are required".to_string(),
        ));
    }

    // Check if code already exists
    let existing = AccessCode::find()
        .filter(access_code::Column::Code.eq(&code_value))
        .one(&state.db)
        .await?;

    if existing.is_some() {
        return Err(AppError::AuthError(
            "Access code already exists".to_string(),
        ));
    }

    // Parse expiration date - accept YYYY-MM-DD format from HTML date input
    let expires_at = if let Some(exp_str) = expires_at_value {
        // Try to parse as date only (YYYY-MM-DD format from HTML date input)
        if let Ok(naive_date) = NaiveDate::parse_from_str(&exp_str, "%Y-%m-%d") {
            // Set time to end of day (23:59:59) in UTC
            let naive_datetime = naive_date
                .and_hms_opt(23, 59, 59)
                .ok_or_else(|| AppError::AuthError("Invalid date - cannot set time".to_string()))?;
            Some(Utc.from_utc_datetime(&naive_datetime).into())
        } else {
            // Fallback: try RFC3339 format for backward compatibility
            Some(chrono::DateTime::parse_from_rfc3339(&exp_str).map_err(|_| {
                AppError::AuthError("Invalid expiration date format. Use YYYY-MM-DD".to_string())
            })?)
        }
    } else {
        None
    };

    // Process templates by replacing {{ACCESS_CODE}} placeholder
    tracing::info!("Processing templates for access code: {}", code_value);

    // ensure both files are Some
    let index_html_data =
        index_html.ok_or_else(|| AppError::AuthError("index.html file is required".to_string()))?;
    let document_docx_data = document_docx
        .ok_or_else(|| AppError::AuthError("Document.docx file is required".to_string()))?;

    let processed_index_html = process_html_template(&index_html_data, &code_value)?;
    let processed_document_docx = process_docx_template(&document_docx_data, &code_value)?;

    // Upload files to S3 (before creating DB entry)
    tracing::info!(
        "Uploading processed files to S3 for access code: {}",
        code_value
    );

    state
        .s3
        .upload_file(&code_value, "index.html", processed_index_html)
        .await
        .map_err(|e| AppError::AuthError(format!("Failed to upload index.html to S3: {}", e)))?;

    state
        .s3
        .upload_file(&code_value, "Document.docx", processed_document_docx)
        .await
        .map_err(|e| AppError::AuthError(format!("Failed to upload Document.docx to S3: {}", e)))?;

    // Create database entry after successful uploads
    let new_code = access_code::ActiveModel {
        id: Set(Uuid::new_v4()),
        code: Set(code_value.clone()),
        name: Set(name_value),
        description: Set(description_value),
        download_filename: Set(download_filename_value),
        expires_at: Set(expires_at),
        created_at: Set(Utc::now().into()),
        created_by: Set(user.id),
        usage_count: Set(0),
        last_used_at: Set(None),
    };

    let result = new_code.insert(&state.db).await?;

    tracing::info!(
        "Access code created successfully: {} by user {}",
        code_value,
        user.id
    );

    Ok((StatusCode::CREATED, Json(result.into())))
}

async fn delete_code(
    State(state): State<AccessCodeState>,
    _user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let code = AccessCode::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::AuthError("Access code not found".to_string()))?;

    let active_model: access_code::ActiveModel = code.into();
    active_model.delete(&state.db).await?;

    Ok(StatusCode::NO_CONTENT)
}
