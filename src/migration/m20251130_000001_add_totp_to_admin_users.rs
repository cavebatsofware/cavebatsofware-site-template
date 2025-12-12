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
