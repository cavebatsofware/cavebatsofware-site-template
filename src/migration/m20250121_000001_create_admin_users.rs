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
        manager
            .create_table(
                Table::create()
                    .table(AdminUsers::Table)
                    .if_not_exists()
                    .col(uuid(AdminUsers::Id).primary_key())
                    .col(string_uniq(AdminUsers::Email))
                    .col(string(AdminUsers::PasswordHash))
                    .col(boolean(AdminUsers::EmailVerified).default(false))
                    .col(string_null(AdminUsers::VerificationToken))
                    .col(timestamp_with_time_zone_null(
                        AdminUsers::VerificationTokenExpiresAt,
                    ))
                    .col(
                        timestamp_with_time_zone(AdminUsers::CreatedAt)
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        timestamp_with_time_zone(AdminUsers::UpdatedAt)
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on verification_token for faster lookups
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_admin_users_verification_token")
                    .table(AdminUsers::Table)
                    .col(AdminUsers::VerificationToken)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AdminUsers::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AdminUsers {
    Table,
    Id,
    Email,
    PasswordHash,
    EmailVerified,
    VerificationToken,
    VerificationTokenExpiresAt,
    CreatedAt,
    UpdatedAt,
}
