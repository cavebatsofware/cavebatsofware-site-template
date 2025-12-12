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

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add admin_user_id column
        manager
            .alter_table(
                Table::alter()
                    .table(AccessLog::Table)
                    .add_column(ColumnDef::new(AccessLog::AdminUserId).uuid().null())
                    .to_owned(),
            )
            .await?;

        // Add admin_user_email column
        manager
            .alter_table(
                Table::alter()
                    .table(AccessLog::Table)
                    .add_column(ColumnDef::new(AccessLog::AdminUserEmail).string().null())
                    .to_owned(),
            )
            .await?;

        // Create foreign key constraint with ON DELETE SET NULL
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_access_log_admin_user")
                    .from(AccessLog::Table, AccessLog::AdminUserId)
                    .to(AdminUsers::Table, AdminUsers::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .to_owned(),
            )
            .await?;

        // Create index on admin_user_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_access_log_admin_user_id")
                    .table(AccessLog::Table)
                    .col(AccessLog::AdminUserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop index
        manager
            .drop_index(
                Index::drop()
                    .name("idx_access_log_admin_user_id")
                    .table(AccessLog::Table)
                    .to_owned(),
            )
            .await?;

        // Drop foreign key
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_access_log_admin_user")
                    .table(AccessLog::Table)
                    .to_owned(),
            )
            .await?;

        // Drop admin_user_email column
        manager
            .alter_table(
                Table::alter()
                    .table(AccessLog::Table)
                    .drop_column(AccessLog::AdminUserEmail)
                    .to_owned(),
            )
            .await?;

        // Drop admin_user_id column
        manager
            .alter_table(
                Table::alter()
                    .table(AccessLog::Table)
                    .drop_column(AccessLog::AdminUserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum AccessLog {
    Table,
    AdminUserId,
    AdminUserEmail,
}

#[derive(DeriveIden)]
enum AdminUsers {
    Table,
    Id,
}
