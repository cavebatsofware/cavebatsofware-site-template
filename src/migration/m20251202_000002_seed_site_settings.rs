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

use sea_orm_migration::prelude::*;
use uuid::Uuid;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Insert default site settings with environment variable fallback values
        let settings = vec![
            (
                "site_name",
                std::env::var("SITE_NAME").unwrap_or_else(|_| "Personal Site".to_string()),
            ),
            (
                "contact_email",
                std::env::var("CONTACT_EMAIL")
                    .unwrap_or_else(|_| "contact@example.com".to_string()),
            ),
            (
                "from_email",
                std::env::var("AWS_SES_FROM_EMAIL")
                    .unwrap_or_else(|_| "noreply@example.com".to_string()),
            ),
        ];

        for (key, value) in settings {
            manager
                .exec_stmt(
                    Query::insert()
                        .into_table(Settings::Table)
                        .columns([
                            Settings::Id,
                            Settings::Key,
                            Settings::Value,
                            Settings::Category,
                        ])
                        .values_panic([
                            Uuid::new_v4().into(),
                            key.into(),
                            value.into(),
                            "site".into(),
                        ])
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove the seeded settings
        manager
            .exec_stmt(
                Query::delete()
                    .from_table(Settings::Table)
                    .and_where(Expr::col(Settings::Key).eq("site_name"))
                    .to_owned(),
            )
            .await?;

        manager
            .exec_stmt(
                Query::delete()
                    .from_table(Settings::Table)
                    .and_where(Expr::col(Settings::Key).eq("contact_email"))
                    .to_owned(),
            )
            .await?;

        manager
            .exec_stmt(
                Query::delete()
                    .from_table(Settings::Table)
                    .and_where(Expr::col(Settings::Key).eq("from_email"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Settings {
    Table,
    Id,
    Key,
    Value,
    Category,
}
