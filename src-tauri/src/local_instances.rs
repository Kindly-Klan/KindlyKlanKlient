use crate::models::{LocalInstance, LocalInstanceMetadata};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use serde_json;

// Generate a slugified ID from name with random suffix
fn generate_instance_id(name: &str) -> String {
    use rand::Rng;
    
    // Slugify the name
    let slug = name
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c.is_whitespace() || c == '-' || c == '_' {
                '-'
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("-");
    
    // Generate random suffix (5 chars alphanumeric)
    let mut rng = rand::thread_rng();
    let suffix: String = (0..5)
        .map(|_| {
            let idx = rng.gen_range(0..36);
            if idx < 10 {
                (b'0' + idx) as char
            } else {
                (b'a' + (idx - 10)) as char
            }
        })
        .collect();
    
    format!("{}-{}", slug, suffix)
}

// Get the local instances directory
fn get_local_instances_dir() -> Result<PathBuf, String> {
    let base = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(|p| PathBuf::from(p))
        .unwrap_or_else(|_| PathBuf::from("."));
    
    Ok(base.join(".kindlyklanklient").join("local_instances"))
}

#[tauri::command]
pub async fn create_local_instance(
    name: String,
    minecraft_version: String,
    fabric_version: String,
    app_handle: AppHandle,
) -> Result<LocalInstance, String> {
    log::info!("🏗️  Creating local instance: {} (MC: {}, Fabric: {})", name, minecraft_version, fabric_version);
    
    let instance_id = generate_instance_id(&name);
    log::info!("📝 Generated instance ID: {}", instance_id);
    
    let local_instances_dir = get_local_instances_dir()?;
    let instance_dir = local_instances_dir.join(&instance_id);
    
    // Create instance directory
    tokio::fs::create_dir_all(&instance_dir)
        .await
        .map_err(|e| format!("Failed to create instance directory: {}", e))?;
    
    log::info!("📁 Instance directory created: {}", instance_dir.display());
    
    // Emit progress: Starting
    let _ = app_handle.emit("local-instance-progress", serde_json::json!({
        "instance_id": instance_id,
        "stage": "starting",
        "percentage": 0,
        "message": "Iniciando creación de instancia..."
    }));
    
    // Download Minecraft client
    let _ = app_handle.emit("local-instance-progress", serde_json::json!({
        "instance_id": instance_id,
        "stage": "minecraft_client",
        "percentage": 10,
        "message": "Descargando cliente de Minecraft..."
    }));
    
    crate::instances::ensure_minecraft_client_present(&instance_dir, &minecraft_version).await?;
    
    log::info!("✅ Minecraft client downloaded");
    
    // Download Minecraft libraries
    let _ = app_handle.emit("local-instance-progress", serde_json::json!({
        "instance_id": instance_id,
        "stage": "minecraft_libraries",
        "percentage": 30,
        "message": "Descargando librerías de Minecraft..."
    }));
    
    crate::instances::ensure_version_libraries(&instance_dir, &minecraft_version).await?;
    
    log::info!("✅ Minecraft libraries downloaded");
    
    // Install Fabric Loader
    let _ = app_handle.emit("local-instance-progress", serde_json::json!({
        "instance_id": instance_id,
        "stage": "fabric_loader",
        "percentage": 50,
        "message": "Instalando Fabric Loader..."
    }));
    
    let mod_loader = crate::models::ModLoader {
        r#type: "fabric".to_string(),
        version: fabric_version.clone(),
    };
    
    crate::instances::install_mod_loader(&minecraft_version, &mod_loader, &instance_dir).await?;
    
    log::info!("✅ Fabric Loader installed");
    
    // Download Minecraft assets
    let _ = app_handle.emit("local-instance-progress", serde_json::json!({
        "instance_id": instance_id,
        "stage": "minecraft_assets",
        "percentage": 70,
        "message": "Descargando assets de Minecraft..."
    }));
    
    crate::instances::ensure_assets_present(&app_handle, &instance_dir, &minecraft_version).await?;
    
    log::info!("✅ Minecraft assets downloaded");
    
    // Create mods directory
    tokio::fs::create_dir_all(instance_dir.join("mods"))
        .await
        .map_err(|e| format!("Failed to create mods directory: {}", e))?;
    
    // Save instance metadata
    let _ = app_handle.emit("local-instance-progress", serde_json::json!({
        "instance_id": instance_id,
        "stage": "saving_metadata",
        "percentage": 90,
        "message": "Guardando metadata..."
    }));
    
    let metadata = LocalInstanceMetadata {
        id: instance_id.clone(),
        name: name.clone(),
        minecraft_version: minecraft_version.clone(),
        fabric_version: fabric_version.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    
    let metadata_path = instance_dir.join("instance_local.json");
    let metadata_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
    
    tokio::fs::write(&metadata_path, metadata_json)
        .await
        .map_err(|e| format!("Failed to write metadata: {}", e))?;
    
    log::info!("✅ Metadata saved");
    
    // Emit completion
    let _ = app_handle.emit("local-instance-progress", serde_json::json!({
        "instance_id": instance_id,
        "stage": "completed",
        "percentage": 100,
        "message": "¡Instancia creada exitosamente!"
    }));
    
    let local_instance = LocalInstance {
        id: instance_id.clone(),
        name: name.clone(),
        minecraft_version: minecraft_version.clone(),
        fabric_version: fabric_version.clone(),
        created_at: metadata.created_at.clone(),
        is_local: true,
        background: None,
    };
    
    log::info!("🎉 Local instance created successfully: {}", instance_id);
    
    Ok(local_instance)
}

#[tauri::command]
pub async fn get_local_instances() -> Result<Vec<LocalInstance>, String> {
    log::info!("📋 Listing local instances");
    
    let local_instances_dir = get_local_instances_dir()?;
    
    if !local_instances_dir.exists() {
        log::info!("📂 Local instances directory does not exist, returning empty list");
        return Ok(Vec::new());
    }
    
    let mut instances = Vec::new();
    
    let mut entries = tokio::fs::read_dir(&local_instances_dir)
        .await
        .map_err(|e| format!("Failed to read local instances directory: {}", e))?;
    
    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        let path = entry.path();
        
        if path.is_dir() {
            let metadata_path = path.join("instance_local.json");
            
            if metadata_path.exists() {
                match tokio::fs::read_to_string(&metadata_path).await {
                    Ok(content) => {
                        match serde_json::from_str::<LocalInstanceMetadata>(&content) {
                            Ok(metadata) => {
                                // Check if background image exists
                                let background_path = path.join("background.png");
                                let background = if background_path.exists() {
                                    Some(background_path.to_string_lossy().to_string())
                                } else {
                                    None
                                };
                                
                                instances.push(LocalInstance {
                                    id: metadata.id,
                                    name: metadata.name,
                                    minecraft_version: metadata.minecraft_version,
                                    fabric_version: metadata.fabric_version,
                                    created_at: metadata.created_at,
                                    is_local: true,
                                    background,
                                });
                            }
                            Err(e) => {
                                log::warn!("⚠️  Failed to parse metadata for {}: {}", path.display(), e);
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("⚠️  Failed to read metadata for {}: {}", path.display(), e);
                    }
                }
            }
        }
    }
    
    log::info!("✅ Found {} local instances", instances.len());
    Ok(instances)
}

#[tauri::command]
pub async fn sync_mods_from_remote(
    local_instance_id: String,
    remote_instance_id: String,
    distribution_url: String,
    app_handle: AppHandle,
) -> Result<String, String> {
    log::info!("🔄 Syncing mods from remote {} to local {}", remote_instance_id, local_instance_id);
    
    let _ = app_handle.emit("mod-sync-progress", serde_json::json!({
        "local_id": local_instance_id,
        "remote_id": remote_instance_id,
        "stage": "loading_remote",
        "percentage": 10,
        "message": "Cargando instancia remota..."
    }));
    
    // Load remote instance manifest
    let base_url = crate::instances::build_distribution_url(&distribution_url);
    let manifest = crate::instances::load_instance_manifest(&base_url, &remote_instance_id).await?;
    
    log::info!("📦 Remote instance loaded: {} mods", manifest.files.mods.len());
    
    let _ = app_handle.emit("mod-sync-progress", serde_json::json!({
        "local_id": local_instance_id,
        "remote_id": remote_instance_id,
        "stage": "clearing_mods",
        "percentage": 20,
        "message": "Limpiando carpeta de mods..."
    }));
    
    // Get local instance directory
    let local_instances_dir = get_local_instances_dir()?;
    let instance_dir = local_instances_dir.join(&local_instance_id);
    let mods_dir = instance_dir.join("mods");
    
    // Clear mods directory
    if mods_dir.exists() {
        tokio::fs::remove_dir_all(&mods_dir)
            .await
            .map_err(|e| format!("Failed to remove mods directory: {}", e))?;
    }
    
    tokio::fs::create_dir_all(&mods_dir)
        .await
        .map_err(|e| format!("Failed to create mods directory: {}", e))?;
    
    log::info!("🗑️  Mods directory cleared");
    
    let _ = app_handle.emit("mod-sync-progress", serde_json::json!({
        "local_id": local_instance_id,
        "remote_id": remote_instance_id,
        "stage": "downloading_mods",
        "percentage": 30,
        "message": format!("Descargando {} mods...", manifest.files.mods.len())
    }));
    
    // Download all mods from remote instance
    let total_mods = manifest.files.mods.len();
    for (index, mod_file) in manifest.files.mods.iter().enumerate() {
        let progress = 30 + ((index as f32 / total_mods as f32) * 60.0) as u32;
        
        let _ = app_handle.emit("mod-sync-progress", serde_json::json!({
            "local_id": local_instance_id,
            "remote_id": remote_instance_id,
            "stage": "downloading_mods",
            "percentage": progress,
            "message": format!("Descargando {} ({}/{})", mod_file.name, index + 1, total_mods)
        }));
        
        let asset = crate::instances::create_asset_from_file_entry(mod_file, &remote_instance_id, &base_url);
        let target_path = mods_dir.join(&mod_file.name);
        
        crate::instances::download_file_with_retry(&asset.url, &target_path).await?;
        
        log::info!("✅ Downloaded mod: {}", mod_file.name);
    }
    
    let _ = app_handle.emit("mod-sync-progress", serde_json::json!({
        "local_id": local_instance_id,
        "remote_id": remote_instance_id,
        "stage": "completed",
        "percentage": 100,
        "message": "¡Mods sincronizados exitosamente!"
    }));
    
    log::info!("🎉 Mods synced successfully: {} mods", total_mods);
    
    Ok(format!("Successfully synced {} mods", total_mods))
}

#[tauri::command]
pub async fn open_instance_folder(instance_id: String) -> Result<(), String> {
    log::info!("📂 Opening folder for instance: {}", instance_id);
    
    let local_instances_dir = get_local_instances_dir()?;
    let instance_dir = local_instances_dir.join(&instance_id);
    
    if !instance_dir.exists() {
        return Err(format!("Instance directory does not exist: {}", instance_dir.display()));
    }
    
    // Open folder using shell plugin
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(instance_dir.to_string_lossy().to_string())
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(instance_dir.to_string_lossy().to_string())
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(instance_dir.to_string_lossy().to_string())
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    log::info!("✅ Folder opened successfully");
    Ok(())
}

#[tauri::command]
pub async fn launch_local_instance(
    instance_id: String,
    access_token: String,
    username: String,
    uuid: String,
    _min_ram_gb: f64,
    max_ram_gb: f64,
    app_handle: AppHandle,
) -> Result<String, String> {
    log::info!("🚀 Launching local instance: {}", instance_id);
    
    let local_instances_dir = get_local_instances_dir()?;
    let instance_dir = local_instances_dir.join(&instance_id);
    
    if !instance_dir.exists() {
        return Err(format!("Instance directory does not exist: {}", instance_dir.display()));
    }
    
    // Load instance metadata
    let metadata_path = instance_dir.join("instance_local.json");
    let metadata_content = tokio::fs::read_to_string(&metadata_path)
        .await
        .map_err(|e| format!("Failed to read instance metadata: {}", e))?;
    
    let metadata: LocalInstanceMetadata = serde_json::from_str(&metadata_content)
        .map_err(|e| format!("Failed to parse instance metadata: {}", e))?;
    
    log::info!("📋 Instance metadata loaded: MC {}, Fabric {}", metadata.minecraft_version, metadata.fabric_version);
    
    // Verify all required files (emit progress events)
    let _ = app_handle.emit("asset-download-progress", serde_json::json!({
        "current": 0,
        "total": 100,
        "percentage": 0,
        "current_file": "",
        "status": "Verificando archivos..."
    }));
    
    // Ensure Minecraft client is present
    crate::instances::ensure_minecraft_client_present(&instance_dir, &metadata.minecraft_version).await?;
    
    let _ = app_handle.emit("asset-download-progress", serde_json::json!({
        "current": 33,
        "total": 100,
        "percentage": 33,
        "current_file": "",
        "status": "Verificando librerías..."
    }));
    
    // Ensure libraries are present
    crate::instances::ensure_version_libraries(&instance_dir, &metadata.minecraft_version).await?;
    
    let _ = app_handle.emit("asset-download-progress", serde_json::json!({
        "current": 66,
        "total": 100,
        "percentage": 66,
        "current_file": "",
        "status": "Verificando assets..."
    }));
    
    // Ensure assets are present
    crate::instances::ensure_assets_present(&app_handle, &instance_dir, &metadata.minecraft_version).await?;
    
    let _ = app_handle.emit("asset-download-progress", serde_json::json!({
        "current": 100,
        "total": 100,
        "percentage": 100,
        "current_file": "",
        "status": "Completado"
    }));
    
    let _ = app_handle.emit("asset-download-completed", serde_json::json!({ "phase": "complete" }));
    
    log::info!("✅ All files verified, launching Minecraft...");
    
    // Use the MinecraftLauncher to launch the game
    let launcher = crate::launcher::MinecraftLauncher::new()
        .map_err(|e| format!("Failed to create launcher: {}", e))?;
    
    let ram_mb = (max_ram_gb * 1024.0) as u32;
    
    launcher.launch_minecraft(
        &metadata.minecraft_version,
        &username,
        ram_mb,
        Some(&access_token),
        Some(&uuid),
    ).await.map_err(|e| format!("Failed to launch Minecraft: {}", e))?;
    
    log::info!("🎮 Minecraft launched successfully");
    
    Ok(format!("Local instance {} launched successfully", instance_id))
}

