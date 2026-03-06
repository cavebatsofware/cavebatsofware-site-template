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

use sea_orm::*;
use std::time::Duration;
use tracing::log::LevelFilter;

pub async fn establish_connection() -> Result<DatabaseConnection, DbErr> {
    establish_connection_with_retry(5, Duration::from_secs(5)).await
}

/// Establish database connection with retry logic for production resilience
pub async fn establish_connection_with_retry(
    max_retries: u32,
    retry_delay: Duration,
) -> Result<DatabaseConnection, DbErr> {
    let database_url = get_database_url();

    let mut opt = ConnectOptions::new(database_url.clone());
    opt.max_connections(20)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(5))
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(1800))
        .sqlx_logging(true) // Enable query logging in debug builds
        .sqlx_logging_level(LevelFilter::Debug);

    let mut attempts = 0;
    let mut last_error = None;

    while attempts < max_retries {
        attempts += 1;

        match Database::connect(opt.clone()).await {
            Ok(conn) => {
                tracing::info!(
                    "✅ Database connected successfully on attempt {}/{}",
                    attempts,
                    max_retries
                );

                // Verify connection with a ping
                if let Err(e) = conn.ping().await {
                    tracing::warn!("Database ping failed after connection: {}", e);
                    last_error = Some(e);

                    if attempts < max_retries {
                        tracing::info!("⏳ Retrying in {:?}...", retry_delay);
                        tokio::time::sleep(retry_delay).await;
                        continue;
                    }
                } else {
                    return Ok(conn);
                }
            }
            Err(e) => {
                tracing::warn!(
                    "❌ Database connection attempt {}/{} failed: {}",
                    attempts,
                    max_retries,
                    e
                );
                last_error = Some(e);

                if attempts < max_retries {
                    tracing::info!("⏳ Retrying in {:?}...", retry_delay);
                    tokio::time::sleep(retry_delay).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        DbErr::Conn(sea_orm::RuntimeErr::Internal(
            "Failed to connect to database after maximum retries".to_string(),
        ))
    }))
}

fn get_database_url() -> String {
    dotenvy::var("DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "DATABASE_URL environment variable must be set and should not use insecure defaults."
        );
    })
}

/// Establish test database connection (used in tests)
#[cfg(test)]
pub async fn establish_test_connection() -> Result<DatabaseConnection, DbErr> {
    let database_url = get_test_database_url();

    let mut opt = ConnectOptions::new(database_url);
    opt.max_connections(10)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(600));

    Database::connect(opt).await
}

#[cfg(test)]
fn get_test_database_url() -> String {
    dotenvy::var("TEST_DATABASE_URL").unwrap()
}

/// Close database connection gracefully
pub async fn close_connection(db: DatabaseConnection) -> Result<(), DbErr> {
    db.close().await
}

/// Run pending migrations
pub async fn run_migrations(db: &DatabaseConnection) -> Result<(), DbErr> {
    use crate::migration::{Migrator, MigratorTrait};

    // Apply all pending migrations
    Migrator::up(db, None).await
}
