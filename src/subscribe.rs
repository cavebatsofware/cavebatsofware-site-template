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

use crate::{
    email::EmailService,
    entities::{subscriber, Subscriber},
    errors::AppResult,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Extension, Json, Router,
};
use basic_axum_rate_limit::SecurityContext;
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct SubscribeState {
    pub email_service: Arc<EmailService>,
    pub callbacks: crate::security_callbacks::AppRateLimitCallbacks,
    pub db: DatabaseConnection,
}

#[derive(Deserialize)]
pub struct SubscribeRequest {
    email: String,
}

#[derive(Deserialize)]
pub struct VerifyQuery {
    token: String,
}

#[derive(Serialize)]
pub struct SubscribeResponse {
    success: bool,
    message: String,
}

pub fn subscribe_routes() -> Router<SubscribeState> {
    Router::new()
        .route("/api/subscribe", post(subscribe))
        .route("/api/subscribe/verify", get(verify_subscription))
}

async fn subscribe(
    State(state): State<SubscribeState>,
    Extension(security_context): Extension<SecurityContext>,
    Json(payload): Json<SubscribeRequest>,
) -> AppResult<impl IntoResponse> {
    // Validate email
    let email = payload.email.trim().to_lowercase();

    if email.is_empty() || email.len() > 254 || !email.contains('@') || !email.contains('.') {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(SubscribeResponse {
                success: false,
                message: "Invalid email address.".to_string(),
            }),
        ));
    }

    let subscribe_key = format!("subscribe:{}", security_context.ip_address);

    let ip_addr = security_context
        .ip_address
        .parse::<std::net::IpAddr>()
        .map_err(|e| {
            tracing::error!("Failed to parse IP address: {}", e);
            crate::errors::AppError::AuthError("Invalid IP address".to_string())
        })?;

    let has_recent_subscription = state
        .callbacks
        .has_recent_subscription(ip_addr)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check subscription rate limit: {}", e);
            e
        })
        .unwrap_or(false);

    if has_recent_subscription {
        tracing::warn!(
            "Subscription rate limit exceeded for IP: {}",
            security_context.ip_address
        );
        return Ok((
            StatusCode::TOO_MANY_REQUESTS,
            Json(SubscribeResponse {
                success: false,
                message: "You can only subscribe once every 24 hours.".to_string(),
            }),
        ));
    }

    // Check if email already exists
    let existing_subscriber = Subscriber::find()
        .filter(subscriber::Column::Email.eq(&email))
        .one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error checking subscriber: {}", e);
            e
        })
        .unwrap_or(None);

    if let Some(existing) = existing_subscriber {
        if existing.verified {
            return Ok((
                StatusCode::OK,
                Json(SubscribeResponse {
                    success: true,
                    message: "You're already subscribed!".to_string(),
                }),
            ));
        } else {
            // Resend verification email
            if let Some(token) = &existing.verification_token {
                let _ = state
                    .email_service
                    .send_subscription_confirmation(&email, token)
                    .await;
            }
            return Ok((
                StatusCode::OK,
                Json(SubscribeResponse {
                    success: true,
                    message: "Verification email resent. Please check your inbox.".to_string(),
                }),
            ));
        }
    }

    // Create new subscriber with verification token
    let verification_token = Uuid::new_v4().to_string();
    let now = Utc::now();

    let new_subscriber = subscriber::ActiveModel {
        id: Set(Uuid::new_v4()),
        email: Set(email.clone()),
        verified: Set(false),
        verification_token: Set(Some(verification_token.clone())),
        verified_at: Set(None),
        active: Set(true),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
    };

    match new_subscriber.insert(&state.db).await {
        Ok(_) => {
            // Send verification email
            match state
                .email_service
                .send_subscription_confirmation(&email, &verification_token)
                .await
            {
                Ok(_) => {
                    let _ = state
                        .callbacks
                        .log_access_attempt(
                            Some(ip_addr),
                            Some(security_context.user_agent.clone()),
                            &subscribe_key,
                            "subscribe_submit",
                            true,
                            0.0, // Not rate-limited
                            None,
                            None,
                        )
                        .await;

                    tracing::info!("New subscription created for {}", email);
                    Ok((
                        StatusCode::OK,
                        Json(SubscribeResponse {
                            success: true,
                            message: "Subscription successful! Please check your email to confirm."
                                .to_string(),
                        }),
                    ))
                }
                Err(e) => {
                    tracing::error!("Failed to send subscription confirmation: {}", e);

                    let _ = state
                        .callbacks
                        .log_access_attempt(
                            Some(ip_addr),
                            Some(security_context.user_agent.clone()),
                            &subscribe_key,
                            "subscribe_submit",
                            false,
                            0.0, // Not rate-limited
                            None,
                            None,
                        )
                        .await;

                    Ok((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(SubscribeResponse {
                            success: false,
                            message: "Failed to send confirmation email. Please try again later."
                                .to_string(),
                        }),
                    ))
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to create subscriber: {}", e);
            Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SubscribeResponse {
                    success: false,
                    message: "Failed to process subscription. Please try again later.".to_string(),
                }),
            ))
        }
    }
}

async fn verify_subscription(
    State(state): State<SubscribeState>,
    Query(query): Query<VerifyQuery>,
) -> Result<Redirect, Redirect> {
    // Find subscriber with this verification token
    let subscriber = Subscriber::find()
        .filter(subscriber::Column::VerificationToken.eq(&query.token))
        .one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error finding subscriber: {}", e);
            e
        })
        .unwrap_or(None);

    match subscriber {
        Some(sub) => {
            // Check if already verified
            if sub.verified {
                tracing::info!("Subscription already verified, redirecting to blog");
                return Ok(Redirect::to("/blog?verified=already"));
            }

            // Check if token is expired (7 days)
            let created_at = sub.created_at.with_timezone(&Utc);
            if Utc::now().signed_duration_since(created_at) > Duration::days(7) {
                tracing::warn!("Verification token expired, redirecting to blog");
                return Err(Redirect::to("/blog?verified=expired"));
            }

            // Verify the subscription
            let mut active_sub: subscriber::ActiveModel = sub.into();
            active_sub.verified = Set(true);
            active_sub.verified_at = Set(Some(Utc::now().into()));
            active_sub.verification_token = Set(None);
            active_sub.updated_at = Set(Utc::now().into());

            match active_sub.update(&state.db).await {
                Ok(_) => {
                    tracing::info!("Subscription verified for token: {}", query.token);
                    Ok(Redirect::to("/blog?verified=success"))
                }
                Err(e) => {
                    tracing::error!("Failed to verify subscription: {}", e);
                    Err(Redirect::to("/blog?verified=error"))
                }
            }
        }
        None => {
            tracing::warn!("Invalid verification token, redirecting to blog");
            Err(Redirect::to("/blog?verified=invalid"))
        }
    }
}
