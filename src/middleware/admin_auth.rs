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

use crate::admin::{AdminAuthBackend, AdminUserAuth};
use crate::middleware::AdminUserInfo;
use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_login::AuthSession;
use tower_sessions::Session;

pub type AdminAuthSession = AuthSession<AdminAuthBackend>;

const MFA_VERIFIED_KEY: &str = "mfa_verified";

pub async fn require_admin_auth(
    auth_session: AdminAuthSession,
    session: Session,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    tracing::debug!(
        "require_admin_auth middleware called for: {}",
        request.uri()
    );
    tracing::debug!("Auth session user present: {}", auth_session.user.is_some());

    if let Some(user) = auth_session.user {
        tracing::debug!("User authenticated: {}", user.email);

        // Check if MFA is required but not verified in session
        if user.totp_enabled {
            let mfa_verified = session
                .get::<bool>(MFA_VERIFIED_KEY)
                .await
                .unwrap_or(None)
                .unwrap_or(false);

            if !mfa_verified {
                tracing::warn!("MFA required but not verified for user: {}", user.email);
                return (StatusCode::FORBIDDEN, "MFA verification required").into_response();
            }
        }

        // Check if user must change password
        // Allow access to change-password and logout endpoints
        if user.force_password_change {
            let path = request.uri().path();
            if path != "/api/admin/change-password" && path != "/api/admin/logout" {
                tracing::warn!("Password change required for user: {}", user.email);
                return (StatusCode::FORBIDDEN, "Password change required").into_response();
            }
        }

        // Store minimal info for access logging (id is Copy, email cloned once)
        let user_info = AdminUserInfo {
            id: user.id,
            email: user.email.clone(),
        };
        request.extensions_mut().insert(user);
        let mut response = next.run(request).await;
        // Insert into response extensions for access_log_middleware
        response.extensions_mut().insert(user_info);
        tracing::debug!("Handler completed with status: {}", response.status());
        response
    } else {
        tracing::warn!("Authentication required but user not present");
        (StatusCode::UNAUTHORIZED, "Not authenticated").into_response()
    }
}

/// Extension type for accessing authenticated admin user in handlers
/// Usage in handlers:
/// ```ignore
/// async fn handler(Extension(user): Extension<AdminUserAuth>) -> impl IntoResponse {
///     // user is guaranteed to be authenticated here
/// }
/// ```
pub type AuthenticatedUser = axum::extract::Extension<AdminUserAuth>;
