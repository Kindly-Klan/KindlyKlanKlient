use crate::versions::MinecraftVersion;
use crate::launcher::MinecraftLauncher;
use crate::AuthSession;
use tokio::fs;
use std::fs::File;
use std::io::Write;
use reqwest;
use tauri::{AppHandle, State};
use tauri::Emitter;
use crate::UpdateState;
use crate::{DistributionManifest, InstanceManifest};
use std::sync::{Arc, Mutex};
use crate::models::{ForgeVersion, NeoForgeVersion};

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
pub async fn set_downloading_state(state: State<'_, Arc<Mutex<bool>>>, is_downloading: bool) -> Result<(), String> {
    if let Ok(mut downloading) = state.lock() {
        *downloading = is_downloading;
    }
    Ok(())
}

#[tauri::command]
pub async fn download_java(version: String, app_handle: AppHandle, state: State<'_, Arc<Mutex<bool>>>) -> Result<String, String> {
    // Establecer estado de descarga
    if let Ok(mut downloading) = state.lock() {
        *downloading = true;
    }
    
    // Notificar que comenzó la descarga
    let _ = app_handle.emit("java-download-started", serde_json::json!({ "version": version }));
    
    let kindly_dir = std::env::var("USERPROFILE")
        .map(|p| std::path::Path::new(&p).join(".kindlyklanklient"))
        .unwrap_or_else(|_| std::path::Path::new(".").join(".kindlyklanklient"));
    let runtime_dir = kindly_dir.join("runtime");
    let java_dir = runtime_dir.join(format!("java-{}", version));
    fs::create_dir_all(&runtime_dir).await.map_err(|e| format!("Failed to create runtime directory: {}", e))?;
    let (os, arch, extension) = if cfg!(target_os = "windows") { ("windows", "x64", "zip") } else if cfg!(target_os = "macos") { ("mac", "x64", "tar.gz") } else { ("linux", "x64", "tar.gz") };
    let jre_url = format!("https://api.adoptium.net/v3/binary/latest/{}/ga/{}/{}/jdk/hotspot/normal/eclipse", version, os, arch);
    
    // Emitir progreso inicial
    let _ = app_handle.emit("java-download-progress", serde_json::json!({
        "percentage": 0,
        "status": "Descargando Java..."
    }));
    
    let client = reqwest::Client::new();
    let response = client.get(&jre_url).header("User-Agent", "KindlyKlanKlient/1.0").header("Accept", "application/octet-stream").send().await.map_err(|e| format!("Failed to download Java: {}", e))?;
    if !response.status().is_success() { return Err(format!("Download failed with status: {}", response.status())); }
    
    // Obtener tamaño total si está disponible
    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded = 0u64;
    
    // Emitir progreso durante descarga
    let _ = app_handle.emit("java-download-progress", serde_json::json!({
        "percentage": 10,
        "status": "Descargando Java..."
    }));
    
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    use futures_util::TryStreamExt;
    loop {
        match stream.try_next().await {
            Ok(Some(chunk)) => {
                downloaded += chunk.len() as u64;
                bytes.extend_from_slice(&chunk);
                
                // Actualizar progreso cada 5%
                if total_size > 0 {
                    let percentage = ((downloaded * 100) / total_size).min(80);
                    let _ = app_handle.emit("java-download-progress", serde_json::json!({
                        "percentage": percentage,
                        "status": "Descargando Java..."
                    }));
                }
            }
            Ok(None) => break,
            Err(e) => return Err(format!("Failed to read chunk: {}", e)),
        }
    }
    
    let temp_file = runtime_dir.join(format!("java-{}.{}", version, extension));
    let mut file = File::create(&temp_file).map_err(|e| format!("Failed to create temp file: {}", e))?;
    file.write_all(&bytes).map_err(|e| format!("Failed to write temp file: {}", e))?;
    file.flush().map_err(|e| format!("Failed to flush file: {}", e))?; 
    drop(file);
    
    // Emitir progreso de extracción
    let _ = app_handle.emit("java-download-progress", serde_json::json!({
        "percentage": 85,
        "status": "Extrayendo Java..."
    }));
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    if java_dir.exists() { let _ = std::fs::remove_dir_all(&java_dir); }
    if temp_file.extension().map_or(false, |e| e == "zip") {
        let reader = std::fs::File::open(&temp_file).map_err(|e| format!("Open zip failed: {}", e))?;
        let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("Read zip failed: {}", e))?;
        let total_files = archive.len();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| format!("Zip index failed: {}", e))?;
            let outpath = runtime_dir.join(file.mangled_name());
            if file.name().ends_with('/') { std::fs::create_dir_all(&outpath).map_err(|e| format!("Create dir failed: {}", e))?; } else {
                if let Some(p) = outpath.parent() { std::fs::create_dir_all(p).map_err(|e| format!("Create parent failed: {}", e))?; }
                let mut outfile = std::fs::File::create(&outpath).map_err(|e| format!("Create file failed: {}", e))?;
                std::io::copy(&mut file, &mut outfile).map_err(|e| format!("Write file failed: {}", e))?;
            }
            // Actualizar progreso de extracción
            let extraction_progress = 85 + ((i * 10) / total_files);
            let _ = app_handle.emit("java-download-progress", serde_json::json!({
                "percentage": extraction_progress,
                "status": "Extrayendo Java..."
            }));
        }
    } else {
        #[cfg(not(target_os = "windows"))]
        { return Err("Unsupported archive format on this OS without external tools".to_string()); }
    }
    
    // Emitir progreso final
    let _ = app_handle.emit("java-download-progress", serde_json::json!({
        "percentage": 95,
        "status": "Finalizando instalación..."
    }));
    
    let all_entries = std::fs::read_dir(&runtime_dir).map_err(|e| format!("Failed to read runtime directory: {}", e))?.collect::<Result<Vec<_>, _>>().map_err(|e| format!("Failed to read directory entries: {}", e))?;
    let extracted_dirs: Vec<_> = all_entries.into_iter().filter(|entry| { let path = entry.path(); path.is_dir() && path != java_dir }).map(|entry| entry.path()).collect();
    if let Some(extracted_dir) = extracted_dirs.first() {
        if java_dir.exists() { let _ = std::fs::remove_dir_all(&java_dir); }
        std::fs::rename(extracted_dir, &java_dir).map_err(|e| format!("Failed to move Java directory: {}", e))?;
        for dir in extracted_dirs.iter().skip(1) { let _ = std::fs::remove_dir_all(dir); }
    } else { return Err("No Java directory found after extraction".to_string()); }
    let _ = std::fs::remove_file(&temp_file);
    
    // Emitir progreso completado
    let _ = app_handle.emit("java-download-progress", serde_json::json!({
        "percentage": 100,
        "status": "Completado"
    }));
    let _ = app_handle.emit("java-download-completed", serde_json::json!({ "version": version }));
    
    // Limpiar estado de descarga
    if let Ok(mut downloading) = state.lock() {
        *downloading = false;
    }
    
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
                // Limpiar el estado ANTES de instalar para evitar que se quede en "necesita instalar"
                let mut new_state = load_update_state().await;
                new_state.downloaded = false;
                new_state.download_ready = false;
                new_state.manual_download = false;
                new_state.available_version = None; // Limpiar también la versión disponible
                save_update_state(&new_state).await.ok(); // Intentar guardar, pero no fallar si no se puede
                
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
    instance_id: String,
    minecraft_version: String,
    base_url: Option<String>,
    instance_url: Option<String>,
    app_handle: AppHandle,
    state: State<'_, Arc<Mutex<bool>>>
) -> Result<String, String> {
    // Establecer estado de descarga
    if let Ok(mut downloading) = state.lock() {
        *downloading = true;
    }
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
    let mut installed_mod_loader_version_id: Option<String> = None;
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
            installed_mod_loader_version_id = crate::instances::install_mod_loader(&minecraft_version, mod_loader, &instance_dir).await?;
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
    
    // Download mod loader libraries if applicable (using the version_id returned by install_mod_loader)
    if let Some(version_id) = &installed_mod_loader_version_id {
        crate::instances::ensure_mod_loader_libraries(&instance_dir, version_id).await?;
    }
    
    if let (Some(instance), Some(base)) = (instance_manifest_for_assets, base_url_for_assets) {
        let _ = app_handle.emit("asset-download-progress", serde_json::json!({
            "current": 95,
            "total": 100,
            "percentage": 95,
            "current_file": "",
            "status": "Instance"
        }));
        // Cargar historial de manifest anterior
        let previous_history = crate::instances::load_manifest_history(&instance_dir)?;
        
        // Obtener patrones de archivos ignorados
        let ignored_patterns = instance.ignored_files.as_ref();
        let empty_vec = Vec::<String>::new();
        let ignored_mods = ignored_patterns.map(|p| &p.mods).unwrap_or(&empty_vec);
        let ignored_configs = ignored_patterns.map(|p| &p.configs).unwrap_or(&empty_vec);
        let ignored_resourcepacks = ignored_patterns.map(|p| &p.resourcepacks).unwrap_or(&empty_vec);
        let ignored_shaderpacks = ignored_patterns.map(|p| &p.shaderpacks).unwrap_or(&empty_vec);
        
        use std::collections::HashSet;
        let mut expected_mods: HashSet<String> = HashSet::new();
        for mod_file in &instance.files.mods {
            // Verificar si el archivo está en ignored_files - si está, NO descargarlo ni sobrescribirlo
            let should_ignore = crate::utils::matches_glob_patterns(&mod_file.name, ignored_mods);
            if should_ignore {
                // Archivo ignorado: no descargar ni sobrescribir
                continue;
            }
            
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
        
        // Limpiar mods: solo borrar si estaba en el historial pero ya no está en el manifest actual
        if let Some(history) = &previous_history {
            let mods_dir = instance_dir.join("mods");
            if mods_dir.exists() {
                for entry in std::fs::read_dir(&mods_dir).map_err(|e| e.to_string())? {
                    let entry = entry.map_err(|e| e.to_string())?;
                    if entry.file_type().map_err(|e| e.to_string())?.is_file() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        // Solo borrar si estaba en el historial pero ya no está en el manifest actual
                        if history.files.mods.contains(&name) && !expected_mods.contains(&name) {
                            let should_ignore = crate::utils::matches_glob_patterns(&name, ignored_mods);
                            if !should_ignore {
                                let _ = std::fs::remove_file(entry.path());
                            }
                        }
                    }
                }
            }
        }

        let mut expected_configs: HashSet<String> = HashSet::new();
        let mut expected_root_files: HashSet<String> = HashSet::new();
        for config_file in &instance.files.configs {
            let file_url = if config_file.url.starts_with("http") { config_file.url.clone() } else { format!("{}/{}", base.trim_end_matches('/'), config_file.url.trim_start_matches('/')) };
            let mut rel = config_file.target.clone().unwrap_or(config_file.path.clone());
            if rel == "config/options.txt" { rel = "options.txt".to_string(); }
            if rel.starts_with("config/config/") { rel = rel.replacen("config/config/", "config/", 1); }
            else if rel.starts_with("config/") { rel = rel.replacen("config/", "config/", 1); }
            expected_configs.insert(rel.clone());
            
            // Si está en la raíz, también agregarlo a expected_root_files
            if !rel.contains('/') {
                expected_root_files.insert(rel.clone());
            }
            
            // Verificar si el archivo está en ignored_files - si está, NO descargarlo ni sobrescribirlo
            let file_name_for_check = if !rel.contains('/') {
                rel.clone() // Para archivos en la raíz, usar solo el nombre
            } else {
                // Para archivos en subdirectorios, extraer solo el nombre del archivo
                rel.split('/').last().unwrap_or(&rel).to_string()
            };
            let should_ignore = crate::utils::matches_glob_patterns(&file_name_for_check, ignored_configs);
            
            if should_ignore {
                // Archivo ignorado: no descargar ni sobrescribir
                continue;
            }
            
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
        
        // Limpiar configs: solo borrar si estaba en el historial pero ya no está en el manifest actual
        if let Some(history) = &previous_history {
            let config_dir = instance_dir.join("config");
            if config_dir.exists() {
                for entry in walkdir::WalkDir::new(&config_dir) {
                    let entry = entry.map_err(|e| e.to_string())?;
                    if entry.file_type().is_file() {
                        let rel_path = entry.path().strip_prefix(&instance_dir).map_err(|e| e.to_string())?.to_string_lossy().replace('\\', "/");
                        // Solo borrar si estaba en el historial pero ya no está en el manifest actual
                        if history.files.configs.contains(&rel_path) && !expected_configs.contains(&rel_path) {
                            let should_ignore = crate::utils::matches_glob_patterns(&rel_path, ignored_configs);
                            if !should_ignore {
                                let _ = std::fs::remove_file(entry.path());
                            }
                        }
                    }
                }
            }
        }
        
        // Limpiar archivos en la raíz: solo borrar si estaban en el historial pero ya no están en el manifest actual
        if let Some(history) = &previous_history {
            if let Ok(entries) = std::fs::read_dir(&instance_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                            // Ignorar archivos del sistema (.manifest_history.json, etc.)
                            if file_name.starts_with('.') {
                                continue;
                            }
                            // Solo procesar archivos que estaban en el historial de root_files
                            if history.files.root_files.contains(&file_name.to_string()) && !expected_root_files.contains(file_name) {
                                let should_ignore = crate::utils::matches_glob_patterns(file_name, ignored_configs);
                                if !should_ignore {
                                    let _ = std::fs::remove_file(&path);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Limpiar resourcepacks: solo borrar si estaban en el historial pero ya no están en el manifest actual
        if let Some(history) = &previous_history {
            let mut expected_resourcepacks: HashSet<String> = HashSet::new();
            if let Some(resourcepacks) = &instance.files.resourcepacks {
                for rp_file in resourcepacks {
                    expected_resourcepacks.insert(rp_file.name.clone());
                }
            }
            
            let resourcepacks_dir = instance_dir.join("resourcepacks");
            if resourcepacks_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&resourcepacks_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                                if history.files.resourcepacks.contains(&file_name.to_string()) && !expected_resourcepacks.contains(file_name) {
                                    let should_ignore = crate::utils::matches_glob_patterns(file_name, ignored_resourcepacks);
                                    if !should_ignore {
                                        let _ = std::fs::remove_file(&path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Limpiar shaderpacks: solo borrar si estaban en el historial pero ya no están en el manifest actual
        if let Some(history) = &previous_history {
            let mut expected_shaderpacks: HashSet<String> = HashSet::new();
            if let Some(shaderpacks) = &instance.files.shaderpacks {
                for sp_file in shaderpacks {
                    expected_shaderpacks.insert(sp_file.name.clone());
                }
            }
            
            let shaderpacks_dir = instance_dir.join("shaderpacks");
            if shaderpacks_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&shaderpacks_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                                if history.files.shaderpacks.contains(&file_name.to_string()) && !expected_shaderpacks.contains(file_name) {
                                    let should_ignore = crate::utils::matches_glob_patterns(file_name, ignored_shaderpacks);
                                    if !should_ignore {
                                        let _ = std::fs::remove_file(&path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Guardar el nuevo historial después de procesar todos los archivos
        crate::instances::save_manifest_history(&instance_dir, &instance).await?;
    }
    
    let _ = app_handle.emit("asset-download-progress", serde_json::json!({
        "current": 100,
        "total": 100,
        "percentage": 100,
        "current_file": "",
        "status": "Completado"
    }));
    let _ = app_handle.emit("asset-download-completed", serde_json::json!({ "phase": "complete" }));
    
    // Limpiar estado de descarga
    if let Ok(mut downloading) = state.lock() {
        *downloading = false;
    }
    
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

// ============================================================================
// Forge API Commands
// ============================================================================

#[tauri::command]
pub async fn get_forge_versions(minecraft_version: String) -> Result<Vec<ForgeVersion>, String> {
    log::info!("🔍 Obteniendo versiones de Forge para Minecraft {}", minecraft_version);
    
    let client = reqwest::Client::new();
    
    // Intentar obtener desde el API de maven-metadata.xml
    let url = format!(
        "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml"
    );
    
    match client.get(&url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let xml_text = response.text().await.map_err(|e| e.to_string())?;
                
                // Parsear XML simple para obtener versiones
                let versions = parse_forge_versions_from_xml(&xml_text, &minecraft_version)?;
                
                if versions.is_empty() {
                    log::warn!("⚠️  No se encontraron versiones de Forge para Minecraft {}", minecraft_version);
                }
                
                Ok(versions)
            } else {
                Err(format!("Error HTTP al obtener versiones de Forge: {}", response.status()))
            }
        }
        Err(e) => {
            log::error!("❌ Error al obtener versiones de Forge: {}", e);
            Err(format!("Error de red: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_recommended_forge_version(minecraft_version: String) -> Result<String, String> {
    log::info!("🔍 Obteniendo versión recomendada de Forge para Minecraft {}", minecraft_version);
    
    let versions = get_forge_versions(minecraft_version.clone()).await?;
    
    // Buscar la primera versión recomendada
    if let Some(recommended) = versions.iter().find(|v| v.recommended) {
        return Ok(recommended.version.clone());
    }
    
    // Si no hay recomendada, devolver la última
    if let Some(latest) = versions.first() {
        return Ok(latest.version.clone());
    }
    
    Err(format!("No se encontró ninguna versión de Forge para Minecraft {}", minecraft_version))
}

fn parse_forge_versions_from_xml(xml: &str, mc_version: &str) -> Result<Vec<ForgeVersion>, String> {
    let mut versions = Vec::new();
    
    // Buscar todas las versiones que coincidan con la versión de MC
    for line in xml.lines() {
        if line.contains("<version>") {
            if let Some(version_str) = extract_xml_tag_content(line, "version") {
                // Las versiones de Forge siguen el formato: {mc_version}-{forge_version}
                // Ej: 1.20.1-47.2.0
                if version_str.starts_with(mc_version) {
                    versions.push(ForgeVersion {
                        version: version_str.clone(),
                        minecraft_version: mc_version.to_string(),
                        recommended: false, // Por ahora, marcaremos la primera como recomendada después
                    });
                }
            }
        }
    }
    
    // Marcar la última versión como recomendada
    if let Some(first) = versions.first_mut() {
        first.recommended = true;
    }
    
    Ok(versions)
}

fn extract_xml_tag_content(line: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);
    
    if let Some(start_idx) = line.find(&start_tag) {
        if let Some(end_idx) = line.find(&end_tag) {
            let content_start = start_idx + start_tag.len();
            if content_start < end_idx {
                return Some(line[content_start..end_idx].trim().to_string());
            }
        }
    }
    
    None
}

// ============================================================================
// NeoForge API Commands
// ============================================================================

#[tauri::command]
pub async fn get_neoforge_versions(minecraft_version: String) -> Result<Vec<NeoForgeVersion>, String> {
    log::info!("🔍 Obteniendo versiones de NeoForge para Minecraft {}", minecraft_version);
    
    // NeoForge solo está disponible para Minecraft 1.20.1+
    let version_parts: Vec<&str> = minecraft_version.split('.').collect();
    if version_parts.len() >= 2 {
        let minor = version_parts.get(1).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
        if minor < 20 {
            return Err("NeoForge solo está disponible para Minecraft 1.20.1 o superior".to_string());
        }
    }
    
    let client = reqwest::Client::new();
    
    // Usar el maven-metadata.xml de NeoForge
    let url = "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";
    
    match client.get(url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let xml_text = response.text().await.map_err(|e| e.to_string())?;
                
                let versions = parse_neoforge_versions_from_xml(&xml_text, &minecraft_version)?;
                
                if versions.is_empty() {
                    log::warn!("⚠️  No se encontraron versiones de NeoForge para Minecraft {}", minecraft_version);
                }
                
                Ok(versions)
            } else {
                Err(format!("Error HTTP al obtener versiones de NeoForge: {}", response.status()))
            }
        }
        Err(e) => {
            log::error!("❌ Error al obtener versiones de NeoForge: {}", e);
            Err(format!("Error de red: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_recommended_neoforge_version(minecraft_version: String) -> Result<String, String> {
    log::info!("🔍 Obteniendo versión recomendada de NeoForge para Minecraft {}", minecraft_version);
    
    let versions = get_neoforge_versions(minecraft_version.clone()).await?;
    
    // Devolver la primera (más reciente)
    if let Some(latest) = versions.first() {
        return Ok(latest.version.clone());
    }
    
    Err(format!("No se encontró ninguna versión de NeoForge para Minecraft {}", minecraft_version))
}

fn parse_neoforge_versions_from_xml(xml: &str, mc_version: &str) -> Result<Vec<NeoForgeVersion>, String> {
    let mut versions = Vec::new();
    
    // NeoForge usa formato específico: 20.x.y para MC 1.20.1, 21.0.x para MC 1.21, 21.1.x para MC 1.21.1, etc.
    // CRITICAL: Mapeo exacto de versiones NeoForge a Minecraft
    // https://neoforged.net/ - Verificar este mapeo regularmente
    let mc_parts: Vec<&str> = mc_version.split('.').collect();
    let mc_minor = mc_parts.get(1).and_then(|v| v.parse::<u32>().ok()).unwrap_or(20);
    let mc_patch = mc_parts.get(2).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    
    for line in xml.lines() {
        if line.contains("<version>") {
            if let Some(version_str) = extract_xml_tag_content(line, "version") {
                // Parse NeoForge version format: major.minor.patch
                let version_parts: Vec<&str> = version_str.split('.').collect();
                if version_parts.len() >= 2 {
                    if let (Ok(nf_major), Ok(nf_minor)) = (
                        version_parts[0].parse::<u32>(),
                        version_parts[1].parse::<u32>()
                    ) {
                        // Mapeo exacto de NeoForge a Minecraft:
                        // NeoForge 20.x.y → MC 1.20.1
                        // NeoForge 21.0.x → MC 1.21
                        // NeoForge 21.1.x → MC 1.21.1
                        // NeoForge 21.2.x → MC 1.21.2
                        // NeoForge 21.3.x → MC 1.21.3
                        // NeoForge 21.4.x → MC 1.21.4
                        // etc.
                        
                        let matches = if nf_major == 20 && mc_minor == 20 && mc_patch == 1 {
                            // NeoForge 20.x es solo para MC 1.20.1
                            true
                        } else if nf_major == 21 {
                            // NeoForge 21.x.y mapea a MC 1.21.x donde x = nf_minor
                            mc_minor == 21 && mc_patch == nf_minor
                        } else {
                            // Para versiones futuras, verificar que major coincida con minor de MC
                            nf_major == mc_minor && mc_patch == nf_minor
                        };
                        
                        if matches {
                            versions.push(NeoForgeVersion {
                                version: version_str.clone(),
                                minecraft_version: mc_version.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    
    Ok(versions)
}

// ==================== FRONTEND LOGGING ====================

/// Obtiene la ruta del archivo de logs del frontend
fn get_frontend_log_path() -> Result<std::path::PathBuf, String> {
    let base = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(|p| std::path::PathBuf::from(p))
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    
    let log_dir = base.join(".kindlyklanklient").join("logs");
    std::fs::create_dir_all(&log_dir)
        .map_err(|e| format!("Failed to create log directory: {}", e))?;
    
    Ok(log_dir.join("frontend.log"))
}

#[tauri::command]
pub async fn log_frontend_error(level: String, message: String, context: Option<String>) -> Result<(), String> {
    let log_path = get_frontend_log_path()?;
    
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let context_str = context.map(|c| format!(" [{}]", c)).unwrap_or_default();
    let log_line = format!("[{}] {}{}: {}\n", timestamp, level.to_uppercase(), context_str, message);
    
    // Escribir al archivo
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("Failed to open log file: {}", e))?;
    
    file.write_all(log_line.as_bytes())
        .map_err(|e| format!("Failed to write to log file: {}", e))?;
    
    // También loggear en el sistema de logs de Rust
    match level.to_lowercase().as_str() {
        "error" => log::error!("[Frontend]{} {}", context_str, message),
        "warn" => log::warn!("[Frontend]{} {}", context_str, message),
        "info" => log::info!("[Frontend]{} {}", context_str, message),
        _ => log::debug!("[Frontend]{} {}", context_str, message),
    }
    
    Ok(())
}

#[tauri::command]
pub async fn get_frontend_logs() -> Result<String, String> {
    let log_path = get_frontend_log_path()?;
    
    if !log_path.exists() {
        return Ok(String::from("No logs available yet."));
    }
    
    std::fs::read_to_string(&log_path)
        .map_err(|e| format!("Failed to read log file: {}", e))
}

#[tauri::command]
pub async fn clear_frontend_logs() -> Result<(), String> {
    let log_path = get_frontend_log_path()?;
    
    if log_path.exists() {
        std::fs::remove_file(&log_path)
            .map_err(|e| format!("Failed to clear log file: {}", e))?;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn open_frontend_log_folder() -> Result<(), String> {
    let log_path = get_frontend_log_path()?;
    let log_dir = log_path.parent()
        .ok_or_else(|| "Failed to get log directory".to_string())?;
    
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(log_dir)
            .spawn()
            .map_err(|e| format!("Failed to open log folder: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(log_dir)
            .spawn()
            .map_err(|e| format!("Failed to open log folder: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(log_dir)
            .spawn()
            .map_err(|e| format!("Failed to open log folder: {}", e))?;
    }
    
    Ok(())
}

