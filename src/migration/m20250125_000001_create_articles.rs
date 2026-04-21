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
                    .table(Articles::Table)
                    .if_not_exists()
                    .col(uuid(Articles::Id).primary_key())
                    .col(string_uniq(Articles::Slug))
                    .col(string(Articles::Title))
                    .col(text(Articles::Content))
                    .col(text_null(Articles::Excerpt))
                    .col(uuid(Articles::AuthorId))
                    .col(boolean(Articles::Published).default(false))
                    .col(timestamp_with_time_zone_null(Articles::PublishedAt))
                    .col(string_null(Articles::Category))
                    .col(json_null(Articles::Tags))
                    .col(
                        timestamp_with_time_zone(Articles::CreatedAt)
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        timestamp_with_time_zone(Articles::UpdatedAt)
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_articles_author")
                            .from(Articles::Table, Articles::AuthorId)
                            .to(AdminUsers::Table, AdminUsers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on slug for faster lookups
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_articles_slug")
                    .table(Articles::Table)
                    .col(Articles::Slug)
                    .to_owned(),
            )
            .await?;

        // Create index on published for filtering
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_articles_published")
                    .table(Articles::Table)
                    .col(Articles::Published)
                    .to_owned(),
            )
            .await?;

        // Create index on category for filtering
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_articles_category")
                    .table(Articles::Table)
                    .col(Articles::Category)
                    .to_owned(),
            )
            .await?;

        // Create index on published_at for sorting
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_articles_published_at")
                    .table(Articles::Table)
                    .col(Articles::PublishedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Articles::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Articles {
    Table,
    Id,
    Slug,
    Title,
    Content,
    Excerpt,
    AuthorId,
    Published,
    PublishedAt,
    Category,
    Tags,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum AdminUsers {
    Table,
    Id,
}
