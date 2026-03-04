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

use axum::{
    extract::Path,
    http::{header, StatusCode},
    middleware::{from_fn, from_fn_with_state},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use axum_login::AuthManagerLayerBuilder;
use std::{env, sync::Arc};
use time::Duration as TimeDuration;
use tower::ServiceBuilder;
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer, trace::TraceLayer};
use tower_sessions::{cookie::SameSite, ExpiredDeletion, Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;

mod admin;
mod app;
mod contact;
mod crypto;
mod database;
mod docx;
mod email;
mod entities;
mod errors;
mod metrics;
mod middleware;
mod migration;
mod oidc;
mod s3;
mod security_callbacks;
mod settings;
mod subscribe;

use self::middleware::{access_log_middleware, csrf_middleware, require_admin_auth, require_administrator};
use app::AppState;
use basic_axum_rate_limit::{
    rate_limit_middleware, security_context_middleware_with_config, IpExtractionStrategy,
    SecurityContextConfig,
};
use errors::{AppError, AppResult};

#[cfg(test)]
mod tests;

async fn serve_access(
    axum::extract::State(state): axum::extract::State<AppState>,
    Path(code): Path<String>,
) -> AppResult<Html<String>> {
    if !state.is_valid_code(&code).await.unwrap_or(false) {
        return Err(AppError::InvalidAccess);
    }

    tracing::info!("Valid access code used: {}", code);

    let html_bytes =
        state.s3.get_file(&code, "index.html").await.map_err(|e| {
            AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::NotFound, e))
        })?;

    let html_content = String::from_utf8(html_bytes).map_err(|e| {
        AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    })?;

    Ok(Html(html_content))
}

async fn download_access(
    axum::extract::State(state): axum::extract::State<AppState>,
    Path(code): Path<String>,
) -> AppResult<impl IntoResponse> {
    // Get access code details to retrieve custom filename
    let access_code = state
        .get_access_code(&code)
        .await
        .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::NotFound, e)))?
        .ok_or(AppError::InvalidAccess)?;

    tracing::info!("Valid access code used for download: {}", code);

    let docx_content = state
        .s3
        .get_file(&code, "Document.docx")
        .await
        .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::NotFound, e)))?;

    // Use custom filename if set, otherwise use default
    let filename = access_code
        .download_filename
        .unwrap_or_else(|| "Grant_DeFayette_Document".to_string());

    let content_disposition = format!("attachment; filename=\"{}.docx\"", filename);

    let response = (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                    .to_owned(),
            ),
            (header::CONTENT_DISPOSITION, content_disposition),
        ],
        docx_content,
    );

    Ok(response)
}

async fn health_check() -> &'static str {
    "OK"
}

async fn serve_robots() -> impl IntoResponse {
    let site_url = env::var("SITE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let robots_content = format!(
        "User-agent: *\nAllow: /\n\nSitemap: {}/sitemap-index.xml",
        site_url
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain")],
        robots_content,
    )
}

async fn serve_favicon_png() -> AppResult<impl IntoResponse> {
    let content = tokio::fs::read("assets/icons/favicon.png")
        .await
        .map_err(AppError::FileSystem)?;

    let response = (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/png")],
        content,
    );

    Ok(response)
}

async fn serve_favicon_svg() -> AppResult<impl IntoResponse> {
    let content = tokio::fs::read("public-assets/favicon.svg")
        .await
        .map_err(AppError::FileSystem)?;
    let response = (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/svg+xml")],
        content,
    );

    Ok(response)
}

async fn serve_admin_spa() -> AppResult<impl IntoResponse> {
    let html_content = tokio::fs::read_to_string("admin-assets/index.html")
        .await
        .map_err(AppError::FileSystem)?;

    let response = (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html_content,
    );

    Ok(response)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Check for migration command
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "migrate" {
        match run_migrations_sync().await {
            Ok(_) => {
                tracing::info!("Database migrations completed successfully");
                return Ok(());
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Database migration failed: {}", e));
            }
        }
    }

    // Register prometheus metrics
    metrics::register_metrics();

    // Create shared app state with database connection
    let state = AppState::new().await?;

    // Setup PostgreSQL-backed session store for admin authentication
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let session_pool = sqlx::PgPool::connect(&database_url)
        .await
        .map_err(|e| anyhow::anyhow!("Session store pool connection failed: {}", e))?;
    let session_store = PostgresStore::new(session_pool);
    session_store
        .migrate()
        .await
        .map_err(|e| anyhow::anyhow!("Session table migration failed: {}", e))?;

    // Spawn background task to clean up expired sessions
    let _deletion_task = tokio::task::spawn(
        session_store
            .clone()
            .continuously_delete_expired(std::time::Duration::from_secs(60)),
    );

    // Session expiry: 1 day of inactivity for better security
    // SameSite::Lax is required for OIDC - the redirect back from the IdP is a
    // cross-site top-level navigation, and Strict would drop the session cookie.
    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(TimeDuration::days(1)))
        .with_same_site(SameSite::Lax);

    // Setup admin auth backend
    let admin_backend = admin::AdminAuthBackend::new(state.db.clone());
    let auth_layer =
        AuthManagerLayerBuilder::new(admin_backend.clone(), session_layer.clone()).build();

    // Setup email service
    let email_service = Arc::new(email::EmailService::new(state.settings.clone()).await?);

    // Create admin state
    let oidc_enabled = state.oidc.config.enabled;
    let oidc_account_url = if oidc_enabled {
        Some(format!(
            "{}/account",
            state.oidc.config.issuer_url.trim_end_matches('/')
        ))
    } else {
        None
    };
    let admin_state = admin::routes::AdminState {
        auth_backend: admin_backend.clone(),
        email_service: email_service.clone(),
        settings: state.settings.clone(),
        oidc_enabled,
        oidc_account_url,
    };

    // Build admin routes (pass auth rate limiter for sensitive routes)
    let admin_routes = admin::routes::admin_api_routes(state.auth_rate_limiter.clone())
        .with_state(admin_state)
        .layer(from_fn(csrf_middleware))
        .layer(auth_layer.clone());

    // Build OIDC routes (login redirect and callback)
    let oidc_state = admin::oidc_routes::OidcState {
        oidc_service: state.oidc.clone(),
        db: state.db.clone(),
    };
    let oidc_routes = Router::new()
        .route("/api/admin/oidc/login", get(admin::oidc_routes::oidc_login))
        .route(
            "/api/admin/oidc/callback",
            get(admin::oidc_routes::oidc_callback),
        )
        .with_state(oidc_state)
        .layer(auth_layer.clone());

    // Build access code management routes
    let access_code_state = admin::access_codes::AccessCodeState {
        db: state.db.clone(),
        s3: state.s3.clone(),
    };
    let access_code_routes = admin::access_codes::access_code_routes()
        .with_state(access_code_state)
        .layer(from_fn(require_administrator))
        .layer(from_fn(require_admin_auth))
        .layer(from_fn(csrf_middleware))
        .layer(auth_layer.clone());

    // Build access log management routes
    let access_log_state = admin::access_logs::AccessLogState {
        db: state.db.clone(),
    };
    let access_log_routes = admin::access_logs::access_log_routes()
        .with_state(access_log_state)
        .layer(from_fn(require_administrator))
        .layer(from_fn(require_admin_auth))
        .layer(from_fn(csrf_middleware))
        .layer(auth_layer.clone());

    // Build admin user management routes
    let admin_user_state = admin::admin_users::AdminUserState {
        db: state.db.clone(),
        auth_backend: admin_backend.clone(),
        email_service: email_service.clone(),
    };
    let admin_user_routes = admin::admin_users::admin_user_routes()
        .with_state(admin_user_state)
        .layer(from_fn(require_administrator))
        .layer(from_fn(require_admin_auth))
        .layer(from_fn(csrf_middleware))
        .layer(auth_layer.clone());

    // Build settings management routes
    let settings_state = admin::settings::SettingsState {
        settings: state.settings.clone(),
    };
    let settings_routes = admin::settings::settings_routes()
        .with_state(settings_state)
        .layer(from_fn(require_administrator))
        .layer(from_fn(require_admin_auth))
        .layer(from_fn(csrf_middleware))
        .layer(auth_layer.clone());

    let contact_state = contact::ContactState {
        email_service: email_service.clone(),
        callbacks: state.callbacks.clone(),
    };
    let contact_routes = contact::contact_routes()
        .with_state(contact_state)
        .layer(from_fn(csrf_middleware));

    let subscribe_state = subscribe::SubscribeState {
        email_service: email_service.clone(),
        callbacks: state.callbacks.clone(),
        db: state.db.clone(),
    };
    let subscribe_routes = subscribe::subscribe_routes()
        .with_state(subscribe_state)
        .layer(from_fn(csrf_middleware));

    // Build the application with routes and middleware stack
    let app = Router::new()
        // API routes (highest priority)
        .merge(admin_routes)
        .merge(oidc_routes)
        .merge(access_code_routes)
        .merge(access_log_routes)
        .merge(admin_user_routes)
        .merge(settings_routes)
        .merge(contact_routes)
        .merge(subscribe_routes)
        // Special routes
        .route("/favicon.png", get(serve_favicon_png))
        .route("/favicon.svg", get(serve_favicon_svg))
        .route("/robots.txt", get(serve_robots))
        .route("/health", get(health_check))
        .route("/metrics", get(metrics::metrics_handler))
        .route("/access/{code}", get(serve_access))
        .route("/access/{code}/download", get(download_access))
        .route("/document/{code}", get(serve_access))
        .route("/document/{code}/download", get(download_access))
        // Admin panel
        .nest_service(
            "/admin/assets",
            tower::ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::if_not_present(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static("public, max-age=86400"), // 1 day
                ))
                .service(ServeDir::new("./admin-assets/assets").precompressed_gzip()),
        )
        .route("/admin", get(serve_admin_spa))
        .route("/admin/{*path}", get(serve_admin_spa))
        // Code-gated document assets (CSS, JS, icons)
        .nest_service(
            "/assets",
            tower::ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::if_not_present(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static("public, max-age=86400"), // 1 day
                ))
                .service(ServeDir::new("./assets").precompressed_gzip()),
        )
        // Public Astro site - serve from root as fallback
        .fallback_service(
            tower::ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::if_not_present(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static("public, max-age=0"), // 1 minute
                ))
                .service(ServeDir::new("./public-assets").precompressed_gzip()),
        )
        .with_state(state.clone());

    // Configure IP extraction strategy based on environment
    // DEV_MODE=true uses socket address (direct connections without proxy)
    // Production (default) expects X-Forwarded-For from a single proxy
    let ip_strategy = if env::var("DEV_MODE").unwrap_or_default() == "true" {
        tracing::info!("DEV_MODE enabled: using socket address for IP extraction");
        IpExtractionStrategy::SocketAddr
    } else {
        tracing::info!("Production mode: using X-Forwarded-For header");
        IpExtractionStrategy::default()
    };
    let security_config = SecurityContextConfig::new().with_ip_extraction(ip_strategy);

    let app = app.layer(
        ServiceBuilder::new()
            .layer(from_fn_with_state(
                security_config,
                security_context_middleware_with_config,
            ))
            .layer(from_fn_with_state(
                state.rate_limiter.clone(),
                rate_limit_middleware,
            ))
            .layer(from_fn_with_state(state.clone(), access_log_middleware))
            .layer(TraceLayer::new_for_http()),
    );

    let cache_cleanup_limiter = state.rate_limiter.clone();
    let auth_cache_cleanup_limiter = state.auth_rate_limiter.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            cache_cleanup_limiter.cleanup_cache();
            auth_cache_cleanup_limiter.cleanup_cache();
        }
    });

    // Metrics refresh task - updates system-level gauges periodically
    let metrics_limiter = state.rate_limiter.clone();
    let metrics_auth_limiter = state.auth_rate_limiter.clone();
    let metrics_db = state.db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
        loop {
            interval.tick().await;
            metrics::refresh_system_metrics(&metrics_limiter, &metrics_auth_limiter, &metrics_db)
                .await;
        }
    });

    let db_cleanup_callbacks = state.callbacks.clone();
    let retention_days = env::var("ACCESS_LOG_RETENTION_DAYS")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<i64>()
        .unwrap_or(1);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;

            // Clean up database logs
            if let Err(e) = db_cleanup_callbacks
                .cleanup_database_logs(retention_days)
                .await
            {
                tracing::error!("Failed to cleanup database logs: {}", e);
            }
        }
    });

    // Determine the bind address
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap_or(3000);

    let addr = format!("0.0.0.0:{}", port);

    // Production environments will likely want to set RUST_LOG=warn
    // unless they want to see very verbose logs
    tracing::info!("Server starting on {}", addr);
    tracing::info!("Access at: http://localhost:{}/access/{{your-code}}", port);
    tracing::info!("RUST_LOG environment variable: {:?}", env::var("RUST_LOG"));

    // Start the server with connection info support
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}", addr, e))?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}

async fn run_migrations_sync() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Running database migrations...");

    let db = database::establish_connection().await?;
    tracing::info!("Database connection established for migrations");

    database::run_migrations(&db).await?;
    tracing::info!("Database migrations completed successfully");

    database::close_connection(db).await?;
    Ok(())
}
