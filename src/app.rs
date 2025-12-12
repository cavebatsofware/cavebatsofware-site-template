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

use crate::entities::{access_code, AccessCode};
use crate::s3::S3Service;
use crate::security_callbacks::AppRateLimitCallbacks;
use crate::settings::SettingsService;
use anyhow::Result;
use basic_axum_rate_limit::{RateLimitConfig, RateLimiter, RequestScreener, ScreeningConfig};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::env;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub rate_limiter: RateLimiter<AppRateLimitCallbacks>,
    pub auth_rate_limiter: RateLimiter<AppRateLimitCallbacks>,
    pub callbacks: AppRateLimitCallbacks,
    pub settings: SettingsService,
    pub s3: S3Service,
    pub enable_logging: bool,
    pub log_successful_attempts: bool,
}

impl AppState {
    pub async fn new() -> Result<Self> {
        let db = crate::database::establish_connection()
            .await
            .map_err(|e| anyhow::anyhow!("Database connection failed: {}", e))?;

        let rate_limit_per_minute = env::var("RATE_LIMIT_PER_MINUTE")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .unwrap_or(30);

        let block_duration_minutes = env::var("BLOCK_DURATION_MINUTES")
            .unwrap_or_else(|_| "15".to_string())
            .parse()
            .unwrap_or(15);

        let enable_logging = env::var("ENABLE_ACCESS_LOGGING")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        let log_successful_attempts = env::var("LOG_SUCCESSFUL_ATTEMPTS")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        let config = RateLimitConfig::new(
            rate_limit_per_minute,
            std::time::Duration::from_secs(block_duration_minutes * 60),
        )
        .with_cache_refund_ratio(0.8);

        let callbacks =
            AppRateLimitCallbacks::new(db.clone(), enable_logging, log_successful_attempts);

        let screening_config = ScreeningConfig::new()
            .with_path_patterns(vec![
                // PHP attacks
                r"\.php\d?$".to_string(),
                r"/vendor/".to_string(),
                r"/phpunit/".to_string(),
                r"eval-stdin".to_string(),
                // .NET attacks
                r"\.aspx?$".to_string(),
                r"\.axd$".to_string(),
                r"/Telerik\.".to_string(),
                r"\.ini$".to_string(),
                // Java attacks
                r"\.jsp$".to_string(),
                r"hjasperserver".to_string(),
                r"\.jar$".to_string(),
                // Git/config exposure
                r"/\.git/".to_string(),
                r"/\.env".to_string(),
                r"/\.aws/".to_string(),
                r"/\.ssh/".to_string(),
                // Windows/RDP
                r"/RDWeb/".to_string(),
                // Router/device admin panels
                r"/webfig/".to_string(),
                r"/ssi\.cgi".to_string(),
                r"\.cc$".to_string(),
                // Monitoring tools
                r"/zabbix/".to_string(),
                // WordPress
                r"/wp-admin".to_string(),
                r"/wp-content".to_string(),
                r"/wp-includes".to_string(),
                r"/xmlrpc\.php".to_string(),
                // Router exploits
                r"\.cgi$".to_string(),
                r"/CSCOL/".to_string(),
                r"/passwd/".to_string(),
                r"/\/sap\//".to_string(),
                r"(\$|%24)(\{|%7B)".to_string(), // JNDI injection patterns
            ])
            .with_user_agent_patterns(vec![
                "libredtail-http".to_string(),
                "zgrab".to_string(),
                "masscan".to_string(),
                "nuclei".to_string(),
                "sqlmap".to_string(),
                "nikto".to_string(),
                "nmap".to_string(),
                "dirbuster".to_string(),
                "gobuster".to_string(),
                "wfuzz".to_string(),
                "ffuf".to_string(),
                r"\$\{\$\{:-j\}\$\{:-n\}\$\{:-d\}\$\{:-i\}".to_string(), // JNDI injection pattern
                "jndi".to_string(),
            ]);

        let screener =
            RequestScreener::new(&screening_config).expect("Failed to compile screening patterns");
        let rate_limiter = RateLimiter::new(config, callbacks.clone()).with_screener(screener);

        // Auth-specific rate limiter: 5 req/min, 30 min block
        // This is much stricter to protect against brute-force attacks
        let auth_rate_limit_per_minute = env::var("AUTH_RATE_LIMIT_PER_MINUTE")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .unwrap_or(5);

        let auth_block_duration_minutes = env::var("AUTH_BLOCK_DURATION_MINUTES")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .unwrap_or(30);

        let auth_config = RateLimitConfig::new(
            auth_rate_limit_per_minute,
            std::time::Duration::from_secs(auth_block_duration_minutes * 60),
        );

        let auth_rate_limiter = RateLimiter::new(auth_config, callbacks.clone());

        let settings = SettingsService::new(db.clone());
        let s3 = S3Service::new().await?;

        tracing::info!("Database connected and services initialized");
        tracing::info!(
            "Rate limit config: {}/min, block_duration={}min, logging_enabled={}, log_successful={}",
            rate_limit_per_minute,
            block_duration_minutes,
            enable_logging,
            log_successful_attempts
        );
        tracing::info!(
            "Auth rate limit config: {}/min, block_duration={}min",
            auth_rate_limit_per_minute,
            auth_block_duration_minutes
        );

        Ok(AppState {
            db,
            rate_limiter,
            auth_rate_limiter,
            callbacks,
            settings,
            s3,
            enable_logging,
            log_successful_attempts,
        })
    }

    /// Check if code is valid in database and increment usage count
    pub async fn is_valid_code(&self, code: &str) -> Result<bool> {
        // Check database
        let db_code = AccessCode::find()
            .filter(access_code::Column::Code.eq(code))
            .one(&self.db)
            .await?;

        if let Some(db_code) = db_code {
            // Check if expired
            if let Some(expires_at) = db_code.expires_at {
                if expires_at.with_timezone(&Utc) < Utc::now() {
                    return Ok(false); // Expired
                }
            }

            // Increment usage count and update last_used_at
            let current_count = db_code.usage_count;
            let mut active_code: access_code::ActiveModel = db_code.into();
            active_code.usage_count = Set(current_count + 1);
            active_code.last_used_at = Set(Some(Utc::now().into()));
            active_code.update(&self.db).await?;

            return Ok(true);
        }

        Ok(false)
    }

    /// Get access code by code string (without incrementing usage count)
    pub async fn get_access_code(&self, code: &str) -> Result<Option<access_code::Model>> {
        let db_code = AccessCode::find()
            .filter(access_code::Column::Code.eq(code))
            .one(&self.db)
            .await?;

        if let Some(db_code) = &db_code {
            // Check if expired
            if let Some(expires_at) = db_code.expires_at {
                if expires_at.with_timezone(&Utc) < Utc::now() {
                    return Ok(None); // Expired
                }
            }
        }

        Ok(db_code)
    }
}
