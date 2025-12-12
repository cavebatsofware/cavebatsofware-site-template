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

use uuid::Uuid;

pub mod access_log;
pub mod admin_auth;

pub use access_log::access_log_middleware;
pub use admin_auth::{require_admin_auth, AuthenticatedUser};

/// Minimal admin user info for access logging. Only stores what's needed for audit trail.
/// Uuid is Copy (16 bytes), email is the only heap allocation.
#[derive(Clone)]
pub struct AdminUserInfo {
    pub id: Uuid,
    pub email: String,
}

// Re-export CSRF from the axum-tower-sessions-csrf crate
pub use axum_tower_sessions_csrf::CsrfMiddleware;

/// CSRF middleware function (alias for CsrfMiddleware::middleware)
pub fn csrf_middleware(
    session: tower_sessions::Session,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> impl std::future::Future<Output = axum::response::Response> {
    CsrfMiddleware::middleware(session, request, next)
}
