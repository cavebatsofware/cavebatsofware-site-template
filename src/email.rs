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

use anyhow::Result;
use aws_sdk_sesv2::{
    types::{Body, Content, Destination, EmailContent, Message},
    Client as SesClient,
};
use std::env;

use crate::settings::SettingsService;

#[derive(Clone)]
pub struct EmailService {
    client: SesClient,
    settings: SettingsService,
    site_url: String,
}

impl EmailService {
    pub async fn new(settings: SettingsService) -> Result<Self> {
        let mut config_loader = aws_config::defaults(aws_config::BehaviorVersion::latest());

        // Override region if AWS_REGION is set in environment
        if let Ok(region) = env::var("AWS_REGION") {
            config_loader = config_loader.region(aws_sdk_sesv2::config::Region::new(region));
        }

        let config = config_loader.load().await;
        let client = SesClient::new(&config);

        let site_url = env::var("SITE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

        Ok(Self {
            client,
            settings,
            site_url,
        })
    }

    pub async fn send_verification_email(
        &self,
        to_email: &str,
        verification_token: &str,
    ) -> Result<()> {
        let site_name = self.settings.get_site_name().await?;
        let from_email = self.settings.get_from_email().await?;

        let verification_url = format!(
            "{}/admin/verify-email?token={}",
            self.site_url, verification_token
        );

        let subject = "Verify Your Admin Account";
        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Verify Your Email</title>
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background-color: #f4f4f4; border-radius: 5px; padding: 20px; margin-bottom: 20px;">
        <h1 style="color: #2c3e50; margin-top: 0;">Welcome to {} Admin</h1>
        <p>Thank you for registering as an admin user. Please verify your email address to complete your registration.</p>
    </div>

    <div style="background-color: white; border: 1px solid #ddd; border-radius: 5px; padding: 20px; margin-bottom: 20px;">
        <p>Click the button below to verify your email address:</p>
        <div style="text-align: center; margin: 30px 0;">
            <a href="{}"
               style="background-color: #3498db; color: white; padding: 12px 30px; text-decoration: none; border-radius: 5px; display: inline-block; font-weight: bold;">
                Verify Email Address
            </a>
        </div>
        <p style="color: #666; font-size: 14px;">Or copy and paste this link into your browser:</p>
        <p style="word-break: break-all; color: #3498db; font-size: 14px;">{}</p>
    </div>

    <div style="color: #666; font-size: 12px; text-align: center;">
        <p>This verification link will expire in 24 hours.</p>
        <p>If you didn't request this verification email, you can safely ignore it.</p>
    </div>
</body>
</html>
"#,
            site_name, verification_url, verification_url
        );

        let text_body = format!(
            r#"
Welcome to {} Admin

Thank you for registering as an admin user. Please verify your email address to complete your registration.

Verification Link: {}

This verification link will expire in 24 hours.

If you didn't request this verification email, you can safely ignore it.
"#,
            site_name, verification_url
        );

        let destination = Destination::builder().to_addresses(to_email).build();

        let subject_content = Content::builder().data(subject).charset("UTF-8").build()?;

        let html_content = Content::builder()
            .data(html_body)
            .charset("UTF-8")
            .build()?;

        let text_content = Content::builder()
            .data(text_body)
            .charset("UTF-8")
            .build()?;

        let body = Body::builder()
            .html(html_content)
            .text(text_content)
            .build();

        let message = Message::builder()
            .subject(subject_content)
            .body(body)
            .build();

        let email_content = EmailContent::builder().simple(message).build();

        self.client
            .send_email()
            .from_email_address(&from_email)
            .destination(destination)
            .content(email_content)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send verification email: {}", e))?;

        tracing::info!("Verification email sent to {}", to_email);

        Ok(())
    }

    pub async fn send_contact_form_email(
        &self,
        from_name: &str,
        from_email: &str,
        subject: &str,
        message: &str,
    ) -> Result<()> {
        let site_name = self.settings.get_site_name().await?;
        let to_email = self.settings.get_contact_email().await?;
        let sender_email = self.settings.get_from_email().await?;

        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Contact Form Submission</title>
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background-color: #f4f4f4; border-radius: 5px; padding: 20px; margin-bottom: 20px;">
        <h1 style="color: #2c3e50; margin-top: 0;">New Contact Form Submission</h1>
    </div>

    <div style="background-color: white; border: 1px solid #ddd; border-radius: 5px; padding: 20px; margin-bottom: 20px;">
        <h2 style="color: #2c3e50; margin-top: 0;">Contact Information</h2>
        <p><strong>Name:</strong> {}</p>
        <p><strong>Email:</strong> {}</p>
        <p><strong>Subject:</strong> {}</p>

        <h2 style="color: #2c3e50; margin-top: 30px;">Message</h2>
        <div style="background-color: #f9f9f9; padding: 15px; border-left: 4px solid #3498db; border-radius: 3px;">
            <p style="margin: 0; white-space: pre-wrap;">{}</p>
        </div>
    </div>

    <div style="color: #666; font-size: 12px; text-align: center;">
        <p>This message was sent via the contact form on {}</p>
    </div>
</body>
</html>
"#,
            html_escape(from_name),
            html_escape(from_email),
            html_escape(subject),
            html_escape(message),
            site_name
        );

        let text_body = format!(
            r#"
New Contact Form Submission

Name: {}
Email: {}
Subject: {}

Message:
{}

---
This message was sent via the contact form on {}
"#,
            from_name, from_email, subject, message, site_name
        );

        let destination = Destination::builder().to_addresses(to_email).build();

        let email_subject = format!("Contact Form: {}", subject);
        let subject_content = Content::builder()
            .data(email_subject)
            .charset("UTF-8")
            .build()?;

        let html_content = Content::builder()
            .data(html_body)
            .charset("UTF-8")
            .build()?;

        let text_content = Content::builder()
            .data(text_body)
            .charset("UTF-8")
            .build()?;

        let body = Body::builder()
            .html(html_content)
            .text(text_content)
            .build();

        let message = Message::builder()
            .subject(subject_content)
            .body(body)
            .build();

        let email_content = EmailContent::builder().simple(message).build();

        self.client
            .send_email()
            .from_email_address(&sender_email)
            .destination(destination)
            .content(email_content)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send contact form email: {}", e))?;

        tracing::info!("Contact form email sent from {}", from_email);

        Ok(())
    }

    pub async fn send_subscription_confirmation(
        &self,
        to_email: &str,
        verification_token: &str,
    ) -> Result<()> {
        let site_name = self.settings.get_site_name().await?;
        let from_email = self.settings.get_from_email().await?;

        let verification_url = format!(
            "{}/api/subscribe/verify?token={}",
            self.site_url, verification_token
        );

        let subject = "Confirm Your Blog Subscription";
        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Confirm Your Subscription</title>
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background-color: #f4f4f4; border-radius: 5px; padding: 20px; margin-bottom: 20px;">
        <h1 style="color: #2c3e50; margin-top: 0;">Welcome to {} Blog!</h1>
        <p>Thank you for subscribing. Please confirm your email address to start receiving updates.</p>
    </div>

    <div style="background-color: white; border: 1px solid #ddd; border-radius: 5px; padding: 20px; margin-bottom: 20px;">
        <p>Click the button below to confirm your subscription:</p>
        <div style="text-align: center; margin: 30px 0;">
            <a href="{}"
               style="background-color: #3498db; color: white; padding: 12px 30px; text-decoration: none; border-radius: 5px; display: inline-block; font-weight: bold;">
                Confirm Subscription
            </a>
        </div>
        <p style="color: #666; font-size: 14px;">Or copy and paste this link into your browser:</p>
        <p style="word-break: break-all; color: #3498db; font-size: 14px;">{}</p>
    </div>

    <div style="color: #666; font-size: 12px; text-align: center;">
        <p>This confirmation link will expire in 7 days.</p>
        <p>If you didn't subscribe to this blog, you can safely ignore this email.</p>
    </div>
</body>
</html>
"#,
            site_name, verification_url, verification_url
        );

        let text_body = format!(
            r#"
Welcome to {} Blog!

Thank you for subscribing. Please confirm your email address to start receiving updates.

Confirmation Link: {}

This confirmation link will expire in 7 days.

If you didn't subscribe to this blog, you can safely ignore this email.
"#,
            site_name, verification_url
        );

        let destination = Destination::builder().to_addresses(to_email).build();

        let subject_content = Content::builder().data(subject).charset("UTF-8").build()?;

        let html_content = Content::builder()
            .data(html_body)
            .charset("UTF-8")
            .build()?;

        let text_content = Content::builder()
            .data(text_body)
            .charset("UTF-8")
            .build()?;

        let body = Body::builder()
            .html(html_content)
            .text(text_content)
            .build();

        let message = Message::builder()
            .subject(subject_content)
            .body(body)
            .build();

        let email_content = EmailContent::builder().simple(message).build();

        self.client
            .send_email()
            .from_email_address(&from_email)
            .destination(destination)
            .content(email_content)
            .send()
            .await
            .map_err(|e| {
                anyhow::anyhow!("Failed to send subscription confirmation email: {}", e)
            })?;

        tracing::info!("Subscription confirmation email sent to {}", to_email);

        Ok(())
    }

    pub async fn send_password_changed_notification(
        &self,
        to_email: &str,
        changed_by_admin: bool,
    ) -> Result<()> {
        let from_email = self.settings.get_from_email().await?;

        let subject = "Your Password Has Been Changed";
        let change_source = if changed_by_admin {
            "by an administrator"
        } else {
            "using your account"
        };

        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Password Changed</title>
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background-color: #f4f4f4; border-radius: 5px; padding: 20px; margin-bottom: 20px;">
        <h1 style="color: #2c3e50; margin-top: 0;">Password Changed</h1>
        <p>Your admin account password was recently changed {}.</p>
    </div>

    <div style="background-color: white; border: 1px solid #ddd; border-radius: 5px; padding: 20px; margin-bottom: 20px;">
        <p>If you made this change, you can safely ignore this email.</p>
        <p style="color: #c0392b;"><strong>If you did not make this change</strong>, please contact an administrator immediately as your account may have been compromised.</p>
    </div>

    <div style="color: #666; font-size: 12px; text-align: center;">
        <p>This is an automated security notification.</p>
    </div>
</body>
</html>
"#,
            change_source
        );

        let text_body = format!(
            r#"
Password Changed

Your admin account password was recently changed {}.

If you made this change, you can safely ignore this email.

If you did NOT make this change, please contact an administrator immediately as your account may have been compromised.

---
This is an automated security notification.
"#,
            change_source
        );

        let destination = Destination::builder().to_addresses(to_email).build();

        let subject_content = Content::builder().data(subject).charset("UTF-8").build()?;

        let html_content = Content::builder()
            .data(html_body)
            .charset("UTF-8")
            .build()?;

        let text_content = Content::builder()
            .data(text_body)
            .charset("UTF-8")
            .build()?;

        let body = Body::builder()
            .html(html_content)
            .text(text_content)
            .build();

        let message = Message::builder()
            .subject(subject_content)
            .body(body)
            .build();

        let email_content = EmailContent::builder().simple(message).build();

        self.client
            .send_email()
            .from_email_address(&from_email)
            .destination(destination)
            .content(email_content)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send password change notification: {}", e))?;

        tracing::info!("Password change notification sent to {}", to_email);

        Ok(())
    }

    pub async fn send_password_reset_email(
        &self,
        to_email: &str,
        reset_token: &str,
    ) -> Result<()> {
        let from_email = self.settings.get_from_email().await?;
        let reset_url = format!(
            "{}/admin/reset-password?token={}",
            self.site_url, reset_token
        );

        let subject = "Password Reset Request";
        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Reset Your Password</title>
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background-color: #f4f4f4; border-radius: 5px; padding: 20px; margin-bottom: 20px;">
        <h1 style="color: #2c3e50; margin-top: 0;">Password Reset Request</h1>
        <p>We received a request to reset your admin account password.</p>
    </div>

    <div style="background-color: white; border: 1px solid #ddd; border-radius: 5px; padding: 20px; margin-bottom: 20px;">
        <p>Click the button below to reset your password:</p>
        <div style="text-align: center; margin: 30px 0;">
            <a href="{}"
               style="background-color: #3498db; color: white; padding: 12px 30px; text-decoration: none; border-radius: 5px; display: inline-block; font-weight: bold;">
                Reset Password
            </a>
        </div>
        <p style="color: #666; font-size: 14px;">Or copy and paste this link into your browser:</p>
        <p style="word-break: break-all; color: #3498db; font-size: 14px;">{}</p>
    </div>

    <div style="color: #666; font-size: 12px; text-align: center;">
        <p><strong>This link will expire in 1 hour.</strong></p>
        <p>If you didn't request a password reset, you can safely ignore this email. Your password will remain unchanged.</p>
    </div>
</body>
</html>
"#,
            reset_url, reset_url
        );

        let text_body = format!(
            r#"
Password Reset Request

We received a request to reset your admin account password.

Reset Link: {}

This link will expire in 1 hour.

If you didn't request a password reset, you can safely ignore this email. Your password will remain unchanged.
"#,
            reset_url
        );

        let destination = Destination::builder().to_addresses(to_email).build();

        let subject_content = Content::builder().data(subject).charset("UTF-8").build()?;

        let html_content = Content::builder()
            .data(html_body)
            .charset("UTF-8")
            .build()?;

        let text_content = Content::builder()
            .data(text_body)
            .charset("UTF-8")
            .build()?;

        let body = Body::builder()
            .html(html_content)
            .text(text_content)
            .build();

        let message = Message::builder()
            .subject(subject_content)
            .body(body)
            .build();

        let email_content = EmailContent::builder().simple(message).build();

        self.client
            .send_email()
            .from_email_address(&from_email)
            .destination(destination)
            .content(email_content)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send password reset email: {}", e))?;

        tracing::info!("Password reset email sent to {}", to_email);

        Ok(())
    }
}

// Helper function to escape HTML entities
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
