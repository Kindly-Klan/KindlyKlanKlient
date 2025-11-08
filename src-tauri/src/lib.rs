// Kindly Klan Klient
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
 
use std::process::Command;
 
 
use tauri::{Emitter, Manager};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

mod logging;
mod sessions;
mod models;
mod versions;
mod launcher;
mod utils;
mod whitelist;
mod sessions_api;
mod instances;
mod auth_ms;
mod commands;
mod admins;
mod local_instances;
pub use models::*;
pub use versions::*;
pub use whitelist::*;
pub use utils::*;
pub use sessions_api::*;
pub use instances::*;
pub use auth_ms::*;
pub use commands::*;
pub use admins::*;
pub use local_instances::*;
 

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub access_token: String,
    pub username: String,
    pub uuid: String,
    pub user_type: String,
    pub expires_at: Option<i64>,
    pub refresh_token: Option<String>,
}
 

const AZURE_CLIENT_ID: &str = "d1538b43-1083-43ac-89d5-c88cb0049ada";

#[allow(dead_code)]
async fn validate_access_token(access_token: &str) -> Result<bool, String> {
    match crate::auth_ms::get_minecraft_profile_from_token(access_token).await {
        Ok(_) => Ok(true),
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("401") || err_str.contains("Unauthorized") {
                Ok(false)
            } else {
                Err(err_str)
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum EnsureSessionResponse {
    Ok { session: sessions::Session, refreshed: bool },
    Err { code: String, message: String },
}

#[tauri::command]
async fn launch_minecraft_with_java(
    app_handle: tauri::AppHandle,
    instance_id: String,
    java_path: String,
    minecraft_version: String,
    _java_version: String,
    access_token: String,
    min_ram_gb: Option<f64>,
    max_ram_gb: Option<f64>
) -> Result<String, String> {
    let instance_dir = crate::launcher::get_instance_directory(&instance_id);
    if !instance_dir.exists() {
        return Err(format!("Instance directory does not exist: {}", instance_dir.display()));
    }

    launch_minecraft_with_auth(&app_handle, &instance_id, &minecraft_version, &java_path, &access_token, min_ram_gb, max_ram_gb).await
}

async fn launch_minecraft_with_auth(
    app_handle: &tauri::AppHandle,
    instance_id: &str,
    minecraft_version: &str,
    java_path: &str,
    access_token: &str,
    min_ram_gb: Option<f64>,
    max_ram_gb: Option<f64>
) -> Result<String, String> {
    let instance_dir = crate::launcher::get_instance_directory(instance_id);

    ensure_minecraft_client_present(&instance_dir, minecraft_version).await?;
    crate::instances::ensure_version_libraries(&instance_dir, minecraft_version).await?;

    let _ = std::fs::create_dir_all(instance_dir.join("libraries"));
    let _ = std::fs::create_dir_all(instance_dir.join("mods"));
    let classpath = crate::launcher::build_minecraft_classpath(&instance_dir)?;
    {
        let mut has_lwjgl = false;
        for entry in walkdir::WalkDir::new(instance_dir.join("libraries")) {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_type().is_file() {
                let p = entry.path();
                if p.to_string_lossy().contains("lwjgl") {
                    has_lwjgl = true;
                    break;
                }
            }
        }
        if !has_lwjgl { ensure_minecraft_client_present(&instance_dir, minecraft_version).await?; }
    }

    let min_ram = min_ram_gb.unwrap_or(2.0);
    let max_ram = max_ram_gb.unwrap_or(4.0);
    
    let (jvm_args_config, gc_config, window_width, window_height) = load_advanced_config().await.unwrap_or((
        String::new(), "G1".to_string(), 1280, 720
    ));
    
    let jvm_args = crate::launcher::build_minecraft_jvm_args(access_token, min_ram, max_ram, &gc_config, &jvm_args_config)?;
    
    let asset_index_id = ensure_assets_present(app_handle, &instance_dir, minecraft_version).await?;

    let profile = crate::auth_ms::get_minecraft_profile_from_token(access_token).await
        .map_err(|e| e.to_string())?;
    let username = profile["name"].as_str().unwrap_or("Player");
    let uuid = profile["id"].as_str().unwrap_or("00000000000000000000000000000000");

    let assets_dir = instance_dir.join("assets");
    let mut mc_args = vec![
        "--username".to_string(), username.to_string(),
        "--uuid".to_string(), uuid.to_string(),
        "--accessToken".to_string(), access_token.to_string(),
        "--version".to_string(), minecraft_version.to_string(),
        "--gameDir".to_string(), instance_dir.to_string_lossy().to_string(),
        "--assetsDir".to_string(), assets_dir.to_string_lossy().to_string(),
        "--assetIndex".to_string(), asset_index_id,
        "--userType".to_string(), "msa".to_string(),
        "--versionType".to_string(), "release".to_string(),
    ];

    mc_args.push("--width".to_string());
    mc_args.push(window_width.to_string());
    mc_args.push("--height".to_string());
    mc_args.push(window_height.to_string());

    let main_class = crate::launcher::select_main_class(&instance_dir);
    let mut command = Command::new(java_path);
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command
        .args(&jvm_args)
        .arg("-cp")
        .arg(&classpath)
        .arg(main_class)
        .args(&mc_args)
        .current_dir(&instance_dir)
        .spawn()
        .map_err(|e| format!("Failed to start Minecraft: {}", e))?;

    let app = app_handle.clone();
    std::thread::spawn(move || {
        let _ = child.wait();
        let _ = app.emit("minecraft_exited", serde_json::json!({ "status": "exited" }));
    });

    Ok("Minecraft launched".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenv::dotenv().ok();
    
    // Initialize logging system
    if let Err(e) = logging::init_logging() {
        eprintln!("Error initializing logging: {}", e);
    }
    
    log::info!("Starting KindlyKlanKlient...");
    
    // Global state for tracking active downloads
    use std::sync::{Arc, Mutex};
    let is_downloading = Arc::new(Mutex::new(false));
    
    tauri::Builder::default()
        .plugin(tauri_plugin_oauth::init())
        .plugin(tauri_plugin_updater::Builder::default().build())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .manage(is_downloading)
        .on_window_event(move |window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app_handle = window.app_handle();
                if let Some(state) = app_handle.try_state::<Arc<Mutex<bool>>>() {
                    if let Ok(downloading) = state.lock() {
                        if *downloading {
                            // Emit event to frontend to show dialog
                            let _ = window.emit("close-requested-during-download", ());
                            api.prevent_close();
                        }
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_versions,
            launch_game,
            start_microsoft_auth,
            load_distribution_manifest,
            get_instance_background_video,
            get_instance_details,
            download_instance,
            create_instance_directory,
            launch_minecraft_with_java,
            get_required_java_version_command,
            check_java_version,
            download_java,
            set_downloading_state,
            get_java_path,
            stop_minecraft_instance,
            restart_application,
            get_system_ram,
            save_ram_config,
            load_ram_config,
            save_advanced_config,
            load_advanced_config,
            check_for_updates,
            install_update,
            get_update_state,
            save_update_state_command,
            download_update_silent,
            check_whitelist_access,
            get_accessible_instances,
            clear_whitelist_cache,
            open_url,
            debug_env_vars,
            save_session,
            get_session,
            get_active_session,
            update_session,
            delete_session,
            clear_all_sessions,
            cleanup_expired_sessions,
            debug_sessions,
            refresh_session,
            get_db_path,
            validate_and_refresh_token,
            ensure_valid_session,
            get_minecraft_profile_safe,
            clear_update_state,
            download_instance_assets,
            test_manifest_url,
            // Admin system
            check_is_admin,
            // Versions
            get_minecraft_versions,
            get_fabric_loader_versions,
            // Local instances
            create_local_instance,
            get_local_instances,
            sync_mods_from_remote,
            open_instance_folder,
            launch_local_instance
        ])
        .run(tauri::generate_context!())
        .expect("error while running kindly klan klient");
}