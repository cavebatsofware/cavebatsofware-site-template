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

use crate::errors::AppResult;
use crate::middleware::AuthenticatedUser;
use crate::settings::SettingsService;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, put},
    Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone)]
pub struct SettingsState {
    pub settings: SettingsService,
}

pub fn settings_routes() -> Router<SettingsState> {
    Router::new()
        .route("/api/admin/settings", get(get_all_settings))
        .route("/api/admin/settings", put(update_setting))
}

#[derive(Serialize)]
struct SettingResponse {
    id: Uuid,
    key: String,
    value: String,
    category: Option<String>,
}

async fn get_all_settings(
    State(state): State<SettingsState>,
    _user: AuthenticatedUser,
) -> AppResult<Json<Vec<SettingResponse>>> {
    tracing::info!("Fetching all settings");

    let settings = state.settings.get_all().await.map_err(|e| {
        tracing::error!("Failed to get settings from database: {}", e);
        crate::errors::AppError::AuthError(e.to_string())
    })?;

    tracing::info!("Found {} settings", settings.len());

    let responses: Vec<SettingResponse> = settings
        .into_iter()
        .map(|s| SettingResponse {
            id: s.id,
            key: s.key,
            value: s.value,
            category: s.category,
        })
        .collect();

    tracing::info!("Returning {} setting responses", responses.len());
    Ok(Json(responses))
}

#[derive(Deserialize)]
struct UpdateSettingRequest {
    key: String,
    value: String,
    category: Option<String>,
}

async fn update_setting(
    State(state): State<SettingsState>,
    _user: AuthenticatedUser,
    Json(req): Json<UpdateSettingRequest>,
) -> AppResult<StatusCode> {
    state
        .settings
        .set(&req.key, &req.value, req.category.as_deref(), None)
        .await
        .map_err(|e| crate::errors::AppError::AuthError(e.to_string()))?;

    Ok(StatusCode::OK)
}
