{% if license_style == "gpl-3.0" -%}
/*  This file is part of {{project-name}}
 *  Copyright (C) {{copyright-year}} {{author}}
 *
 *  {{project-name}} is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, version 3 of the License (GPL-3.0-only).
 *
 *  {{project-name}} is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with {{project-name}}.  If not, see <https://www.gnu.org/licenses/gpl-3.0.html>.
 */
{%- elsif license_style == "bsd-3-clause" -%}
/*  This file is part of {{project-name}}
 *  Copyright (C) {{copyright-year}} {{author}}
 *
 *  Licensed under the BSD 3-Clause License.
 *  See <https://opensource.org/licenses/BSD-3-Clause> for full license text.
 */
{%- endif %}
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AccessCodes::Table)
                    .if_not_exists()
                    .col(uuid(AccessCodes::Id).primary_key())
                    .col(string_uniq(AccessCodes::Code))
                    .col(string(AccessCodes::Name))
                    .col(timestamp_with_time_zone_null(AccessCodes::ExpiresAt))
                    .col(
                        timestamp_with_time_zone(AccessCodes::CreatedAt)
                            .default(Expr::current_timestamp()),
                    )
                    .col(uuid(AccessCodes::CreatedBy))
                    .to_owned(),
            )
            .await?;

        // Create index on code for faster lookups
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_access_codes_code")
                    .table(AccessCodes::Table)
                    .col(AccessCodes::Code)
                    .to_owned(),
            )
            .await?;

        // Create index on expires_at for cleanup queries
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_access_codes_expires_at")
                    .table(AccessCodes::Table)
                    .col(AccessCodes::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AccessCodes::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AccessCodes {
    Table,
    Id,
    Code,
    Name,
    ExpiresAt,
    CreatedAt,
    CreatedBy,
}
