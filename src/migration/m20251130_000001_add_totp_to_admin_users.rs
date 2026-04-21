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
        // Add totp_secret column (nullable, stores base32-encoded secret)
        manager
            .alter_table(
                Table::alter()
                    .table(AdminUsers::Table)
                    .add_column(string_null(AdminUsers::TotpSecret))
                    .to_owned(),
            )
            .await?;

        // Add totp_enabled column (boolean, defaults to false)
        manager
            .alter_table(
                Table::alter()
                    .table(AdminUsers::Table)
                    .add_column(boolean_null(AdminUsers::TotpEnabled).default(false))
                    .to_owned(),
            )
            .await?;

        // Add totp_enabled_at column (nullable timestamp)
        manager
            .alter_table(
                Table::alter()
                    .table(AdminUsers::Table)
                    .add_column(timestamp_with_time_zone_null(AdminUsers::TotpEnabledAt))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(AdminUsers::Table)
                    .drop_column(AdminUsers::TotpSecret)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(AdminUsers::Table)
                    .drop_column(AdminUsers::TotpEnabled)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(AdminUsers::Table)
                    .drop_column(AdminUsers::TotpEnabledAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum AdminUsers {
    Table,
    TotpSecret,
    TotpEnabled,
    TotpEnabledAt,
}
