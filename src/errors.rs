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

// Custom error types for better error handling and user experience

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;
use thiserror::Error;

/// Application-specific errors with proper HTTP status codes
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("File system error: {0}")]
    FileSystem(#[from] std::io::Error),

    #[error("Invalid access code")]
    InvalidAccess,

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Database(ref e) => {
                tracing::error!("Database error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            AppError::FileSystem(ref e) => {
                tracing::error!("File system error: {}", e);
                (StatusCode::NOT_FOUND, "Resource not found".to_string())
            }
            AppError::InvalidAccess => {
                tracing::warn!("Invalid access attempt");
                (StatusCode::NOT_FOUND, "Not found".to_string())
            }
            AppError::AuthError(msg) => {
                tracing::warn!("Authentication error: {}", msg);
                (StatusCode::UNAUTHORIZED, msg)
            }
            AppError::ValidationError(msg) => {
                tracing::debug!("Validation error: {}", msg);
                (StatusCode::BAD_REQUEST, msg)
            }
            AppError::InternalError(ref e) => {
                tracing::error!("Internal error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        (
            status,
            Json(ErrorResponse {
                error: error_message,
            }),
        )
            .into_response()
    }
}

/// Result type alias for convenience
pub type AppResult<T> = Result<T, AppError>;
