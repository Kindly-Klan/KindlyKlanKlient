use crate::versions::MinecraftVersion;
use crate::launcher::MinecraftLauncher;
use crate::AuthSession;
use tokio::fs;
use std::fs::File;
use std::io::Write;
use reqwest;
use tauri::AppHandle;
use tauri::Emitter;
use crate::UpdateState;
use crate::{DistributionManifest, InstanceManifest};

#[tauri::command]
pub async fn greet(name: String) -> String {
    format!("Hello, {}! Welcome to Kindly Klan Klient!", name)
}

#[tauri::command]
pub async fn create_instance_directory(instance_id: String, java_version: String) -> Result<String, String> {
    let kindly_dir = std::env::var("USERPROFILE")
        .map(|p| std::path::Path::new(&p).join(".kindlyklanklient"))
        .unwrap_or_else(|_| std::path::Path::new(".").join(".kindlyklanklient"));

    let instance_dir = kindly_dir.join(&instance_id);
    let runtime_dir = kindly_dir.join("runtime");
    let java_dir = runtime_dir.join(format!("java-{}", java_version));

    fs::create_dir_all(&instance_dir).await
        .map_err(|e| format!("Failed to create instance directory: {}", e))?;
    fs::create_dir_all(&java_dir).await
        .map_err(|e| format!("Failed to create Java directory: {}", e))?;

    Ok(format!("Instance directory created: {}", instance_dir.display()))
}

#[tauri::command]
pub async fn get_required_java_version_command(minecraft_version: String) -> Result<String, String> {
    Ok(super::get_required_java_version(&minecraft_version))
}

#[tauri::command]
pub async fn get_versions() -> Result<Vec<MinecraftVersion>, String> {
    let launcher = MinecraftLauncher::new().map_err(|e| e.to_string())?;
    launcher.get_available_versions().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn launch_game(version: String, session: AuthSession) -> Result<String, String> {
    let launcher = MinecraftLauncher::new().map_err(|e| e.to_string())?;
    launcher.config.ensure_directories().await.map_err(|e| e.to_string())?;

    let ram_mb = crate::launcher::get_total_ram_mb().unwrap_or(4096);

    let version_dir = launcher.config.versions_dir.join(&version);
    let jar_path = version_dir.join(format!("{}.jar", version));

    let versions = launcher.get_available_versions().await.map_err(|e| e.to_string())?;

    if let Some(target_version) = versions.into_iter().find(|v| v.id == version) {
        let assets_dir = launcher.config.assets_dir.join("objects");
        let missing_assets = [
            "5f/5ff04807c356f1beed0b86ccf659b44b9983e3fa",
            "b3/b3305151c36cc6e776f0130e85e8baee7ea06ec9",
            "b8/b84572b0d91367c41ff73b22edd5a2e9c02eab13",
            "40/402ded0eebd448033ef415e861a17513075f80e7",
            "89/89e4e7c845d442d308a6194488de8bd3397f0791"
        ];

        let need_download = !jar_path.exists() || missing_assets.iter().any(|asset_path| !assets_dir.join(asset_path).exists());
        if need_download {
            launcher.download_version(&target_version).await.map_err(|e| e.to_string())?;
        }
    } else {
        return Err("Version not found".to_string());
    }

    launcher.launch_minecraft(&version, &session.username, ram_mb, Some(&session.access_token), Some(&session.uuid)).await.map_err(|e| e.to_string())?;
    Ok("Minecraft launched successfully!".to_string())
}

#[tauri::command]
pub async fn check_java_version(version: String) -> Result<String, String> {
    let kindly_dir = std::env::var("USERPROFILE")
        .map(|p| std::path::Path::new(&p).join(".kindlyklanklient"))
        .unwrap_or_else(|_| std::path::Path::new(".").join(".kindlyklanklient"));
    let java_dir = kindly_dir.join("runtime").join(format!("java-{}", version));
    let java_path = if cfg!(target_os = "windows") { java_dir.join("bin").join("java.exe") } else { java_dir.join("bin").join("java") };
    if java_path.exists() { Ok("installed".to_string()) } else { Ok("not_installed".to_string()) }
}

#[tauri::command]
pub async fn download_java(version: String) -> Result<String, String> {
    let kindly_dir = std::env::var("USERPROFILE")
        .map(|p| std::path::Path::new(&p).join(".kindlyklanklient"))
        .unwrap_or_else(|_| std::path::Path::new(".").join(".kindlyklanklient"));
    let runtime_dir = kindly_dir.join("runtime");
    let java_dir = runtime_dir.join(format!("java-{}", version));
    fs::create_dir_all(&runtime_dir).await.map_err(|e| format!("Failed to create runtime directory: {}", e))?;
    let (os, arch, extension) = if cfg!(target_os = "windows") { ("windows", "x64", "zip") } else if cfg!(target_os = "macos") { ("mac", "x64", "tar.gz") } else { ("linux", "x64", "tar.gz") };
    let jre_url = format!("https://api.adoptium.net/v3/binary/latest/{}/ga/{}/{}/jdk/hotspot/normal/eclipse", version, os, arch);
    let client = reqwest::Client::new();
    let response = client.get(&jre_url).header("User-Agent", "KindlyKlanKlient/1.0").header("Accept", "application/octet-stream").send().await.map_err(|e| format!("Failed to download Java: {}", e))?;
    if !response.status().is_success() { return Err(format!("Download failed with status: {}", response.status())); }
    let bytes = response.bytes().await.map_err(|e| format!("Failed to read response: {}", e))?;
    let temp_file = runtime_dir.join(format!("java-{}.{}", version, extension));
    let mut file = File::create(&temp_file).map_err(|e| format!("Failed to create temp file: {}", e))?;
    file.write_all(&bytes).map_err(|e| format!("Failed to write temp file: {}", e))?;
    file.flush().map_err(|e| format!("Failed to flush file: {}", e))?; drop(file);
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    if java_dir.exists() { let _ = std::fs::remove_dir_all(&java_dir); }
    if temp_file.extension().map_or(false, |e| e == "zip") {
        let reader = std::fs::File::open(&temp_file).map_err(|e| format!("Open zip failed: {}", e))?;
        let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("Read zip failed: {}", e))?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| format!("Zip index failed: {}", e))?;
            let outpath = runtime_dir.join(file.mangled_name());
            if file.name().ends_with('/') { std::fs::create_dir_all(&outpath).map_err(|e| format!("Create dir failed: {}", e))?; } else {
                if let Some(p) = outpath.parent() { std::fs::create_dir_all(p).map_err(|e| format!("Create parent failed: {}", e))?; }
                let mut outfile = std::fs::File::create(&outpath).map_err(|e| format!("Create file failed: {}", e))?;
                std::io::copy(&mut file, &mut outfile).map_err(|e| format!("Write file failed: {}", e))?;
            }
        }
    } else {
        #[cfg(not(target_os = "windows"))]
        { return Err("Unsupported archive format on this OS without external tools".to_string()); }
    }
    let all_entries = std::fs::read_dir(&runtime_dir).map_err(|e| format!("Failed to read runtime directory: {}", e))?.collect::<Result<Vec<_>, _>>().map_err(|e| format!("Failed to read directory entries: {}", e))?;
    let extracted_dirs: Vec<_> = all_entries.into_iter().filter(|entry| { let path = entry.path(); path.is_dir() && path != java_dir }).map(|entry| entry.path()).collect();
    if let Some(extracted_dir) = extracted_dirs.first() {
        if java_dir.exists() { let _ = std::fs::remove_dir_all(&java_dir); }
        std::fs::rename(extracted_dir, &java_dir).map_err(|e| format!("Failed to move Java directory: {}", e))?;
        for dir in extracted_dirs.iter().skip(1) { let _ = std::fs::remove_dir_all(dir); }
    } else { return Err("No Java directory found after extraction".to_string()); }
    let _ = std::fs::remove_file(&temp_file);
    Ok(format!("Java {} downloaded and installed successfully", version))
}

#[tauri::command]
pub async fn get_java_path(version: String) -> Result<String, String> {
    let kindly_dir = std::env::var("USERPROFILE").map(|p| std::path::Path::new(&p).join(".kindlyklanklient")).unwrap_or_else(|_| std::path::Path::new(".").join(".kindlyklanklient"));
    let java_dir = kindly_dir.join("runtime").join(format!("java-{}", version));
    let java_path = if cfg!(target_os = "windows") { java_dir.join("bin").join("java.exe") } else { java_dir.join("bin").join("java") };
    if java_path.exists() { Ok(java_path.to_string_lossy().to_string()) } else { Err(format!("Java executable not found at: {}", java_path.display())) }
}

#[tauri::command]
pub async fn upload_skin_to_mojang(file_path: String, variant: String, access_token: String) -> Result<String, String> {
    use std::fs;
    let path = std::path::Path::new(&file_path);
    if !path.exists() { return Err(format!("File does not exist: {}", file_path)); }
    if path.extension().unwrap_or_default() != "png" { return Err("File must be a PNG image".to_string()); }
    let file_data = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
    if file_data.len() > 24 * 1024 { return Err("Skin file must be smaller than 24KB".to_string()); }
    let client = reqwest::Client::new();
    let form = reqwest::multipart::Form::new()
        .part("file", reqwest::multipart::Part::bytes(file_data).file_name("skin.png").mime_str("image/png").unwrap())
        .text("variant", variant);
    let response = client.post("https://api.minecraftservices.com/minecraft/profile/skins")
        .header("Authorization", format!("Bearer {}", access_token))
        .multipart(form).send().await.map_err(|e| format!("Failed to upload skin: {}", e))?;
    let status = response.status();
    let response_text = response.text().await.unwrap_or_default();
    if !status.is_success() { return Err(format!("Mojang API error ({}): {}", status, response_text)); }
    Ok("Skin uploaded successfully".to_string())
}

#[tauri::command]
pub async fn set_skin_variant(file_path: String, variant: String, access_token: String) -> Result<String, String> {
    use std::fs;
    let path = std::path::Path::new(&file_path);
    if !path.exists() { return Err(format!("File does not exist: {}", file_path)); }
    if path.extension().unwrap_or_default() != "png" { return Err("File must be a PNG image".to_string()); }
    let file_data = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
    if file_data.len() > 24 * 1024 { return Err("Skin file must be smaller than 24KB".to_string()); }
    let client = reqwest::Client::new();
    let form = reqwest::multipart::Form::new()
        .part("file", reqwest::multipart::Part::bytes(file_data).file_name("skin.png").mime_str("image/png").unwrap())
        .text("variant", variant);
    let response = client.post("https://api.minecraftservices.com/minecraft/profile/skins")
        .header("Authorization", format!("Bearer {}", access_token))
        .multipart(form).send().await.map_err(|e| format!("Failed to upload skin: {}", e))?;
    let status = response.status();
    let response_text = response.text().await.unwrap_or_default();
    if !status.is_success() { return Err(format!("Mojang API error ({}): {}", status, response_text)); }
    Ok("Skin variant updated".to_string())
}

#[tauri::command]
pub async fn create_temp_file(file_name: String, file_data: Vec<u8>) -> Result<String, String> {
    use std::fs::File;
    use std::io::Write;
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(&file_name);
    let mut file = File::create(&file_path).map_err(|e| format!("Failed to create temp file: {}", e))?;
    file.write_all(&file_data).map_err(|e| format!("Failed to write temp file: {}", e))?;
    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn check_for_updates(app_handle: AppHandle) -> Result<String, String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app_handle.updater().map_err(|e| format!("Failed to get updater: {}", e))?;
    let mut state = load_update_state().await;
    // Ensure that always use the real version
    state.current_version = env!("CARGO_PKG_VERSION").to_string();
    state.last_check = chrono::Utc::now().to_rfc3339();
    match updater.check().await {
        Ok(update) => {
            if let Some(update) = update {
                state.available_version = Some(update.version.clone());
                state.downloaded = false;
                state.download_ready = false;
                state.manual_download = false;
                save_update_state(&state).await?;
                Ok(format!("Update available: {}", update.version))
            } else {
                state.available_version = None;
                state.downloaded = false;
                state.download_ready = false;
                state.manual_download = false;
                save_update_state(&state).await?;
                Ok("No updates available".to_string())
            }
        }
        Err(e) => {
            save_update_state(&state).await?;
            Err(format!("Failed to check for updates: {}", e))
        }
    }
}

#[tauri::command]
pub async fn install_update(app_handle: AppHandle) -> Result<String, String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app_handle.updater().map_err(|e| format!("Failed to get updater: {}", e))?;
    match updater.check().await {
        Ok(update) => {
            if let Some(update) = update {
                app_handle.emit("update-install-start", ()).unwrap_or_default();
                update.download_and_install(
                    |chunk_length, content_length| {
                        println!("Downloading and installing update: {} of {:?}", chunk_length, content_length);
                        let percentage = if let Some(total) = content_length {
                            ((chunk_length as f64 / total as f64) * 100.0) as u32
                        } else {
                            0
                        };
                        let _ = app_handle.emit("update-download-progress", percentage);
                    },
                    || {
                        println!("Install finished - app will restart");
                        let _ = app_handle.emit("update-install-complete", ());
                    }
                ).await.map_err(|e| format!("Failed to install update: {}", e))?;
                // Clear the state after the installation
                let mut new_state = load_update_state().await;
                new_state.downloaded = false;
                new_state.download_ready = false;
                new_state.manual_download = false;
                save_update_state(&new_state).await?;
                Ok("Update installed successfully".to_string())
            } else {
                Ok("No update available to install".to_string())
            }
        }
        Err(e) => Err(format!("Failed to check for updates: {}", e))
    }
}

fn update_state_path() -> std::path::PathBuf {
    let base = std::env::var("USERPROFILE")
        .map(|p| std::path::Path::new(&p).join(".kindlyklanklient"))
        .unwrap_or_else(|_| std::path::Path::new(".").join(".kindlyklanklient"));
    base.join("update_state.json")
}

async fn save_update_state(state: &UpdateState) -> Result<(), String> {
    let path = update_state_path();
    if let Some(dir) = path.parent() { tokio::fs::create_dir_all(dir).await.map_err(|e| e.to_string())?; }
    let data = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    tokio::fs::write(&path, data).await.map_err(|e| e.to_string())
}

async fn read_update_state_file() -> Option<UpdateState> {
    let path = update_state_path();
    let Ok(text) = tokio::fs::read_to_string(&path).await else { return None; };
    serde_json::from_str(&text).ok()
}

async fn load_update_state() -> UpdateState {
    let real_version = env!("CARGO_PKG_VERSION").to_string();
    if let Some(mut state) = read_update_state_file().await {
        // Always use the real version from Cargo.toml, not the saved one
        state.current_version = real_version;
        // Si es un estado antiguo sin manual_download, establecerlo a false por defecto
        // serde debería manejar esto, pero por si acaso:
        return state;
    }
    UpdateState { last_check: String::new(), available_version: None, current_version: real_version, downloaded: false, download_ready: false, manual_download: false }
}

#[tauri::command]
pub async fn get_update_state() -> Result<UpdateState, String> {
    let mut state = load_update_state().await;
    state.current_version = env!("CARGO_PKG_VERSION").to_string();
    Ok(state)
}

#[tauri::command]
pub async fn save_update_state_command(state: UpdateState) -> Result<String, String> {
    save_update_state(&state).await?;
    Ok("ok".to_string())
}

#[tauri::command]
pub async fn clear_update_state() -> Result<String, String> {
    let path = update_state_path();
    if tokio::fs::try_exists(&path).await.map_err(|e| e.to_string())? {
        tokio::fs::remove_file(&path).await.map_err(|e| e.to_string())?;
    }
    Ok("ok".to_string())
}

#[tauri::command]
pub async fn download_update_silent(app_handle: AppHandle, manual: Option<bool>) -> Result<String, String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app_handle.updater().map_err(|e| format!("Failed to get updater: {}", e))?;
    let is_manual = manual.unwrap_or(false);
    match updater.check().await {
        Ok(Some(update)) => {
            // Emit the download start event
            let _ = app_handle.emit("update-download-start", ());
            
            // Download the update with callbacks to report progress
            update.download(
                |chunk_length, content_length| {
                    let percentage = if let Some(total) = content_length {
                        ((chunk_length as f64 / total as f64) * 100.0) as u32
                    } else {
                        0
                    };
                    let _ = app_handle.emit("update-download-progress", percentage);
                },
                || {
                    let _ = app_handle.emit("update-download-complete", ());
                }
            ).await.map_err(|e| format!("Failed to download update: {}", e))?;
            
            // Update the state to indicate that the download is ready
            let mut state = load_update_state().await;
            state.available_version = Some(update.version.clone());
            state.downloaded = true;
            state.download_ready = true;
            state.manual_download = is_manual;
            save_update_state(&state).await?;
            Ok("downloaded successfully".to_string())
        }
        Ok(None) => Ok("no-update".to_string()),
        Err(e) => Err(format!("Failed to check for updates: {}", e))
    }
}

#[tauri::command]
pub async fn download_instance_assets(
    app_handle: AppHandle,
    instance_id: String,
    minecraft_version: String,
    base_url: Option<String>,
    instance_url: Option<String>
) -> Result<String, String> {
    let base = std::env::var("USERPROFILE")
        .map(|p| std::path::Path::new(&p).join(".kindlyklanklient"))
        .unwrap_or_else(|_| std::path::Path::new(".").join(".kindlyklanklient"));
    let instance_dir = base.join(&instance_id);
    let _ = tokio::fs::create_dir_all(instance_dir.join("libraries")).await;
    let _ = tokio::fs::create_dir_all(instance_dir.join("mods")).await;
    let _ = app_handle.emit("asset-download-progress", serde_json::json!({
        "current": 0,
        "total": 1,
        "percentage": 0,
        "current_file": "",
        "status": "Starting"
    }));
    let _ = app_handle.emit("asset-download-progress", serde_json::json!({
        "current": 0,
        "total": 100,
        "percentage": 1,
        "current_file": "",
        "status": "Version"
    }));
    crate::instances::ensure_minecraft_client_present(&instance_dir, &minecraft_version).await?;
    
    let mut instance_manifest_for_assets: Option<crate::models::InstanceManifest> = None;
    let mut base_url_for_assets: Option<String> = None;
    if let (Some(base_ml), Some(inst_url_ml)) = (base_url.clone(), instance_url.clone()) {
        base_url_for_assets = Some(base_ml.clone());
        let full_url = if inst_url_ml.starts_with("http") { inst_url_ml } else { format!("{}/{}", base_ml.trim_end_matches('/'), inst_url_ml.trim_start_matches('/')) };
        let client = reqwest::Client::new();
        let response = client.get(&full_url).send().await.map_err(|e| format!("Failed to fetch instance details: {}", e))?;
        if !response.status().is_success() { return Err(format!("HTTP error: {}", response.status())); }
        let manifest: crate::models::InstanceManifest = response.json().await.map_err(|e| format!("Failed to parse instance JSON: {}", e))?;
        instance_manifest_for_assets = Some(manifest.clone());
        if let Some(mod_loader) = manifest.instance.mod_loader.as_ref() {
            let _ = app_handle.emit("asset-download-progress", serde_json::json!({
                "current": 3,
                "total": 100,
                "percentage": 3,
                "current_file": "",
                "status": "ModLoader"
            }));
            crate::instances::install_mod_loader(&minecraft_version, mod_loader, &instance_dir).await?;
        }
    }

    let _ = app_handle.emit("asset-download-progress", serde_json::json!({
        "current": 10,
        "total": 100,
        "percentage": 10,
        "current_file": "",
        "status": "Mojang"
    }));
    let _ = crate::instances::ensure_assets_present_with_progress(&app_handle, &instance_dir, &minecraft_version, None).await?;
    
    let _ = app_handle.emit("asset-download-progress", serde_json::json!({
        "current": 90,
        "total": 100,
        "percentage": 90,
        "current_file": "",
        "status": "Libraries"
    }));
    crate::instances::ensure_version_libraries(&instance_dir, &minecraft_version).await?;
    
    if let (Some(instance), Some(base)) = (instance_manifest_for_assets, base_url_for_assets) {
        let _ = app_handle.emit("asset-download-progress", serde_json::json!({
            "current": 95,
            "total": 100,
            "percentage": 95,
            "current_file": "",
            "status": "Instance"
        }));
        use std::collections::HashSet;
        let mut expected_mods: HashSet<String> = HashSet::new();
        for mod_file in &instance.files.mods {
            let file_url = if mod_file.url.starts_with("http") { mod_file.url.clone() } else { format!("{}/{}", base.trim_end_matches('/'), mod_file.url.trim_start_matches('/')) };
            let target_path = instance_dir.join("mods").join(&mod_file.name);
            expected_mods.insert(mod_file.name.clone());
            if let Some(parent) = target_path.parent() { tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?; }
            let mut needs_download = true;
            if target_path.exists() {
                if !mod_file.sha256.is_empty() {
                    if crate::instances::verify_file_checksum(&target_path, &mod_file.sha256).is_ok() { needs_download = false; }
                } else if let Some(md5) = mod_file.md5.as_ref() {
                    if !md5.is_empty() {
                        if crate::instances::verify_file_md5(&target_path, md5).is_ok() { needs_download = false; }
                    }
                }
            }
            if needs_download { crate::instances::download_file(&file_url, &target_path).await.map_err(|e| e.to_string())?; }
        }
        let mods_dir = instance_dir.join("mods");
        if mods_dir.exists() {
            for entry in std::fs::read_dir(&mods_dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.file_type().map_err(|e| e.to_string())?.is_file() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !expected_mods.contains(&name) { let _ = std::fs::remove_file(entry.path()); }
                }
            }
        }

        let mut expected_configs: HashSet<String> = HashSet::new();
        for config_file in &instance.files.configs {
            let file_url = if config_file.url.starts_with("http") { config_file.url.clone() } else { format!("{}/{}", base.trim_end_matches('/'), config_file.url.trim_start_matches('/')) };
            let mut rel = config_file.target.clone().unwrap_or(config_file.path.clone());
            if rel == "config/options.txt" { rel = "options.txt".to_string(); }
            if rel.starts_with("config/config/") { rel = rel.replacen("config/config/", "config/", 1); }
            else if rel.starts_with("config/") { rel = rel.replacen("config/", "config/", 1); }
            expected_configs.insert(rel.clone());
            let target_path = instance_dir.join(&rel);
            if let Some(parent) = target_path.parent() { tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?; }
            let mut needs_download = true;
            if target_path.exists() {
                if !config_file.sha256.is_empty() {
                    if crate::instances::verify_file_checksum(&target_path, &config_file.sha256).is_ok() { needs_download = false; }
                } else if let Some(md5) = config_file.md5.as_ref() {
                    if !md5.is_empty() {
                        if crate::instances::verify_file_md5(&target_path, md5).is_ok() { needs_download = false; }
                    }
                }
            }
            if needs_download { crate::instances::download_file(&file_url, &target_path).await.map_err(|e| e.to_string())?; }
        }       
        let config_dir = instance_dir.join("config");
        if config_dir.exists() {
            for entry in walkdir::WalkDir::new(&config_dir) {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.file_type().is_file() {
                    let rel_path = entry.path().strip_prefix(&instance_dir).map_err(|e| e.to_string())?.to_string_lossy().replace('\\', "/");
                    if !expected_configs.contains(&rel_path) { let _ = std::fs::remove_file(entry.path()); }
                }
            }
        }
        let root_options = instance_dir.join("options.txt");
        if root_options.exists() && !expected_configs.contains("options.txt") { let _ = std::fs::remove_file(&root_options); }
    }
    
    let _ = app_handle.emit("asset-download-completed", serde_json::json!({ "phase": "complete" }));
    Ok("ok".to_string())
}

#[tauri::command]
pub async fn load_distribution_manifest(url: String) -> Result<DistributionManifest, String> {
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await.map_err(|e| format!("Failed to fetch manifest: {}", e))?;
    if !response.status().is_success() { return Err(format!("HTTP error: {}", response.status())); }
    let manifest: DistributionManifest = response.json().await.map_err(|e| format!("Failed to parse manifest JSON: {}", e))?;
    Ok(manifest)
}

#[tauri::command]
pub async fn get_instance_background_video(
    base_url: String,
    instance_id: String,
    video_path: String,
) -> Result<Vec<u8>, String> {
    use std::path::Path;
    
    let launcher = crate::launcher::MinecraftLauncher::new().map_err(|e| e.to_string())?;
    let instance_dir = launcher.config.minecraft_dir.join("instances").join(&instance_id);
    let video_dir = instance_dir.join("assets");
    tokio::fs::create_dir_all(&video_dir).await.map_err(|e| e.to_string())?;
    
    // Construir nombre del archivo desde la ruta (ej: "instances/thanatophobia2/assets/th2trailer.mp4" -> "th2trailer.mp4")
    let video_file_name = Path::new(&video_path)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid video path".to_string())?;
    
    let local_video_path = video_dir.join(video_file_name);
    
    // Si el video no existe localmente, descargarlo
    if !local_video_path.exists() {
        // Construir URL completa del video
        let video_url = if video_path.starts_with("http") {
            video_path
        } else {
            format!("{}/{}", base_url.trim_end_matches('/'), video_path.trim_start_matches('/'))
        };
        
        // Descargar el video
        crate::instances::download_file(&video_url, &local_video_path).await.map_err(|e| e.to_string())?;
    }
    
    // Leer el archivo como bytes
    let video_bytes = tokio::fs::read(&local_video_path).await.map_err(|e| format!("Failed to read video file: {}", e))?;
    
    Ok(video_bytes)
}

#[tauri::command]
pub async fn get_instance_details(base_url: String, instance_url: String) -> Result<InstanceManifest, String> {
    let full_url = if instance_url.starts_with("http") { instance_url } else { format!("{}/{}", base_url.trim_end_matches('/'), instance_url.trim_start_matches('/')) };
    let client = reqwest::Client::new();
    let response = client.get(&full_url).send().await.map_err(|e| format!("Failed to fetch instance details: {}", e))?;
    if !response.status().is_success() { return Err(format!("HTTP error: {}", response.status())); }
    let instance: InstanceManifest = response.json().await.map_err(|e| format!("Failed to parse instance JSON: {}", e))?;
    Ok(instance)
}

#[tauri::command]
pub async fn download_instance(
    base_url: String,
    instance: InstanceManifest,
    _session: crate::AuthSession
) -> Result<String, String> {
    let launcher = crate::launcher::MinecraftLauncher::new().map_err(|e| e.to_string())?;
    launcher.config.ensure_directories().await.map_err(|e| e.to_string())?;
    let instance_dir = launcher.config.versions_dir.join(&instance.instance.id);
    tokio::fs::create_dir_all(&instance_dir).await.map_err(|e| e.to_string())?;
    let versions = launcher.get_available_versions().await.map_err(|e| e.to_string())?;
    if let Some(mc_version) = versions.into_iter().find(|v| v.id == instance.instance.minecraft_version) {
        launcher.download_version(&mc_version).await.map_err(|e| e.to_string())?;
    } else {
        return Err(format!("Minecraft version {} not found", instance.instance.minecraft_version));
    }
    if let Some(_mod_loader) = &instance.instance.mod_loader { /* reserved */ }
    for mod_file in &instance.files.mods {
        let file_url = if mod_file.url.starts_with("http") { mod_file.url.clone() } else { format!("{}/{}", base_url.trim_end_matches('/'), mod_file.url.trim_start_matches('/')) };
        let target_path = launcher.config.minecraft_dir.join("instances").join(&instance.instance.id).join("mods").join(&mod_file.name);
        if let Some(parent) = target_path.parent() { tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?; }
        crate::instances::download_file(&file_url, &target_path).await.map_err(|e| e.to_string())?;
    }
    for config_file in &instance.files.configs {
        let file_url = if config_file.url.starts_with("http") { config_file.url.clone() } else { format!("{}/{}", base_url.trim_end_matches('/'), config_file.url.trim_start_matches('/')) };
        let target_path = launcher.config.minecraft_dir.join("instances").join(&instance.instance.id).join(config_file.target.as_ref().unwrap_or(&config_file.path));
        if let Some(parent) = target_path.parent() { tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?; }
        crate::instances::download_file(&file_url, &target_path).await.map_err(|e| e.to_string())?;
    }
    Ok(format!("Instance {} ready to launch!", instance.instance.name))
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ProfileResponse {
    pub status: String,
    pub profile: Option<serde_json::Value>,
    pub code: Option<String>,
    pub message: Option<String>,
}

#[tauri::command]
pub async fn get_minecraft_profile_safe(access_token: String) -> Result<ProfileResponse, String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;
    if response.status().as_u16() == 401 {
        return Ok(ProfileResponse { status: "Err".into(), profile: None, code: Some("PROFILE_401".into()), message: Some("Unauthorized".into()) });
    }
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Ok(ProfileResponse { status: "Err".into(), profile: None, code: Some("PROFILE_OTHER".into()), message: Some(error_text) });
    }
    let json = response.json::<serde_json::Value>().await.map_err(|e| format!("Failed to parse profile json: {}", e))?;
    Ok(ProfileResponse { status: "Ok".into(), profile: Some(json), code: None, message: None })
}

pub fn get_required_java_version(minecraft_version: &str) -> String {
    let version_parts: Vec<&str> = minecraft_version.split('.').collect();
    let minor_version = version_parts.get(1).unwrap_or(&"8").parse::<u32>().unwrap_or(8);
    match minor_version {
        21..=u32::MAX => "21".to_string(),
        20..=20 => "17".to_string(),
        18..=19 => "17".to_string(),
        17..=17 => "16".to_string(),
        8..=16 => "8".to_string(),
        _ => "8".to_string(),
    }
}

#[tauri::command]
pub async fn save_advanced_config(
    jvm_args: String,
    garbage_collector: String,
    window_width: u32,
    window_height: u32
) -> Result<(), String> {
    use std::fs;
    let config_dir = dirs::config_dir().ok_or("Could not find config directory")?.join("KindlyKlanKlient");
    fs::create_dir_all(&config_dir).map_err(|e| format!("Failed to create config directory: {}", e))?;
    let config_file = config_dir.join("advanced_config.json");
    let config = serde_json::json!({
        "jvm_args": jvm_args,
        "garbage_collector": garbage_collector,
        "window_width": window_width,
        "window_height": window_height,
        "last_updated": chrono::Utc::now().to_rfc3339()
    });
    fs::write(&config_file, serde_json::to_string_pretty(&config).unwrap())
        .map_err(|e| format!("Failed to write config file: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn load_advanced_config() -> Result<(String, String, u32, u32), String> {
    use std::fs;
    let config_dir = dirs::config_dir().ok_or("Could not find config directory")?.join("KindlyKlanKlient");
    let config_file = config_dir.join("advanced_config.json");
    if !config_file.exists() {
        return Ok((String::new(), "G1".to_string(), 1280, 720));
    }
    let config_content = fs::read_to_string(&config_file).map_err(|e| format!("Failed to read config file: {}", e))?;
    let config: serde_json::Value = serde_json::from_str(&config_content).map_err(|e| format!("Failed to parse config file: {}", e))?;
    let jvm_args = config["jvm_args"].as_str().unwrap_or("").to_string();
    let garbage_collector = config["garbage_collector"].as_str().unwrap_or("G1").to_string();
    let window_width = config["window_width"].as_u64().unwrap_or(1280) as u32;
    let window_height = config["window_height"].as_u64().unwrap_or(720) as u32;
    Ok((jvm_args, garbage_collector, window_width, window_height))
}

#[tauri::command]
pub async fn save_ram_config(min_ram: f64, max_ram: f64) -> Result<(), String> {
    use std::fs;
    let config_dir = dirs::config_dir().ok_or("Could not find config directory")?.join("KindlyKlanKlient");
    fs::create_dir_all(&config_dir).map_err(|e| format!("Failed to create config directory: {}", e))?;
    let config_file = config_dir.join("ram_config.json");
    let config = serde_json::json!({
        "min_ram": min_ram,
        "max_ram": max_ram,
        "last_updated": chrono::Utc::now().to_rfc3339()
    });
    fs::write(&config_file, serde_json::to_string_pretty(&config).unwrap())
        .map_err(|e| format!("Failed to write config file: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn load_ram_config() -> Result<(f64, f64), String> {
    use std::fs;
    let config_dir = dirs::config_dir().ok_or("Could not find config directory")?.join("KindlyKlanKlient");
    let config_file = config_dir.join("ram_config.json");
    if !config_file.exists() { return Ok((2.0, 4.0)); }
    let config_content = fs::read_to_string(&config_file).map_err(|e| format!("Failed to read config file: {}", e))?;
    let config: serde_json::Value = serde_json::from_str(&config_content).map_err(|e| format!("Failed to parse config file: {}", e))?;
    let min_ram = config["min_ram"].as_f64().unwrap_or(2.0);
    let max_ram = config["max_ram"].as_f64().unwrap_or(4.0);
    Ok((min_ram, max_ram))
}

#[tauri::command]
pub fn get_system_ram() -> Result<u32, String> {
    use sysinfo::System;
    let mut system = System::new_all();
    system.refresh_memory();
    let total_memory_bytes = system.total_memory();
    let total_memory_gb = (total_memory_bytes / (1024 * 1024 * 1024)) as u32;
    Ok(std::cmp::max(total_memory_gb, 4))
}

#[tauri::command]
pub async fn stop_minecraft_instance(instance_id: String) -> Result<String, String> {
    Ok(format!("Minecraft instance {} stopped", instance_id))
}

#[tauri::command]
pub async fn restart_application() -> Result<String, String> {
    Ok("Application will be restarted".to_string())
}


