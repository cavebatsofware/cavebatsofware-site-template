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

use super::auth::verify_password;
use super::password::PasswordValidator;
use super::totp;
use super::{AdminAuthBackend, Credentials};
use crate::email::EmailService;
use crate::errors::{AppError, AppResult};
use crate::security_callbacks::AppRateLimitCallbacks;
use crate::settings::SettingsService;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    middleware::from_fn_with_state,
    response::Json,
    routing::{get, post},
    Router,
};
use axum_login::AuthSession;
use axum_tower_sessions_csrf::get_or_create_token;
use basic_axum_rate_limit::{rate_limit_middleware, RateLimiter};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_sessions::Session;

const MFA_VERIFIED_KEY: &str = "mfa_verified";

pub type AdminAuthSession = AuthSession<AdminAuthBackend>;

#[derive(Clone)]
pub struct AdminState {
    pub auth_backend: AdminAuthBackend,
    pub email_service: Arc<EmailService>,
    pub settings: SettingsService,
}

pub fn admin_api_routes(
    auth_rate_limiter: RateLimiter<AppRateLimitCallbacks>,
) -> Router<AdminState> {
    // Routes that need stricter rate limiting (auth-sensitive)
    let rate_limited_routes = Router::new()
        .route("/api/admin/register", post(register))
        .route("/api/admin/login", post(login))
        .route("/api/admin/mfa/verify", post(mfa_verify))
        .route("/api/admin/forgot-password", post(forgot_password))
        .route(
            "/api/admin/forgot-password/verify-mfa",
            post(forgot_password_verify_mfa),
        )
        .route("/api/admin/reset-password", post(reset_password))
        .layer(from_fn_with_state(auth_rate_limiter, rate_limit_middleware));

    // Routes that don't need stricter rate limiting
    let standard_routes = Router::new()
        .route("/api/admin/logout", post(logout))
        .route("/api/admin/verify-email", get(verify_email))
        .route("/api/admin/me", get(me))
        .route("/api/admin/csrf-token", get(get_csrf_token))
        .route("/api/admin/change-password", post(change_password))
        .route("/api/admin/mfa/setup", post(mfa_setup))
        .route("/api/admin/mfa/confirm-setup", post(mfa_confirm_setup))
        .route("/api/admin/mfa/disable", post(mfa_disable));

    rate_limited_routes.merge(standard_routes)
}

#[derive(Deserialize)]
struct RegisterRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct RegisterResponse {
    message: String,
    email: String,
}

async fn register(
    State(state): State<AdminState>,
    Json(req): Json<RegisterRequest>,
) -> AppResult<Json<RegisterResponse>> {
    // Check if registration is enabled
    let registration_enabled = state
        .settings
        .get_bool("admin_registration_enabled", Some("system"), None)
        .await
        .unwrap_or(false);

    if !registration_enabled {
        return Err(AppError::AuthError(
            "Registration is currently disabled".to_string(),
        ));
    }

    // Create admin user
    let (admin, verification_token) = state
        .auth_backend
        .create_admin(&req.email, &req.password)
        .await
        .map_err(|e| AppError::AuthError(e.to_string()))?;

    // Send verification email
    state
        .email_service
        .send_verification_email(&admin.email, &verification_token)
        .await
        .map_err(|e| AppError::AuthError(format!("Failed to send verification email: {}", e)))?;

    Ok(Json(RegisterResponse {
        message: "Registration successful. Please check your email to verify your account."
            .to_string(),
        email: admin.email,
    }))
}

async fn login(
    mut auth_session: AdminAuthSession,
    Json(creds): Json<Credentials>,
) -> AppResult<Json<UserResponse>> {
    let user = auth_session
        .authenticate(creds)
        .await
        .map_err(|e| AppError::AuthError(e.to_string()))?
        .ok_or_else(|| AppError::AuthError("Invalid email or password".to_string()))?;

    auth_session
        .login(&user)
        .await
        .map_err(|e| AppError::AuthError(e.to_string()))?;

    // If MFA is enabled but not yet verified, indicate that MFA is required
    let mfa_required = user.totp_enabled && !user.mfa_verified;

    Ok(Json(UserResponse {
        id: user.id,
        email: user.email,
        email_verified: user.email_verified,
        totp_enabled: user.totp_enabled,
        mfa_required,
        active: user.active,
        force_password_change: user.force_password_change,
    }))
}

async fn logout(mut auth_session: AdminAuthSession) -> AppResult<StatusCode> {
    auth_session
        .logout()
        .await
        .map_err(|e| AppError::AuthError(e.to_string()))?;

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
struct VerifyQuery {
    token: String,
}

#[derive(Serialize)]
struct VerifyResponse {
    message: String,
    email: String,
}

async fn verify_email(
    State(state): State<AdminState>,
    Query(query): Query<VerifyQuery>,
) -> AppResult<Json<VerifyResponse>> {
    let admin = state
        .auth_backend
        .verify_email(&query.token)
        .await
        .map_err(|e| AppError::AuthError(e.to_string()))?;

    Ok(Json(VerifyResponse {
        message: "Email verified successfully. You can now log in.".to_string(),
        email: admin.email,
    }))
}

#[derive(Serialize)]
struct UserResponse {
    id: uuid::Uuid,
    email: String,
    email_verified: bool,
    totp_enabled: bool,
    mfa_required: bool,
    active: bool,
    force_password_change: bool,
}

async fn me(auth_session: AdminAuthSession, session: Session) -> AppResult<Json<UserResponse>> {
    let user = auth_session
        .user
        .ok_or_else(|| AppError::AuthError("Not authenticated".to_string()))?;

    // Check session for MFA verified status
    let mfa_verified = session
        .get::<bool>(MFA_VERIFIED_KEY)
        .await
        .unwrap_or(None)
        .unwrap_or(false);

    // MFA is required if TOTP is enabled and not yet verified in this session
    let mfa_required = user.totp_enabled && !mfa_verified;

    Ok(Json(UserResponse {
        id: user.id,
        email: user.email,
        email_verified: user.email_verified,
        totp_enabled: user.totp_enabled,
        mfa_required,
        active: user.active,
        force_password_change: user.force_password_change,
    }))
}

#[derive(Serialize)]
struct CsrfTokenResponse {
    token: String,
}

/// Get CSRF token for the current session
async fn get_csrf_token(session: Session) -> AppResult<Json<CsrfTokenResponse>> {
    let token = get_or_create_token(&session)
        .await
        .map_err(|e| AppError::AuthError(e))?;

    Ok(Json(CsrfTokenResponse { token }))
}

// ==================== MFA Endpoints ====================

/// Helper to get authenticated user, returning error if not logged in
fn get_authenticated_user(auth_session: &AdminAuthSession) -> AppResult<super::AdminUserAuth> {
    auth_session
        .user
        .clone()
        .ok_or_else(|| AppError::AuthError("Not authenticated".to_string()))
}

#[derive(Serialize)]
struct MfaSetupResponse {
    secret: String,
    qr_code: String,
    otpauth_url: String,
}

/// Generate a new TOTP secret and QR code for MFA setup
/// Requires full authentication (not pending MFA)
async fn mfa_setup(auth_session: AdminAuthSession) -> AppResult<Json<MfaSetupResponse>> {
    let user = get_authenticated_user(&auth_session)?;

    // Don't allow setup if MFA is already pending verification
    if user.totp_enabled && !user.mfa_verified {
        return Err(AppError::AuthError(
            "Please complete MFA verification first".to_string(),
        ));
    }

    let setup = totp::generate_secret(&user.email)
        .map_err(|e| AppError::AuthError(format!("Failed to generate TOTP secret: {}", e)))?;

    Ok(Json(MfaSetupResponse {
        secret: setup.secret_base32,
        qr_code: setup.qr_code_base64,
        otpauth_url: setup.otpauth_url,
    }))
}

#[derive(Deserialize)]
struct MfaConfirmRequest {
    secret: String,
    code: String,
}

#[derive(Serialize)]
struct MfaConfirmResponse {
    message: String,
    totp_enabled: bool,
}

/// Confirm MFA setup by verifying the code matches the secret
async fn mfa_confirm_setup(
    State(state): State<AdminState>,
    auth_session: AdminAuthSession,
    session: Session,
    Json(req): Json<MfaConfirmRequest>,
) -> AppResult<Json<MfaConfirmResponse>> {
    let user = get_authenticated_user(&auth_session)?;

    // Verify and save the secret in one step
    // First verify the code is correct
    let is_valid = totp::verify_code(&req.secret, &req.code, &user.email)
        .map_err(|e| AppError::AuthError(format!("Failed to verify code: {}", e)))?;

    if !is_valid {
        return Err(AppError::AuthError("Invalid verification code".to_string()));
    }

    // Enable TOTP for the user (save secret to DB)
    state
        .auth_backend
        .update_totp(user.id, Some(req.secret), true)
        .await
        .map_err(|e| AppError::AuthError(format!("Failed to enable MFA: {}", e)))?;

    // Mark MFA as verified in the session (user just proved they have the authenticator)
    session
        .insert(MFA_VERIFIED_KEY, true)
        .await
        .map_err(|e| AppError::AuthError(format!("Failed to update session: {}", e)))?;

    Ok(Json(MfaConfirmResponse {
        message: "MFA enabled successfully".to_string(),
        totp_enabled: true,
    }))
}

#[derive(Deserialize)]
struct MfaVerifyRequest {
    code: String,
}

#[derive(Serialize)]
struct MfaVerifyResponse {
    message: String,
    id: uuid::Uuid,
    email: String,
}

/// Verify MFA code during login (after password authentication)
async fn mfa_verify(
    State(state): State<AdminState>,
    auth_session: AdminAuthSession,
    session: Session,
    Json(req): Json<MfaVerifyRequest>,
) -> AppResult<Json<MfaVerifyResponse>> {
    let user = get_authenticated_user(&auth_session)?;

    // Must have MFA enabled
    if !user.totp_enabled {
        return Err(AppError::AuthError("MFA is not enabled".to_string()));
    }

    // Check if account is locked out
    let is_locked = state
        .auth_backend
        .is_mfa_locked(user.id)
        .await
        .map_err(|e| AppError::AuthError(e.to_string()))?;

    if is_locked {
        return Err(AppError::AuthError(
            "Account is temporarily locked due to too many failed MFA attempts. Please try again later.".to_string(),
        ));
    }

    // Check if already verified in this session
    let already_verified = session
        .get::<bool>(MFA_VERIFIED_KEY)
        .await
        .unwrap_or(None)
        .unwrap_or(false);

    if already_verified {
        return Err(AppError::AuthError("MFA already verified".to_string()));
    }

    // Get the decrypted TOTP secret from database
    let totp_secret = state
        .auth_backend
        .get_totp_secret(user.id)
        .await
        .map_err(|e| AppError::AuthError(e.to_string()))?
        .ok_or_else(|| AppError::AuthError("TOTP secret not configured".to_string()))?;

    // Verify the code
    let is_valid = totp::verify_code(&totp_secret, &req.code, &user.email)
        .map_err(|e| AppError::AuthError(format!("Failed to verify code: {}", e)))?;

    if !is_valid {
        // Record failed attempt
        let (attempts, is_now_locked) = state
            .auth_backend
            .record_mfa_failure(user.id)
            .await
            .map_err(|e| AppError::AuthError(e.to_string()))?;

        if is_now_locked {
            tracing::warn!(
                "MFA lockout triggered for user {} after {} failed attempts",
                user.email,
                attempts
            );
            return Err(AppError::AuthError(
                "Too many failed attempts. Account is now temporarily locked.".to_string(),
            ));
        }

        return Err(AppError::AuthError("Invalid verification code".to_string()));
    }

    // Reset failed attempts on success
    state
        .auth_backend
        .reset_mfa_failures(user.id)
        .await
        .map_err(|e| AppError::AuthError(e.to_string()))?;

    // Store MFA verified status in session
    session
        .insert(MFA_VERIFIED_KEY, true)
        .await
        .map_err(|e| AppError::AuthError(format!("Failed to update session: {}", e)))?;

    Ok(Json(MfaVerifyResponse {
        message: "MFA verified successfully".to_string(),
        id: user.id,
        email: user.email,
    }))
}

#[derive(Deserialize)]
struct MfaDisableRequest {
    password: String,
}

#[derive(Serialize)]
struct MfaDisableResponse {
    message: String,
    totp_enabled: bool,
}

/// Disable MFA for the user (requires password confirmation)
async fn mfa_disable(
    State(state): State<AdminState>,
    auth_session: AdminAuthSession,
    session: Session,
    Json(req): Json<MfaDisableRequest>,
) -> AppResult<Json<MfaDisableResponse>> {
    let user = get_authenticated_user(&auth_session)?;

    // Must be fully authenticated - check session for MFA verification
    let mfa_verified = session
        .get::<bool>(MFA_VERIFIED_KEY)
        .await
        .unwrap_or(None)
        .unwrap_or(false);

    if user.totp_enabled && !mfa_verified {
        return Err(AppError::AuthError(
            "Please complete MFA verification first".to_string(),
        ));
    }

    // Get the user from database to verify password
    let admin = state
        .auth_backend
        .get_admin_by_id(user.id)
        .await
        .map_err(|e| AppError::AuthError(e.to_string()))?
        .ok_or_else(|| AppError::AuthError("User not found".to_string()))?;

    // Verify password
    let password_valid = verify_password(&req.password, &admin.password_hash)
        .map_err(|e| AppError::AuthError(e.to_string()))?;

    if !password_valid {
        return Err(AppError::AuthError("Invalid password".to_string()));
    }

    // Disable TOTP
    state
        .auth_backend
        .update_totp(user.id, None, false)
        .await
        .map_err(|e| AppError::AuthError(format!("Failed to disable MFA: {}", e)))?;

    // Remove MFA verified flag from session (no longer needed)
    session
        .remove::<bool>(MFA_VERIFIED_KEY)
        .await
        .map_err(|e| AppError::AuthError(format!("Failed to update session: {}", e)))?;

    Ok(Json(MfaDisableResponse {
        message: "MFA disabled successfully".to_string(),
        totp_enabled: false,
    }))
}

// ==================== Password Management Endpoints ====================

#[derive(Deserialize)]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

#[derive(Serialize)]
struct ChangePasswordResponse {
    message: String,
}

/// Change password for the authenticated user (requires current password)
async fn change_password(
    State(state): State<AdminState>,
    auth_session: AdminAuthSession,
    session: Session,
    Json(req): Json<ChangePasswordRequest>,
) -> AppResult<Json<ChangePasswordResponse>> {
    let user = get_authenticated_user(&auth_session)?;

    // Must be fully authenticated (MFA verified if enabled)
    let mfa_verified = session
        .get::<bool>(MFA_VERIFIED_KEY)
        .await
        .unwrap_or(None)
        .unwrap_or(false);

    if user.totp_enabled && !mfa_verified {
        return Err(AppError::AuthError(
            "Please complete MFA verification first".to_string(),
        ));
    }

    // Get the user from database to verify current password
    let admin = state
        .auth_backend
        .get_admin_by_id(user.id)
        .await
        .map_err(|e| AppError::AuthError(e.to_string()))?
        .ok_or_else(|| AppError::AuthError("User not found".to_string()))?;

    // Verify current password
    let password_valid = verify_password(&req.current_password, &admin.password_hash)
        .map_err(|e| AppError::AuthError(e.to_string()))?;

    if !password_valid {
        return Err(AppError::AuthError(
            "Current password is incorrect".to_string(),
        ));
    }

    // Validate new password
    if let Err(errors) = PasswordValidator::validate(&req.new_password, &user.email) {
        return Err(AppError::ValidationError(errors.join("; ")));
    }

    // Change the password (force_change = false since user is changing their own)
    state
        .auth_backend
        .change_password(user.id, &req.new_password, false)
        .await
        .map_err(|e| AppError::AuthError(format!("Failed to change password: {}", e)))?;

    // Send notification email
    if let Err(e) = state
        .email_service
        .send_password_changed_notification(&user.email, false)
        .await
    {
        tracing::warn!("Failed to send password change notification: {}", e);
    }

    Ok(Json(ChangePasswordResponse {
        message: "Password changed successfully".to_string(),
    }))
}

#[derive(Deserialize)]
struct ForgotPasswordRequest {
    email: String,
}

#[derive(Serialize)]
struct ForgotPasswordResponse {
    requires_mfa: bool,
    message: String,
}

/// Request password reset (always returns requires_mfa: true for enumeration protection)
async fn forgot_password(
    State(state): State<AdminState>,
    Json(req): Json<ForgotPasswordRequest>,
) -> AppResult<Json<ForgotPasswordResponse>> {
    // Check if user exists (we'll handle this silently for enumeration protection)
    let admin = state
        .auth_backend
        .get_admin_by_email(&req.email)
        .await
        .map_err(|e| AppError::AuthError(e.to_string()))?;

    // If user exists, check for cooldown
    if let Some(ref user) = admin {
        if let Some(expires_at) = user.password_reset_token_expires_at {
            if chrono::Utc::now() < expires_at.with_timezone(&chrono::Utc) {
                return Err(AppError::AuthError(
                    "Password reset already requested. Please wait for the current request to expire.".to_string(),
                ));
            }
        }
    }

    // Always return requires_mfa: true regardless of whether user exists or has MFA
    // This prevents email enumeration attacks
    Ok(Json(ForgotPasswordResponse {
        requires_mfa: true,
        message: "Please enter your MFA code to continue".to_string(),
    }))
}

#[derive(Deserialize)]
struct ForgotPasswordVerifyMfaRequest {
    email: String,
    code: String,
}

#[derive(Serialize)]
struct ForgotPasswordVerifyMfaResponse {
    message: String,
}

/// Verify MFA for password reset (uses strict verification with zero grace period)
async fn forgot_password_verify_mfa(
    State(state): State<AdminState>,
    Json(req): Json<ForgotPasswordVerifyMfaRequest>,
) -> AppResult<Json<ForgotPasswordVerifyMfaResponse>> {
    // Get user by email
    let admin = state
        .auth_backend
        .get_admin_by_email(&req.email)
        .await
        .map_err(|e| AppError::AuthError(e.to_string()))?;

    // For enumeration protection: if user doesn't exist, return same error as wrong MFA code
    let admin = match admin {
        Some(a) => a,
        None => {
            return Err(AppError::AuthError("Invalid verification code".to_string()));
        }
    };

    // Re-check cooldown
    if let Some(expires_at) = admin.password_reset_token_expires_at {
        if chrono::Utc::now() < expires_at.with_timezone(&chrono::Utc) {
            return Err(AppError::AuthError(
                "Password reset already requested. Please wait for the current request to expire."
                    .to_string(),
            ));
        }
    }

    // Check if user has MFA enabled
    let totp_enabled = admin.totp_enabled.unwrap_or(false);
    if !totp_enabled {
        // User has no MFA - return same error for enumeration protection
        // Rate limiter will block repeated attempts
        return Err(AppError::AuthError("Invalid verification code".to_string()));
    }

    // Get the decrypted TOTP secret
    let totp_secret = state
        .auth_backend
        .get_totp_secret(admin.id)
        .await
        .map_err(|e| AppError::AuthError(e.to_string()))?
        .ok_or_else(|| AppError::AuthError("Invalid verification code".to_string()))?;

    // Verify with strict mode (zero grace period)
    let is_valid = totp::verify_code_strict(&totp_secret, &req.code, &admin.email)
        .map_err(|e| AppError::AuthError(format!("Failed to verify code: {}", e)))?;

    if !is_valid {
        return Err(AppError::AuthError("Invalid verification code".to_string()));
    }

    // Create reset token
    let reset_token = state
        .auth_backend
        .create_password_reset_token(&req.email)
        .await
        .map_err(|e| AppError::AuthError(e.to_string()))?
        .ok_or_else(|| AppError::AuthError("Invalid verification code".to_string()))?;

    // Send password reset email
    state
        .email_service
        .send_password_reset_email(&admin.email, &reset_token)
        .await
        .map_err(|e| AppError::AuthError(format!("Failed to send reset email: {}", e)))?;

    Ok(Json(ForgotPasswordVerifyMfaResponse {
        message: "Password reset email sent. Please check your inbox.".to_string(),
    }))
}

#[derive(Deserialize)]
struct ResetPasswordRequest {
    token: String,
    new_password: String,
}

#[derive(Serialize)]
struct ResetPasswordResponse {
    message: String,
}

/// Complete password reset using token
async fn reset_password(
    State(state): State<AdminState>,
    Json(req): Json<ResetPasswordRequest>,
) -> AppResult<Json<ResetPasswordResponse>> {
    // Validate the token first to get the user email for password validation
    let admin = state
        .auth_backend
        .validate_reset_token(&req.token)
        .await
        .map_err(|e| AppError::AuthError(e.to_string()))?
        .ok_or_else(|| {
            AppError::AuthError("Invalid or expired password reset token".to_string())
        })?;

    // Validate new password
    if let Err(errors) = PasswordValidator::validate(&req.new_password, &admin.email) {
        return Err(AppError::ValidationError(errors.join("; ")));
    }

    // Reset the password
    state
        .auth_backend
        .reset_password_with_token(&req.token, &req.new_password)
        .await
        .map_err(|e| AppError::AuthError(e.to_string()))?;

    // Send notification email
    if let Err(e) = state
        .email_service
        .send_password_changed_notification(&admin.email, false)
        .await
    {
        tracing::warn!("Failed to send password change notification: {}", e);
    }

    Ok(Json(ResetPasswordResponse {
        message: "Password reset successfully. You can now log in with your new password."
            .to_string(),
    }))
}
