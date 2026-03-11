use anyhow::Result;
use crate::service::email_service::EmailClient;
use crate::models::notebook::CollaboratorRole;

/// Email service for notebook collaboration notifications
pub struct CollaborationEmailService {
    client: EmailClient,
    app_url: String,
}

impl CollaborationEmailService {
    pub fn new() -> Result<Self> {
        let client = EmailClient::new()?;
        let app_url = std::env::var("APP_URL")
            .unwrap_or_else(|_| "https://tradstry.com".to_string());
        
        Ok(Self { client, app_url })
    }

    pub fn with_client(client: EmailClient, app_url: &str) -> Self {
        Self {
            client,
            app_url: app_url.to_string(),
        }
    }

    /// Send collaboration invitation email
    pub async fn send_invitation(
        &self,
        invitee_email: &str,
        inviter_name: &str,
        inviter_email: &str,
        note_title: &str,
        invitation_token: &str,
        role: &CollaboratorRole,
        personal_message: Option<&str>,
    ) -> Result<String> {
        let invitation_link = format!(
            "{}/app/notebook/invite/{}",
            self.app_url, invitation_token
        );

        let role_text = match role {
            CollaboratorRole::Owner => "full ownership",
            CollaboratorRole::Editor => "edit access",
            CollaboratorRole::Viewer => "view access",
        };

        let subject = format!("{} invited you to collaborate on \"{}\"", inviter_name, note_title);
        
        let html = self.build_invitation_html(
            inviter_name,
            inviter_email,
            note_title,
            &invitation_link,
            role_text,
            personal_message,
        );

        let text = self.build_invitation_text(
            inviter_name,
            note_title,
            &invitation_link,
            role_text,
            personal_message,
        );

        let email_id = self.client.send(
            &[invitee_email],
            &subject,
            &html,
            Some(&text),
        ).await?;

        Ok(email_id)
    }

    fn build_invitation_html(
        &self,
        inviter_name: &str,
        inviter_email: &str,
        note_title: &str,
        invitation_link: &str,
        role_text: &str,
        personal_message: Option<&str>,
    ) -> String {
        let message_section = personal_message.map(|msg| format!(r#"
            <div style="background: #f8fafc; border-left: 4px solid #6366f1; padding: 16px 20px; margin: 24px 0; border-radius: 0 8px 8px 0;">
                <p style="margin: 0; color: #64748b; font-size: 12px; text-transform: uppercase; letter-spacing: 0.5px; font-weight: 600;">Personal Message</p>
                <p style="margin: 8px 0 0 0; color: #334155; font-size: 15px; line-height: 1.6; font-style: italic;">"{}"</p>
            </div>
        "#, html_escape(msg))).unwrap_or_default();

        format!(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Collaboration Invitation</title>
</head>
<body style="margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; background-color: #f1f5f9;">
    <table role="presentation" style="width: 100%; border-collapse: collapse;">
        <tr>
            <td style="padding: 40px 20px;">
                <table role="presentation" style="max-width: 600px; margin: 0 auto; background: #ffffff; border-radius: 16px; overflow: hidden; box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);">
                    <!-- Header -->
                    <tr>
                        <td style="background: linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%); padding: 32px 40px; text-align: center;">
                            <h1 style="margin: 0; color: #ffffff; font-size: 24px; font-weight: 700; letter-spacing: -0.5px;">
                                📝 Tradstry
                            </h1>
                            <p style="margin: 8px 0 0 0; color: rgba(255, 255, 255, 0.9); font-size: 14px;">
                                Trading Journal & Analytics
                            </p>
                        </td>
                    </tr>
                    
                    <!-- Content -->
                    <tr>
                        <td style="padding: 40px;">
                            <!-- Invitation Badge -->
                            <div style="text-align: center; margin-bottom: 32px;">
                                <span style="display: inline-block; background: #eef2ff; color: #4f46e5; padding: 8px 16px; border-radius: 20px; font-size: 13px; font-weight: 600;">
                                    ✨ You've been invited to collaborate
                                </span>
                            </div>

                            <!-- Main Message -->
                            <h2 style="margin: 0 0 16px 0; color: #1e293b; font-size: 22px; font-weight: 700; text-align: center; line-height: 1.3;">
                                {inviter_name} wants to share a note with you
                            </h2>
                            
                            <p style="margin: 0 0 24px 0; color: #64748b; font-size: 15px; text-align: center; line-height: 1.6;">
                                You've been granted <strong style="color: #4f46e5;">{role_text}</strong> to collaborate on this note.
                            </p>

                            <!-- Note Card -->
                            <div style="background: linear-gradient(135deg, #fafafa 0%, #f5f5f5 100%); border: 1px solid #e2e8f0; border-radius: 12px; padding: 24px; margin: 24px 0; text-align: center;">
                                <div style="width: 48px; height: 48px; background: #6366f1; border-radius: 12px; margin: 0 auto 16px auto; display: flex; align-items: center; justify-content: center;">
                                    <span style="font-size: 24px;">📄</span>
                                </div>
                                <h3 style="margin: 0 0 8px 0; color: #1e293b; font-size: 18px; font-weight: 600;">
                                    {note_title}
                                </h3>
                                <p style="margin: 0; color: #64748b; font-size: 13px;">
                                    Shared by {inviter_email}
                                </p>
                            </div>

                            {message_section}

                            <!-- CTA Button -->
                            <div style="text-align: center; margin: 32px 0;">
                                <a href="{invitation_link}" style="display: inline-block; background: linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%); color: #ffffff; text-decoration: none; padding: 16px 40px; border-radius: 10px; font-size: 16px; font-weight: 600; box-shadow: 0 4px 14px 0 rgba(99, 102, 241, 0.4); transition: all 0.2s ease;">
                                    Accept Invitation →
                                </a>
                            </div>

                            <p style="margin: 24px 0 0 0; color: #94a3b8; font-size: 13px; text-align: center;">
                                This invitation expires in 7 days
                            </p>
                        </td>
                    </tr>

                    <!-- Footer -->
                    <tr>
                        <td style="background: #f8fafc; padding: 24px 40px; border-top: 1px solid #e2e8f0;">
                            <p style="margin: 0 0 8px 0; color: #64748b; font-size: 13px; text-align: center;">
                                If you didn't expect this invitation, you can safely ignore this email.
                            </p>
                            <p style="margin: 0; color: #94a3b8; font-size: 12px; text-align: center;">
                                © 2024 Tradstry. All rights reserved.
                            </p>
                        </td>
                    </tr>
                </table>
            </td>
        </tr>
    </table>
</body>
</html>
        "#,
            inviter_name = html_escape(inviter_name),
            inviter_email = html_escape(inviter_email),
            note_title = html_escape(note_title),
            invitation_link = invitation_link,
            role_text = role_text,
            message_section = message_section,
        )
    }

    fn build_invitation_text(
        &self,
        inviter_name: &str,
        note_title: &str,
        invitation_link: &str,
        role_text: &str,
        personal_message: Option<&str>,
    ) -> String {
        let message_section = personal_message
            .map(|msg| format!("\nPersonal message from {}:\n\"{}\"\n", inviter_name, msg))
            .unwrap_or_default();

        format!(
            r#"You've been invited to collaborate on Tradstry!

{} wants to share a note with you.

Note: "{}"
Access Level: {}
{}
Click the link below to accept the invitation:
{}

This invitation expires in 7 days.

---
If you didn't expect this invitation, you can safely ignore this email.

© 2024 Tradstry
"#,
            inviter_name,
            note_title,
            role_text,
            message_section,
            invitation_link,
        )
    }

    /// Send notification when invitation is accepted
    pub async fn send_invitation_accepted(
        &self,
        inviter_email: &str,
        invitee_name: &str,
        invitee_email: &str,
        note_title: &str,
        note_link: &str,
    ) -> Result<String> {
        let subject = format!("{} accepted your invitation to \"{}\"", invitee_name, note_title);
        
        let html = self.build_accepted_html(invitee_name, invitee_email, note_title, note_link);
        let text = self.build_accepted_text(invitee_name, note_title, note_link);

        let email_id = self.client.send(
            &[inviter_email],
            &subject,
            &html,
            Some(&text),
        ).await?;

        Ok(email_id)
    }

    fn build_accepted_html(
        &self,
        invitee_name: &str,
        invitee_email: &str,
        note_title: &str,
        note_link: &str,
    ) -> String {
        format!(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background-color: #f1f5f9;">
    <table role="presentation" style="width: 100%; border-collapse: collapse;">
        <tr>
            <td style="padding: 40px 20px;">
                <table role="presentation" style="max-width: 600px; margin: 0 auto; background: #ffffff; border-radius: 16px; overflow: hidden; box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);">
                    <tr>
                        <td style="background: linear-gradient(135deg, #10b981 0%, #059669 100%); padding: 32px 40px; text-align: center;">
                            <h1 style="margin: 0; color: #ffffff; font-size: 24px; font-weight: 700;">
                                🎉 Invitation Accepted!
                            </h1>
                        </td>
                    </tr>
                    <tr>
                        <td style="padding: 40px;">
                            <div style="text-align: center;">
                                <div style="width: 64px; height: 64px; background: #d1fae5; border-radius: 50%; margin: 0 auto 24px auto; display: flex; align-items: center; justify-content: center;">
                                    <span style="font-size: 32px;">✅</span>
                                </div>
                                
                                <h2 style="margin: 0 0 16px 0; color: #1e293b; font-size: 20px; font-weight: 700;">
                                    {invitee_name} is now a collaborator
                                </h2>
                                
                                <p style="margin: 0 0 24px 0; color: #64748b; font-size: 15px;">
                                    <strong>{invitee_email}</strong> accepted your invitation to collaborate on <strong>"{note_title}"</strong>
                                </p>

                                <a href="{note_link}" style="display: inline-block; background: #10b981; color: #ffffff; text-decoration: none; padding: 14px 32px; border-radius: 10px; font-size: 15px; font-weight: 600;">
                                    Open Note →
                                </a>
                            </div>
                        </td>
                    </tr>
                    <tr>
                        <td style="background: #f8fafc; padding: 20px 40px; border-top: 1px solid #e2e8f0;">
                            <p style="margin: 0; color: #94a3b8; font-size: 12px; text-align: center;">
                                © 2025 Tradstry. All rights reserved.
                            </p>
                        </td>
                    </tr>
                </table>
            </td>
        </tr>
    </table>
</body>
</html>
        "#,
            invitee_name = html_escape(invitee_name),
            invitee_email = html_escape(invitee_email),
            note_title = html_escape(note_title),
            note_link = note_link,
        )
    }

    fn build_accepted_text(
        &self,
        invitee_name: &str,
        note_title: &str,
        note_link: &str,
    ) -> String {
        format!(
            r#"Great news! Your invitation was accepted.

{} is now a collaborator on "{}".

Open the note: {}

---
© 2025 Tradstry
"#,
            invitee_name, note_title, note_link
        )
    }
}

/// Simple HTML escape for user content
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
