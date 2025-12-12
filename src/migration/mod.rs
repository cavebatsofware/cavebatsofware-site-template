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

pub use sea_orm_migration::prelude::*;

mod m20250119_000001_create_access_log;
mod m20250120_000001_drop_mac_address;
mod m20250121_000001_create_admin_users;
mod m20250122_000001_create_access_codes;
mod m20250123_000001_add_usage_count;
mod m20250124_000001_create_settings;
mod m20250125_000001_create_articles;
mod m20250127_000001_create_subscribers;
mod m20250203_000001_add_description_to_access_codes;
mod m20250210_000001_add_download_filename_to_access_codes;
mod m20250211_000001_add_last_used_at_to_access_codes;
mod m20251116_000001_add_access_log_composite_indexes;
mod m20251116_000002_create_build_status;
mod m20251120_125244_rename_count_to_tokens;
mod m20251122_000001_drop_articles_and_build_status;
mod m20251130_000001_add_totp_to_admin_users;
mod m20251130_000002_add_mfa_lockout_fields;
mod m20251202_000001_add_user_management_fields;
mod m20251202_000002_seed_site_settings;
mod m20251202_000003_add_admin_user_to_access_log;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250119_000001_create_access_log::Migration),
            Box::new(m20250120_000001_drop_mac_address::Migration),
            Box::new(m20250121_000001_create_admin_users::Migration),
            Box::new(m20250122_000001_create_access_codes::Migration),
            Box::new(m20250123_000001_add_usage_count::Migration),
            Box::new(m20250124_000001_create_settings::Migration),
            Box::new(m20250125_000001_create_articles::Migration),
            Box::new(m20250127_000001_create_subscribers::Migration),
            Box::new(m20250203_000001_add_description_to_access_codes::Migration),
            Box::new(m20250210_000001_add_download_filename_to_access_codes::Migration),
            Box::new(m20250211_000001_add_last_used_at_to_access_codes::Migration),
            Box::new(m20251116_000001_add_access_log_composite_indexes::Migration),
            Box::new(m20251116_000002_create_build_status::Migration),
            Box::new(m20251120_125244_rename_count_to_tokens::Migration),
            Box::new(m20251122_000001_drop_articles_and_build_status::Migration),
            Box::new(m20251130_000001_add_totp_to_admin_users::Migration),
            Box::new(m20251130_000002_add_mfa_lockout_fields::Migration),
            Box::new(m20251202_000001_add_user_management_fields::Migration),
            Box::new(m20251202_000002_seed_site_settings::Migration),
            Box::new(m20251202_000003_add_admin_user_to_access_log::Migration),
        ]
    }
}
