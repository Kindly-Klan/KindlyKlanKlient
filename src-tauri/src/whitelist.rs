use crate::models::{AccessCheck, WhitelistEntry};
use anyhow::Result;
use reqwest;
use serde_json;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::LazyLock;

// Cache for whitelist entries (5 minutes TTL)
static WHITELIST_CACHE: LazyLock<Mutex<HashMap<String, (AccessCheck, u64)>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn is_cache_valid(timestamp: u64) -> bool {
    get_current_timestamp() - timestamp < 300
}

pub fn get_supabase_config() -> (String, String) {
    let url = std::env::var("SUPABASE_URL")
        .unwrap_or_else(|_| env!("SUPABASE_URL").to_string());
    let key = std::env::var("SUPABASE_ANON_KEY")
        .unwrap_or_else(|_| env!("SUPABASE_ANON_KEY").to_string());
    if url == "https://your-project.supabase.co" || key == "your-anon-key" {
        log::warn!(" Supabase not configured - using fallback values");
    } else {
        log::info!("✅ Supabase configured successfully");
    }
    (url, key)
}

#[tauri::command]
pub async fn check_whitelist_access(username: String) -> Result<AccessCheck, String> {
    log::info!("🔍 Checking whitelist access for user: {}", username);
    let (supabase_url, supabase_key) = get_supabase_config();

    if supabase_url == "https://your-project.supabase.co" || supabase_key == "your-anon-key" {
        log::warn!("⚠️  Whitelist disabled - allowing access for user: {}", username);
        return Ok(AccessCheck { has_access: true, allowed_instances: Vec::new(), global_access: true });
    }

    {
        let cache = WHITELIST_CACHE.lock().unwrap();
        if let Some((cached_result, timestamp)) = cache.get(&username) {
            if is_cache_valid(*timestamp) {
                log::info!("✅ Using cached whitelist result for user: {}", username);
                return Ok(cached_result.clone());
            }
        }
    }

    log::info!("🌐 Querying Supabase for user: {}", username);
    let client = reqwest::Client::new();
    let url = format!("{}/rest/v1/whitelist?minecraft_username=eq.{}", supabase_url, username);

    let response = client
        .get(&url)
        .header("apikey", &supabase_key)
        .header("Authorization", &format!("Bearer {}", supabase_key))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| {
            log::error!("❌ Failed to send request to Supabase: {}", e);
            format!("Failed to query whitelist: {}", e)
        })?;

    let status = response.status();
    log::info!("📡 Response status: {}", status);
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        log::error!("❌ API error response: {}", error_text);
        return Err(format!("Whitelist API error: {} - {}", status, error_text));
    }

    let response_text = response.text().await.map_err(|e| {
        log::error!("❌ Failed to read response: {}", e);
        format!("Failed to read whitelist response: {}", e)
    })?;
    log::info!("📄 Response body: {}", response_text);

    let entries: Vec<WhitelistEntry> = serde_json::from_str(&response_text).map_err(|e| {
        log::error!("❌ Failed to parse JSON: {}", e);
        log::error!("❌ Raw response: {}", response_text);
        format!("Failed to parse whitelist response: {}", e)
    })?;

    log::info!("📊 Found {} entries for user: {}", entries.len(), username);
    let result = if entries.is_empty() {
        log::warn!("❌ User not found in whitelist: {}", username);
        AccessCheck { has_access: false, allowed_instances: Vec::new(), global_access: false }
    } else {
        let entry = &entries[0];
        log::info!("✅ User found in whitelist: {}", username);
        log::info!("   Global access: {}", entry.global_access);
        log::info!("   Allowed instances: {:?}", entry.allowed_instances);
        AccessCheck { has_access: true, allowed_instances: entry.allowed_instances.clone().unwrap_or_default(), global_access: entry.global_access }
    };

    {
        let mut cache = WHITELIST_CACHE.lock().unwrap();
        cache.insert(username, (result.clone(), get_current_timestamp()));
    }

    Ok(result)
}

#[tauri::command]
pub async fn get_accessible_instances(username: String, all_instances: Vec<String>) -> Result<Vec<String>, String> {
    let access_check = check_whitelist_access(username).await?;
    if !access_check.has_access { return Ok(Vec::new()); }
    if access_check.global_access { Ok(all_instances) } else {
        let accessible: Vec<String> = all_instances.into_iter().filter(|instance| access_check.allowed_instances.contains(instance)).collect();
        Ok(accessible)
    }
}

#[tauri::command]
pub async fn clear_whitelist_cache() -> Result<String, String> {
    let mut cache = WHITELIST_CACHE.lock().unwrap();
    cache.clear();
    Ok("Whitelist cache cleared".to_string())
}


