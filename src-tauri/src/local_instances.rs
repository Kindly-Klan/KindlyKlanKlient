use crate::models::{LocalInstance, LocalInstanceMetadata};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
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

// Get instance directory (works for both local and remote instances)
pub fn get_instance_directory_smart(instance_id: &str) -> PathBuf {
    let base = std::env::var("USERPROFILE")
        .map(|p| std::path::Path::new(&p).join(".kindlyklanklient"))
        .unwrap_or_else(|_| std::path::Path::new(".").join(".kindlyklanklient"));
    
    // Check if it's a local instance
    let local_instances_dir = base.join("local_instances");
    let local_instance_dir = local_instances_dir.join(instance_id);
    
    if local_instance_dir.exists() {
        local_instance_dir
    } else {
        // Fallback to remote instance directory
        base.join(instance_id)
    }
}

#[tauri::command]
pub async fn create_local_instance(
    name: String,
    minecraft_version: String,
    mod_loader_type: String,
    mod_loader_version: String,
    app_handle: AppHandle,
) -> Result<LocalInstance, String> {
    log::info!("🏗️  Creating local instance: {} (MC: {}, Loader: {} {})", name, minecraft_version, mod_loader_type, mod_loader_version);
    
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
    
    // Ensure Java is installed BEFORE installing mod loader
    let _ = app_handle.emit("local-instance-progress", serde_json::json!({
        "instance_id": instance_id,
        "stage": "java_check",
        "percentage": 40,
        "message": "Verificando Java..."
    }));
    
    log::info!("🔍 Verificando Java para Minecraft {}", minecraft_version);
    match crate::launcher::find_or_install_java_for_minecraft(&minecraft_version).await {
        Ok(java_path) => {
            log::info!("✅ Java disponible en: {}", java_path);
        }
        Err(e) => {
            log::error!("❌ Error al instalar Java: {}", e);
            return Err(format!("Error al instalar Java: {}", e));
        }
    }
    
    // Install Mod Loader (if not vanilla)
    let version_id = if mod_loader_type != "vanilla" {
        let loader_display_name = match mod_loader_type.as_str() {
            "fabric" => "Fabric",
            "forge" => "Forge",
            "neoforge" => "NeoForge",
            _ => "Mod Loader",
        };
        
        let _ = app_handle.emit("local-instance-progress", serde_json::json!({
            "instance_id": instance_id,
            "stage": "mod_loader",
            "percentage": 50,
            "message": format!("Instalando {} {}...", loader_display_name, mod_loader_version)
        }));
        
        let mod_loader = crate::models::ModLoader {
            r#type: mod_loader_type.clone(),
            version: mod_loader_version.clone(),
        };
        
        // Instalar mod loader y obtener el version_id creado
        let vid = crate::instances::install_mod_loader(&minecraft_version, &mod_loader, &instance_dir).await?;
        
        log::info!("✅ {} {} installed", loader_display_name, mod_loader_version);
        if let Some(ref v) = vid {
            log::info!("📋 Version ID creado: {}", v);
        }
        vid
    } else {
        log::info!("✅ Vanilla instance, skipping mod loader installation");
        None
    };
    
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
    
    let mod_loader_obj = if mod_loader_type != "vanilla" {
        Some(crate::models::ModLoader {
            r#type: mod_loader_type.clone(),
            version: mod_loader_version.clone(),
        })
    } else {
        None
    };
    
    let metadata = LocalInstanceMetadata {
        id: instance_id.clone(),
        name: name.clone(),
        minecraft_version: minecraft_version.clone(),
        fabric_version: mod_loader_version.clone(), // Mantener compatibilidad retroactiva
        mod_loader: mod_loader_obj.clone(),
        version_id: version_id.clone(),
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
        fabric_version: mod_loader_version.clone(), // Mantener compatibilidad retroactiva
        mod_loader: mod_loader_obj,
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
                                    mod_loader: metadata.mod_loader,
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
    
    log::info!("📦 Remote instance loaded: {} mods, {} configs", manifest.files.mods.len(), manifest.files.configs.len());
    
    // Get local instance directory
    let local_instances_dir = get_local_instances_dir()?;
    let instance_dir = local_instances_dir.join(&local_instance_id);
    let mods_dir = instance_dir.join("mods");
    let config_dir = instance_dir.join("config");
    
    // Create directories if they don't exist (don't clear existing mods)
    tokio::fs::create_dir_all(&mods_dir)
        .await
        .map_err(|e| format!("Failed to create mods directory: {}", e))?;
    tokio::fs::create_dir_all(&config_dir)
        .await
        .map_err(|e| format!("Failed to create config directory: {}", e))?;
    
    log::info!("📁 Directories ready (preserving existing mods)");
    
    // Download mods from remote instance (only if they don't exist or are different)
    let total_mods = manifest.files.mods.len();
    let mut downloaded_mods = 0;
    let mut skipped_mods = 0;
    
    let _ = app_handle.emit("mod-sync-progress", serde_json::json!({
        "local_id": local_instance_id,
        "remote_id": remote_instance_id,
        "stage": "downloading_mods",
        "percentage": 20,
        "message": format!("Sincronizando {} mods...", total_mods)
    }));
    
    for (index, mod_file) in manifest.files.mods.iter().enumerate() {
        let progress = 20 + ((index as f32 / total_mods as f32) * 40.0) as u32;
        
        let _ = app_handle.emit("mod-sync-progress", serde_json::json!({
            "local_id": local_instance_id,
            "remote_id": remote_instance_id,
            "stage": "downloading_mods",
            "percentage": progress,
            "message": format!("Sincronizando {} ({}/{})", mod_file.name, index + 1, total_mods)
        }));
        
        let asset = crate::instances::create_asset_from_file_entry(mod_file, &remote_instance_id, &base_url);
        let target_path = mods_dir.join(&mod_file.name);
        
        // Only download if file doesn't exist or checksum differs
        let should_download = if target_path.exists() {
            if !mod_file.sha256.is_empty() {
                !crate::instances::verify_file_checksum(&target_path, &mod_file.sha256).is_ok()
            } else if let Some(md5) = &mod_file.md5 {
                !md5.is_empty() && !crate::instances::verify_file_md5(&target_path, md5).is_ok()
            } else {
                true // No checksum available, download to be safe
            }
        } else {
            true // File doesn't exist, download it
        };
        
        if should_download {
            crate::instances::download_file_with_retry(&asset.url, &target_path).await?;
            downloaded_mods += 1;
            log::info!("✅ Downloaded mod: {}", mod_file.name);
        } else {
            skipped_mods += 1;
            log::info!("⏭️  Skipped mod (already exists): {}", mod_file.name);
        }
    }
    
    // Download configs from remote instance
    let total_configs = manifest.files.configs.len();
    let mut downloaded_configs = 0;
    
    let _ = app_handle.emit("mod-sync-progress", serde_json::json!({
        "local_id": local_instance_id,
        "remote_id": remote_instance_id,
        "stage": "downloading_configs",
        "percentage": 60,
        "message": format!("Sincronizando {} configs...", total_configs)
    }));
    
    for (index, config_file) in manifest.files.configs.iter().enumerate() {
        let progress = 60 + ((index as f32 / total_configs as f32) * 35.0) as u32;
        
        let _ = app_handle.emit("mod-sync-progress", serde_json::json!({
            "local_id": local_instance_id,
            "remote_id": remote_instance_id,
            "stage": "downloading_configs",
            "percentage": progress,
            "message": format!("Sincronizando config {} ({}/{})", config_file.name, index + 1, total_configs)
        }));
        
        let asset = crate::instances::create_asset_from_file_entry(config_file, &remote_instance_id, &base_url);
        
        // Determine target path (respect target if specified, otherwise use name)
        let target_path = if let Some(target) = &config_file.target {
            config_dir.join(target)
        } else {
            config_dir.join(&config_file.name)
        };
        
        // Create parent directories if needed
        if let Some(parent) = target_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }
        
        crate::instances::download_file_with_retry(&asset.url, &target_path).await?;
        downloaded_configs += 1;
        log::info!("✅ Downloaded config: {}", config_file.name);
    }
    
    let _ = app_handle.emit("mod-sync-progress", serde_json::json!({
        "local_id": local_instance_id,
        "remote_id": remote_instance_id,
        "stage": "completed",
        "percentage": 100,
        "message": format!("¡Sincronización completada! {} mods, {} configs", downloaded_mods, downloaded_configs)
    }));
    
    log::info!("🎉 Sync completed: {} mods downloaded ({} skipped), {} configs downloaded", downloaded_mods, skipped_mods, downloaded_configs);
    
    Ok(format!("Successfully synced {} mods ({} skipped, {} new) and {} configs", downloaded_mods + skipped_mods, skipped_mods, downloaded_mods, downloaded_configs))
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
    min_ram_gb: f64,
    max_ram_gb: f64,
    app_handle: AppHandle,
) -> Result<String, String> {
    use std::process::Command;
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;
    
    log::info!("🚀 Launching local instance: {}", instance_id);
    
    // Validate and refresh token before launching
    log::info!("🔐 Validating and refreshing token for user: {}", username);
    let (validated_access_token, validated_uuid) = match crate::sessions_api::validate_and_refresh_token(app_handle.clone(), username.clone()).await {
        Ok(crate::EnsureSessionResponse::Ok { session, refreshed }) => {
            if refreshed {
                log::info!("✅ Token refreshed successfully");
            } else {
                log::info!("✅ Token is still valid");
            }
            // Use the validated/refreshed token and UUID from session
            (session.access_token, session.uuid)
        }
        Ok(crate::EnsureSessionResponse::Err { code, message }) => {
            log::warn!("⚠️  Token validation failed: {} - {}, using provided token", code, message);
            // Fallback to provided token if validation fails
            (access_token, uuid)
        }
        Err(e) => {
            log::warn!("⚠️  Token validation error: {}, using provided token", e);
            // Fallback to provided token if validation fails
            (access_token, uuid)
        }
    };
    
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
    
    let mut metadata: LocalInstanceMetadata = serde_json::from_str(&metadata_content)
        .map_err(|e| format!("Failed to parse instance metadata: {}", e))?;
    
    // Si no hay version_id (instancias antiguas), intentar detectarlo
    if metadata.version_id.is_none() && metadata.mod_loader.is_some() {
        if let Some(ref mod_loader) = metadata.mod_loader {
            let detected_version_id = crate::instances::find_version_id_in_versions_dir(
                &instance_dir, 
                &mod_loader.r#type
            );
            
            if let Some(ref vid) = detected_version_id {
                log::info!("🔍 Detectado version_id para instancia antigua: {}", vid);
                metadata.version_id = detected_version_id.clone();
                
                // Guardar el metadata actualizado
                if let Ok(updated_metadata_json) = serde_json::to_string_pretty(&metadata) {
                    let _ = tokio::fs::write(&metadata_path, updated_metadata_json).await;
                    log::info!("✅ Metadata actualizado con version_id");
                }
            }
        }
    }
    
    // Log metadata con información correcta del mod loader
    if let Some(ref mod_loader) = metadata.mod_loader {
        log::info!("📋 Instance metadata loaded: MC {}, {} {}", 
            metadata.minecraft_version, 
            mod_loader.r#type.to_uppercase(), 
            mod_loader.version
        );
        if let Some(ref vid) = metadata.version_id {
            log::info!("📋 Version ID: {}", vid);
        }
    } else {
        log::info!("📋 Instance metadata loaded: MC {}, Vanilla (legacy: {})", 
            metadata.minecraft_version, 
            metadata.fabric_version
        );
    }
    
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
    
    // Ensure libraries are present (vanilla MC)
    crate::instances::ensure_version_libraries(&instance_dir, &metadata.minecraft_version).await?;
    
    // Ensure mod loader libraries are present (Fabric/NeoForge/Forge specific libraries)
    if let Some(version_id) = &metadata.version_id {
        crate::instances::ensure_mod_loader_libraries(&instance_dir, version_id).await?;
    }
    
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
    
    // Create mods directory if it doesn't exist
    let _ = tokio::fs::create_dir_all(instance_dir.join("mods")).await;
    
    // Get mod loader JVM args usando el version_id del metadata
    let mod_loader_jvm_args = crate::launcher::get_mod_loader_jvm_args(
        &instance_dir,
        metadata.version_id.as_deref(),
        metadata.mod_loader.as_ref().map(|ml| ml.r#type.as_str()),
        metadata.mod_loader.as_ref().map(|ml| ml.version.as_str()),
    );
    
    // Build classpath FROM JSON, respecting include_in_classpath field (como Modrinth)
    let version_json_path = instance_dir
        .join("versions")
        .join(metadata.version_id.as_ref().unwrap_or(&metadata.minecraft_version))
        .join(format!("{}.json", metadata.version_id.as_ref().unwrap_or(&metadata.minecraft_version)));
    
    let classpath = crate::launcher::build_minecraft_classpath_from_json(&instance_dir, &version_json_path)?;
    
    // Check for lwjgl
    {
        let mut has_lwjgl = false;
        if let Ok(entries) = std::fs::read_dir(instance_dir.join("libraries")) {
            for entry in entries.flatten() {
                if entry.path().to_string_lossy().contains("lwjgl") {
                    has_lwjgl = true;
                    break;
                }
            }
        }
        if !has_lwjgl {
            crate::instances::ensure_minecraft_client_present(&instance_dir, &metadata.minecraft_version).await?;
        }
    }
    
    // Load advanced config
    let (jvm_args_config, gc_config, window_width, window_height) = crate::commands::load_advanced_config()
        .await
        .unwrap_or((String::new(), "G1".to_string(), 1280, 720));
    
    // Build JVM args using validated token
    let mut jvm_args = crate::launcher::build_minecraft_jvm_args(
        &validated_access_token,
        min_ram_gb,
        max_ram_gb,
        &gc_config,
        &jvm_args_config,
    )?;
    
    // Add mod loader specific JVM args (Forge/NeoForge/Fabric)
    if !mod_loader_jvm_args.is_empty() {
        jvm_args.extend(mod_loader_jvm_args);
    }
    
    let asset_index_id = crate::instances::ensure_assets_present(&app_handle, &instance_dir, &metadata.minecraft_version).await?;
    let user_properties = "{}".to_string();
    
    let assets_dir = instance_dir.join("assets");
    
    let uuid_simple = validated_uuid.replace("-", "");
    
    let mut mc_args = vec![
        "--username".to_string(), username,
        "--uuid".to_string(), uuid_simple,
        "--accessToken".to_string(), validated_access_token,
        "--version".to_string(), metadata.minecraft_version.clone(),
        "--gameDir".to_string(), instance_dir.to_string_lossy().to_string(),
        "--assetsDir".to_string(), assets_dir.to_string_lossy().to_string(),
        "--assetIndex".to_string(), asset_index_id,
        "--userType".to_string(), "msa".to_string(),
        "--userProperties".to_string(), user_properties.clone(),
        "--versionType".to_string(), "release".to_string(),
        "--width".to_string(), window_width.to_string(),
        "--height".to_string(), window_height.to_string(),
    ];
    
    let mod_loader_game_args = crate::launcher::get_mod_loader_game_args(&instance_dir, metadata.version_id.as_deref());
    if !mod_loader_game_args.is_empty() {
        mc_args.extend(mod_loader_game_args);
    }
    
    let main_class = crate::launcher::select_main_class(&instance_dir, metadata.version_id.as_deref());
    let java_path = crate::launcher::find_or_install_java_for_minecraft(&metadata.minecraft_version).await?;
    
    let mut command = Command::new(&java_path);
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    
    log::info!("📋 userProperties being passed: {}", user_properties);
    log::info!("📋 userProperties length: {} chars", user_properties.len());
    
    command.args(&jvm_args);
    command.arg("-cp").arg(&classpath);
    command.arg(&main_class).args(&mc_args);
    command.current_dir(&instance_dir);
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    
    let mut child = command.spawn()
        .map_err(|e| format!("Failed to start Minecraft: {}", e))?;
    
    let pid = child.id();
    
    if let Some(state) = app_handle.try_state::<std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, u32>>>>() {
        if let Ok(mut processes) = state.lock() {
            log::info!("💾 Guardando proceso local para instancia: {} con PID: {}", instance_id, pid);
            processes.insert(instance_id.clone(), pid);
            log::info!("📋 Procesos activos: {:?}", processes.keys().collect::<Vec<_>>());
        }
    } else {
        log::warn!("⚠️ No se pudo obtener el estado de procesos");
    }
    
    if let Some(stdout) = child.stdout.take() {
        use std::io::{BufRead, BufReader};
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                if line.contains("ERROR") || line.contains("FATAL") || line.contains("Exception") {
                    log::error!("[MC] {}", line);
                }
            }
        });
    }
    
    if let Some(stderr) = child.stderr.take() {
        use std::io::{BufRead, BufReader};
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().flatten() {
                log::error!("[MC] {}", line);
            }
        });
    }
    
    let app = app_handle.clone();
    let instance_id_clone = instance_id.clone();
    let processes_state = if let Some(state) = app_handle.try_state::<std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, u32>>>>() {
        state.inner().clone()
    } else {
        return Err("Failed to get processes state".to_string());
    };
    std::thread::spawn(move || {
        match child.wait() {
            Ok(status) => {
                log::info!("🎮 Minecraft exited with status: {:?}", status);
                if let Ok(mut processes) = processes_state.lock() {
                    processes.remove(&instance_id_clone);
                }
                let _ = app.emit("minecraft_exited", serde_json::json!({ 
                    "instance_id": instance_id_clone,
                    "status": "exited",
                    "code": status.code()
                }));
            }
            Err(e) => {
                log::error!("❌ Error waiting for Minecraft: {}", e);
                if let Ok(mut processes) = processes_state.lock() {
                    processes.remove(&instance_id_clone);
                }
                let _ = app.emit("minecraft_exited", serde_json::json!({ 
                    "instance_id": instance_id_clone,
                    "status": "error",
                    "error": e.to_string()
                }));
            }
        }
    });
    
    log::info!("🎮 Minecraft process spawned successfully");
    
    Ok(format!("Local instance {} launched successfully", instance_id))
}

#[tauri::command]
pub async fn delete_local_instance(instance_id: String) -> Result<String, String> {
    log::info!("🗑️  Deleting local instance: {}", instance_id);
    
    let local_instances_dir = get_local_instances_dir()?;
    let instance_dir = local_instances_dir.join(&instance_id);
    
    if !instance_dir.exists() {
        return Err(format!("Instance directory does not exist: {}", instance_dir.display()));
    }
    
    tokio::fs::remove_dir_all(&instance_dir)
        .await
        .map_err(|e| format!("Failed to delete instance directory: {}", e))?;
    
    log::info!("✅ Local instance deleted successfully: {}", instance_id);
    
    Ok(format!("Local instance {} deleted successfully", instance_id))
}

