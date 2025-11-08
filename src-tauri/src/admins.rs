use crate::models::AdminEntry;
use anyhow::Result;
use reqwest;
use serde_json;

fn get_supabase_config() -> (String, String) {
    let url = std::env::var("SUPABASE_URL")
        .unwrap_or_else(|_| env!("SUPABASE_URL").to_string());
    let key = std::env::var("SUPABASE_ANON_KEY")
        .unwrap_or_else(|_| env!("SUPABASE_ANON_KEY").to_string());
    (url, key)
}

#[tauri::command]
pub async fn check_is_admin(username: String) -> Result<bool, String> {
    log::info!("🔍 Checking admin status for user: {}", username);
    let (supabase_url, supabase_key) = get_supabase_config();

    if supabase_url == "https://your-project.supabase.co" || supabase_key == "your-anon-key" {
        log::warn!("⚠️  Supabase not configured - denying admin access for user: {}", username);
        return Ok(false);
    }

    log::info!("🌐 Querying Supabase admins table for user: {}", username);
    let client = reqwest::Client::new();
    let url = format!("{}/rest/v1/admins?minecraft_username=eq.{}", supabase_url, username);

    let response = client
        .get(&url)
        .header("apikey", &supabase_key)
        .header("Authorization", &format!("Bearer {}", supabase_key))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| {
            log::error!("❌ Failed to send request to Supabase: {}", e);
            format!("Failed to query admins table: {}", e)
        })?;

    let status = response.status();
    log::info!("📡 Response status: {}", status);
    
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        log::error!("❌ API error response: {}", error_text);
        return Err(format!("Admins API error: {} - {}", status, error_text));
    }

    let response_text = response.text().await.map_err(|e| {
        log::error!("❌ Failed to read response: {}", e);
        format!("Failed to read admins response: {}", e)
    })?;
    
    log::info!("📄 Response body: {}", response_text);

    let entries: Vec<AdminEntry> = serde_json::from_str(&response_text).map_err(|e| {
        log::error!("❌ Failed to parse JSON: {}", e);
        log::error!("❌ Raw response: {}", response_text);
        format!("Failed to parse admins response: {}", e)
    })?;

    let is_admin = !entries.is_empty();
    
    if is_admin {
        log::info!("✅ User {} is an admin", username);
    } else {
        log::info!("❌ User {} is NOT an admin", username);
    }

    Ok(is_admin)
}

