use crate::models::InstanceManifest;
use std::collections::HashMap;
use crate::models::{FileEntry, InstanceAsset, ModLoader};
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
 
 
use tauri::Emitter;

#[tauri::command]
pub async fn test_manifest_url(
    distribution_url: String,
    instance_id: String
) -> Result<String, String> {
    let base_url = crate::build_distribution_url(&distribution_url);
    let instance_url = format!("{}/instances/{}/instance.json", base_url, instance_id);

    match reqwest::get(&instance_url).await {
        Ok(response) => {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| "Failed to read response".to_string());

            if status.is_success() {
                Ok(format!("✅ Success ({}): {} bytes\nPreview: {}", status, text.len(), &text[..std::cmp::min(200, text.len())]))
            } else {
                Ok(format!("❌ HTTP Error ({}): {}", status, text))
            }
        }
        Err(e) => {
            Err(format!("❌ Network Error: {}", e))
        }
    }
}

pub async fn load_instance_manifest(distribution_url: &str, instance_id: &str) -> Result<InstanceManifest, String> {
    let base_url = crate::build_distribution_url(distribution_url);
    let instance_url = format!("{}/instances/{}/instance.json", base_url, instance_id);

    let response = reqwest::get(&instance_url)
        .await
        .map_err(|e| format!("Failed to fetch instance manifest: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("HTTP error {}: {}", status, error_text));
    }

    let text = response.text().await
        .map_err(|e| format!("Failed to read response text: {}", e))?;

    if text.trim().is_empty() {
        return Err("Empty response from server".to_string());
    }

    let manifest: InstanceManifest = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse instance manifest JSON: {}", e))?;

    Ok(manifest)
}

pub async fn load_checksums(distribution_url: &str, instance_id: &str) -> Result<HashMap<String, String>, String> {
    let base_url = crate::build_distribution_url(distribution_url);
    let checksums_url = format!("{}/instances/{}/checksums.json", base_url, instance_id);

    let response = reqwest::get(&checksums_url)
        .await
        .map_err(|e| format!("Failed to fetch checksums: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("HTTP error {}: {}", status, error_text));
    }

    let text = response.text().await
        .map_err(|e| format!("Failed to read checksums response text: {}", e))?;

    if text.trim().is_empty() {
        return Err("Empty checksums response from server".to_string());
    }

    let checksums: HashMap<String, String> = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse checksums JSON: {}", e))?;

    Ok(checksums)
}

pub fn create_asset_from_file_entry(file_entry: &FileEntry, instance_id: &str, distribution_url: &str) -> InstanceAsset {
    let base_url = crate::build_distribution_url(distribution_url);

    let resolved_url = if !file_entry.url.is_empty() {
        if file_entry.url.starts_with("http://") || file_entry.url.starts_with("https://") {
            file_entry.url.clone()
        } else {
            format!("{}/{}", base_url.trim_end_matches('/'), file_entry.url.trim_start_matches('/'))
        }
    } else {
        let server_relative = file_entry
            .path
            .trim_start_matches('/')
            .trim_start_matches("files/");
        format!(
            "{}/instances/{}/{}",
            base_url.trim_end_matches('/'),
            instance_id,
            server_relative
        )
    };

    InstanceAsset {
        name: file_entry.name.clone(),
        path: file_entry.path.clone(),
        url: resolved_url,
        sha256: file_entry.sha256.clone(),
        md5: file_entry.md5.clone(),
        size: file_entry.size,
        required: file_entry.required,
        target: file_entry.target.clone(),
    }
}

pub fn get_local_file_path(instance_dir: &Path, file_path: &str) -> Result<PathBuf, String> {
    let normalized = file_path.trim_start_matches('/');
    let without_files = if normalized.starts_with("files/") { &normalized[6..] } else { normalized };

    let mut parts: Vec<&str> = without_files.split('/').collect();
    if parts.is_empty() {
        return Err(format!("Invalid file path: {}", file_path));
    }

    let file_name = parts.last().copied().unwrap_or("");

    if (without_files.starts_with("config/") || without_files.starts_with("config/config/"))
        && (file_name.eq_ignore_ascii_case("options.txt") || file_name.eq_ignore_ascii_case("servers.dat"))
    {
        return Ok(instance_dir.join(file_name));
    }

    if parts.len() >= 2 && parts[0] == "config" && parts[1] == "config" {
        parts.remove(1);
    }

    let target_path = PathBuf::from(parts.join("/"));
    Ok(instance_dir.join(target_path))
}

pub async fn download_file(url: &str, file_path: &Path) -> Result<(), String> {
    use tokio::io::AsyncWriteExt;

    let client = reqwest::Client::builder()
        .user_agent("KindlyKlanKlient/1.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to start download from {}: {}", url, e))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("HTTP error {} for {}: {}", status, url, error_text));
    }

    let parent_dir = file_path.parent().ok_or_else(|| format!("Invalid path: {}", file_path.display()))?;
    tokio::fs::create_dir_all(parent_dir).await
        .map_err(|e| format!("Failed to create parent directory {}: {}", parent_dir.display(), e))?;

    let tmp_path = file_path.with_extension("kk.tmp");
    let mut tmp_file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|e| format!("Failed to create temp file {}: {}", tmp_path.display(), e))?;

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("Failed to read response chunk from {}: {}", url, e))?
    {
        tmp_file
            .write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write temp file {}: {}", tmp_path.display(), e))?;
    }

    tmp_file
        .flush()
        .await
        .map_err(|e| format!("Failed to flush temp file {}: {}", tmp_path.display(), e))?;
    tmp_file
        .sync_all()
        .await
        .map_err(|e| format!("Failed to sync temp file {}: {}", tmp_path.display(), e))?;
    drop(tmp_file);

    tokio::fs::rename(&tmp_path, file_path)
        .await
        .map_err(|e| format!("Failed to move temp file to {}: {}", file_path.display(), e))?;

    Ok(())
}

pub async fn download_file_with_retry(url: &str, file_path: &Path) -> Result<(), String> {
    const MAX_RETRIES: u32 = 3;

    for attempt in 1..=MAX_RETRIES {
        match download_file(url, file_path).await {
            Ok(_) => return Ok(()),
            Err(_e) => {
                if attempt < MAX_RETRIES {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    Err(format!("Failed to download {} after {} attempts", url, MAX_RETRIES))
}

pub fn verify_file_checksum(file_path: &Path, expected_sha256: &str) -> Result<(), String> {
    use sha2::{Digest, Sha256};

    let content = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read file for checksum verification: {}", e))?;

    let mut hasher = Sha256::new();
    hasher.update(&content);
    let actual_sha256 = format!("{:x}", hasher.finalize());

    if actual_sha256 != expected_sha256 {
        return Err(format!(
            "Checksum verification failed for {}: expected {}, got {}",
            file_path.display(),
            expected_sha256,
            actual_sha256
        ));
    }

    Ok(())
}

pub fn verify_file_md5(file_path: &Path, expected_md5: &str) -> Result<(), String> {
    let content = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read file for md5 verification: {}", e))?;
    let digest = md5::compute(content);
    let actual = format!("{:x}", digest);
    if actual.eq_ignore_ascii_case(expected_md5) {
        Ok(())
    } else {
        Err(format!("MD5 mismatch for {}: expected {}, got {}", file_path.display(), expected_md5, actual))
    }
}

pub fn build_distribution_url(distribution_url: &str) -> String {
    if distribution_url.trim_end_matches('/').ends_with("/dist") {
        distribution_url.trim_end_matches('/').to_string()
    } else {
        distribution_url.trim_end_matches('/').to_string()
    }
}

pub fn count_instance_files(manifest: &crate::models::InstanceManifest) -> usize {
    let mut n = manifest.files.mods.len() + manifest.files.configs.len();
    if let Some(rp) = &manifest.files.resourcepacks { n += rp.len(); }
    if let Some(sp) = &manifest.files.shaderpacks { n += sp.len(); }
    n
}

pub async fn count_mojang_assets_pending(instance_dir: &Path, mc_version: &str) -> Result<usize, String> {
    let version_dir = instance_dir.join("versions").join(mc_version);
    let json_path = version_dir.join(format!("{}.json", mc_version));
    if !json_path.exists() { return Ok(0); }
    #[derive(serde::Deserialize)]
    struct AssetIndexRef { id: String, url: String }
    #[derive(serde::Deserialize)]
    struct VJson { #[serde(rename="assetIndex")] asset_index: Option<AssetIndexRef> }
    let vtext = tokio::fs::read_to_string(&json_path).await.map_err(|e| e.to_string())?;
    let vj: VJson = serde_json::from_str(&vtext).map_err(|e| e.to_string())?;
    let Some(ai) = vj.asset_index else { return Ok(0); };
    let assets_dir = instance_dir.join("assets");
    let indexes_dir = assets_dir.join("indexes");
    tokio::fs::create_dir_all(&indexes_dir).await.map_err(|e| e.to_string())?;
    let index_path = indexes_dir.join(format!("{}.json", ai.id));
    if !index_path.exists() {
        download_file_with_retry(&ai.url, &index_path).await?;
    }
    let index_text = tokio::fs::read_to_string(&index_path).await.map_err(|e| e.to_string())?;
    #[derive(serde::Deserialize)]
    struct AssetObject { hash: String }
    #[derive(serde::Deserialize)]
    struct AssetIndex { objects: std::collections::HashMap<String, AssetObject> }
    let aidx: AssetIndex = serde_json::from_str(&index_text).map_err(|e| e.to_string())?;
    let objects_dir = assets_dir.join("objects");
    tokio::fs::create_dir_all(&objects_dir).await.map_err(|e| e.to_string())?;
    let mut pending = 0usize;
    for (_name, obj) in aidx.objects {
        let prefix = &obj.hash[0..2];
        let obj_path = objects_dir.join(prefix).join(&obj.hash);
        if !obj_path.exists() { pending += 1; }
    }
    Ok(pending)
}

pub async fn create_instance_directory_safe(instance_id: &str, _app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    use std::env;

    let mut data_dir = if let Ok(home) = env::var("HOME") {
        PathBuf::from(home)
    } else if let Ok(home) = env::var("USERPROFILE") {
        PathBuf::from(home)
    } else {
        return Err("Could not determine user home directory".to_string());
    };

    data_dir.push(".kindlyklanklient");
    data_dir.push(instance_id);

    tokio::fs::create_dir_all(&data_dir).await
        .map_err(|e| format!("Failed to create instance directory: {}", e))?;

    Ok(data_dir)
}

pub async fn ensure_minecraft_client_present(instance_dir: &Path, mc_version: &str) -> Result<(), String> {
    let version_dir = instance_dir.join("versions").join(mc_version);
    let jar_path = version_dir.join(format!("{}.jar", mc_version));
    let json_path = version_dir.join(format!("{}.json", mc_version));

    tokio::fs::create_dir_all(&version_dir).await
        .map_err(|e| format!("Failed to create version dir {}: {}", version_dir.display(), e))?;

    if !json_path.exists() {
        let manifest_url = "https://launchermeta.mojang.com/mc/game/version_manifest.json";
        let manifest_text = reqwest::get(manifest_url).await
            .map_err(|e| format!("Failed to fetch Mojang manifest: {}", e))?
            .text().await
            .map_err(|e| format!("Failed to read Mojang manifest: {}", e))?;

        #[derive(serde::Deserialize)]
        struct VmEntry { id: String, url: String }
        #[derive(serde::Deserialize)]
        struct VmRoot { versions: Vec<VmEntry> }

        let vm: VmRoot = serde_json::from_str(&manifest_text)
            .map_err(|e| format!("Failed to parse Mojang manifest: {}", e))?;
        let Some(ver) = vm.versions.into_iter().find(|v| v.id == mc_version) else {
            return Err(format!("Minecraft version {} not found in Mojang manifest", mc_version));
        };

        let vjson_text = reqwest::get(&ver.url).await
            .map_err(|e| format!("Failed to fetch version json: {}", e))?
            .text().await
            .map_err(|e| format!("Failed to read version json: {}", e))?;
        tokio::fs::write(&json_path, &vjson_text).await
            .map_err(|e| format!("Failed to write version json: {}", e))?;
    }

    if !jar_path.exists() {
        let vjson_text = tokio::fs::read_to_string(&json_path).await
            .map_err(|e| format!("Failed to read version json: {}", e))?;
        #[derive(serde::Deserialize)]
        struct Dls { client: Option<Info> }
        #[derive(serde::Deserialize)]
        struct Info { url: String }
        #[derive(serde::Deserialize)]
        struct Vj { downloads: Option<Dls> }
        let vj: Vj = serde_json::from_str(&vjson_text)
            .map_err(|e| format!("Failed to parse version json: {}", e))?;
        if let Some(url) = vj.downloads.and_then(|d| d.client).map(|c| c.url) {
            download_file_with_retry(&url, &jar_path).await?;
        } else {
            return Err("Client download URL not found in version json".to_string());
        }
    }
    Ok(())
}

pub async fn ensure_version_libraries(instance_dir: &Path, mc_version: &str) -> Result<(), String> {
    let version_dir = instance_dir.join("versions").join(mc_version);
    let json_path = version_dir.join(format!("{}.json", mc_version));
    if !json_path.exists() { return Err(format!("Version json not found: {}", json_path.display())); }
    let version_data = tokio::fs::read_to_string(&json_path).await.map_err(|e| e.to_string())?;
    #[derive(serde::Deserialize)]
    struct VersionJson { libraries: Vec<crate::versions::Library> }
    let vj: VersionJson = serde_json::from_str(&version_data).map_err(|e| e.to_string())?;
    let os_name = if cfg!(target_os = "windows") { "windows" } else { "linux" };
    for lib in vj.libraries.iter() {
        if !crate::versions::is_library_allowed(lib, os_name) { continue; }
        if let Some(downloads) = &lib.downloads { if let Some(artifact) = &downloads.artifact {
            let lib_path = instance_dir.join("libraries").join(&artifact.path);
            if let Some(parent) = lib_path.parent() { tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?; }
            if !lib_path.exists() {
                download_file_with_retry(&artifact.url, &lib_path).await?;
            }
        }}
    }
    Ok(())
}

pub async fn install_mod_loader(minecraft_version: &str, mod_loader: &ModLoader, instance_dir: &Path) -> Result<(), String> {
    match mod_loader.r#type.as_str() {
        "fabric" => install_fabric(minecraft_version, &mod_loader.version, instance_dir).await,
        "neoforge" => install_neoforge(minecraft_version, &mod_loader.version, instance_dir).await,
        "vanilla" => Ok(()),
        _ => Err(format!("Unsupported mod loader type: {}", mod_loader.r#type))
    }
}

async fn install_fabric(minecraft_version: &str, fabric_version: &str, instance_dir: &Path) -> Result<(), String> {
    let loader_jar = instance_dir
        .join("libraries")
        .join("net")
        .join("fabricmc")
        .join("fabric-loader")
        .join(fabric_version)
        .join(format!("fabric-loader-{}.jar", fabric_version));
    if loader_jar.exists() {
        return Ok(());
    }

    let libraries_dir = instance_dir.join("libraries");
    tokio::fs::create_dir_all(&libraries_dir).await
        .map_err(|e| format!("Failed to create libraries directory: {}", e))?;

    let installer_info = get_fabric_installer_info().await?;
    let installer_path = download_fabric_installer(&installer_info, &libraries_dir).await?;
    let profile_json = get_fabric_profile_json(minecraft_version, fabric_version).await?;
    download_fabric_libraries(&profile_json, &libraries_dir).await?;
    run_fabric_installer(&installer_path, instance_dir, minecraft_version, fabric_version).await?;
    ensure_minecraft_client_present(instance_dir, minecraft_version).await?;
    Ok(())
}

pub async fn install_neoforge(_minecraft_version: &str, _neo_version: &str, _instance_dir: &Path) -> Result<(), String> {
    // TODO: Implement NeoForge installation when supported
    Ok(())
}

// Stubs expected to be defined elsewhere in codebase (existing functions)
async fn get_fabric_installer_info() -> Result<crate::models::FabricInstallerMeta, String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://meta.fabricmc.net/v2/versions/installer")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Fabric installer info: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let installers: Vec<crate::models::FabricInstallerMeta> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse installer info: {}", e))?;

    let stable_installer = installers
        .into_iter()
        .find(|i| i.stable)
        .ok_or("No stable Fabric installer found")?;

    Ok(stable_installer)
}

async fn get_fabric_profile_json(minecraft_version: &str, fabric_version: &str) -> Result<crate::models::FabricProfileJson, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
        minecraft_version, fabric_version
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Fabric profile: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let profile: crate::models::FabricProfileJson = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Fabric profile: {}", e))?;

    Ok(profile)
}
async fn download_fabric_installer(info: &crate::models::FabricInstallerMeta, libs: &Path) -> Result<PathBuf, String> {
    let installer_path = libs.join(format!("fabric-installer-{}.jar", info.version));
    download_file_with_retry(&info.url, &installer_path).await?;
    Ok(installer_path)
}

async fn download_fabric_libraries(profile: &crate::models::FabricProfileJson, libs: &Path) -> Result<(), String> {
    for library in profile.libraries.iter() {
        let library_path = resolve_maven_path(&library.name, libs)?;
        if let Some(parent) = library_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| format!("Failed to create library directory: {}", e))?;
        }
        let library_url = build_library_url(library)?;
        download_file_with_retry(&library_url, &library_path).await?;
    }
    Ok(())
}

async fn run_fabric_installer(installer: &Path, instance_dir: &Path, mc: &str, fabric: &str) -> Result<(), String> {
    let java_path = crate::launcher::find_java_executable().await?;
    let mut cmd = Command::new(&java_path);
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd
        .args(&[
            "-jar",
            &installer.to_string_lossy(),
            "client",
            "-noprofile",
            "-dir",
            &instance_dir.to_string_lossy(),
            "-mcversion",
            mc,
            "-loader",
            fabric
        ])
        .output()
        .map_err(|e| format!("Failed to run Fabric installer: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Fabric installer failed: {}", stderr));
    }
    Ok(())
}
fn resolve_maven_path(maven_id: &str, libraries_dir: &Path) -> Result<PathBuf, String> {
    let parts: Vec<&str> = maven_id.split(':').collect();
    if parts.len() < 3 { return Err(format!("Invalid Maven ID: {}", maven_id)); }
    let group_id = parts[0].replace('.', "/");
    let artifact_id = parts[1];
    let version = parts[2];
    let filename = format!("{}-{}.jar", artifact_id, version);
    Ok(libraries_dir.join(&group_id).join(artifact_id).join(version).join(filename))
}

fn build_library_url(library: &crate::models::FabricLibrary) -> Result<String, String> {
    let parts: Vec<&str> = library.name.split(':').collect();
    if parts.len() < 3 { return Err(format!("Invalid Maven ID: {}", library.name)); }
    let group_id_path = parts[0].replace('.', "/");
    let artifact_id = parts[1];
    let version = parts[2];
    let filename = format!("{}-{}.jar", artifact_id, version);
    let base = library.url.as_ref().map(|u| u.trim_end_matches('/').to_string()).unwrap_or_else(|| "https://repo1.maven.org/maven2".to_string());
    Ok(format!("{}/{}/{}/{}/{}", base, group_id_path, artifact_id, version, filename))
}

pub async fn ensure_assets_present(app_handle: &tauri::AppHandle, instance_dir: &Path, mc_version: &str) -> Result<String, String> {
    let version_dir = instance_dir.join("versions").join(mc_version);
    let json_path = version_dir.join(format!("{}.json", mc_version));
    if !json_path.exists() { return Err(format!("Version json not found: {}", json_path.display())); }
    #[derive(serde::Deserialize)]
    struct AssetIndexRef { id: String, url: String }
    #[derive(serde::Deserialize)]
    struct VJson { #[serde(rename="assetIndex")] asset_index: Option<AssetIndexRef> }
    let vtext = tokio::fs::read_to_string(&json_path).await.map_err(|e| e.to_string())?;
    let vj: VJson = serde_json::from_str(&vtext).map_err(|e| e.to_string())?;
    let Some(ai) = vj.asset_index else { return Err("assetIndex missing in version json".to_string()); };
    let assets_dir = instance_dir.join("assets");
    let indexes_dir = assets_dir.join("indexes");
    tokio::fs::create_dir_all(&indexes_dir).await.map_err(|e| e.to_string())?;
    let index_path = indexes_dir.join(format!("{}.json", ai.id));
    if !index_path.exists() { download_file_with_retry(&ai.url, &index_path).await?; }
    let index_text = tokio::fs::read_to_string(&index_path).await.map_err(|e| e.to_string())?;
    #[derive(serde::Deserialize)]
    struct AssetObject { hash: String }
    #[derive(serde::Deserialize)]
    struct AssetIndex { objects: std::collections::HashMap<String, AssetObject> }
    let aidx: AssetIndex = serde_json::from_str(&index_text).map_err(|e| e.to_string())?;
    let objects_dir = assets_dir.join("objects");
    tokio::fs::create_dir_all(&objects_dir).await.map_err(|e| e.to_string())?;
    let mut pending: Vec<(String, String)> = Vec::new();
    for (_name, obj) in aidx.objects {
        let prefix = obj.hash[0..2].to_string();
        let obj_dir = objects_dir.join(&prefix);
        tokio::fs::create_dir_all(&obj_dir).await.map_err(|e| e.to_string())?;
        let obj_path = obj_dir.join(&obj.hash);
        if !obj_path.exists() { pending.push((prefix, obj.hash)); }
    }
    if pending.is_empty() { return Ok(ai.id); }
    let parallel = num_cpus::get().saturating_mul(8).max(50);
    let progress = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let total_count = pending.len() as u64;
    let client = std::sync::Arc::new(reqwest::Client::builder()
        .user_agent("KindlyKlanKlient/1.0")
        .timeout(std::time::Duration::from_secs(30))
        .pool_max_idle_per_host(parallel)
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .build().map_err(|e| e.to_string())?);
    use futures_util::stream::{self, StreamExt};
    let results: Vec<Result<(), String>> = stream::iter(pending.into_iter().map(|(prefix, hash)| {
        let client = client.clone();
        let objects_dir = objects_dir.clone();
        let progress = progress.clone();
        let app_handle = app_handle.clone();
        async move {
            let url = format!("https://resources.download.minecraft.net/{}/{}", prefix, hash);
            let obj_path = objects_dir.join(&prefix).join(&hash);
            let resp = client.get(&url).send().await.map_err(|e| format!("Request failed: {}", e))?;
            if !resp.status().is_success() { return Err(format!("Asset HTTP {} for {}", resp.status(), url)); }
            let tmp = obj_path.with_extension("kk.tmp");
            let bytes = resp.bytes().await.map_err(|e| format!("Download failed: {}", e))?;
            tokio::fs::write(&tmp, &bytes).await.map_err(|e| format!("Write failed: {}", e))?;
            tokio::fs::rename(&tmp, &obj_path).await.map_err(|e| format!("Rename failed: {}", e))?;
            let cur = progress.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            let _ = app_handle.emit("asset-download-progress", serde_json::json!({
                "current": cur,
                "total": total_count,
                "percentage": ((cur as f32 / total_count as f32) * 100.0).min(100.0),
                "current_file": "",
                "status": "Mojang"
            }));
            Ok(())
        }
    })).buffer_unordered(parallel).collect().await;
    for result in results { if let Err(e) = result { eprintln!("Warning: Mojang asset download error: {}", e); } }
    let _ = app_handle.emit("asset-download-completed", serde_json::json!({ "phase": "mojang" }));
    Ok(ai.id)
}

pub async fn ensure_assets_present_with_progress(
    app_handle: &tauri::AppHandle,
    instance_dir: &Path,
    mc_version: &str,
    combined: Option<(std::sync::Arc<std::sync::atomic::AtomicU64>, u64)>
) -> Result<String, String> {
    let version_dir = instance_dir.join("versions").join(mc_version);
    let json_path = version_dir.join(format!("{}.json", mc_version));
    if !json_path.exists() { return Err(format!("Version json not found: {}", json_path.display())); }
    #[derive(serde::Deserialize)]
    struct AssetIndexRef { id: String, url: String }
    #[derive(serde::Deserialize)]
    struct VJson { #[serde(rename="assetIndex")] asset_index: Option<AssetIndexRef> }
    let vtext = tokio::fs::read_to_string(&json_path).await.map_err(|e| e.to_string())?;
    let vj: VJson = serde_json::from_str(&vtext).map_err(|e| e.to_string())?;
    let Some(ai) = vj.asset_index else { return Err("assetIndex missing in version json".to_string()); };
    let assets_dir = instance_dir.join("assets");
    let indexes_dir = assets_dir.join("indexes");
    tokio::fs::create_dir_all(&indexes_dir).await.map_err(|e| e.to_string())?;
    let index_path = indexes_dir.join(format!("{}.json", ai.id));
    if !index_path.exists() { download_file_with_retry(&ai.url, &index_path).await?; }
    let index_text = tokio::fs::read_to_string(&index_path).await.map_err(|e| e.to_string())?;
    #[derive(serde::Deserialize)]
    struct AssetObject { hash: String }
    #[derive(serde::Deserialize)]
    struct AssetIndex { objects: std::collections::HashMap<String, AssetObject> }
    let aidx: AssetIndex = serde_json::from_str(&index_text).map_err(|e| e.to_string())?;
    let objects_dir = assets_dir.join("objects");
    tokio::fs::create_dir_all(&objects_dir).await.map_err(|e| e.to_string())?;
    let mut pending: Vec<(String, String)> = Vec::new();
    for (_name, obj) in aidx.objects {
        let prefix = obj.hash[0..2].to_string();
        let obj_dir = objects_dir.join(&prefix);
        tokio::fs::create_dir_all(&obj_dir).await.map_err(|e| e.to_string())?;
        let obj_path = obj_dir.join(&obj.hash);
        if !obj_path.exists() { pending.push((prefix, obj.hash)); }
    }
    if pending.is_empty() { return Ok(ai.id); }
    let parallel = num_cpus::get().saturating_mul(8).max(50);
    use futures_util::stream::{self, StreamExt};
    let results: Vec<Result<(), String>> = stream::iter(pending.into_iter().map(|(prefix, hash)| {
        let objects_dir = objects_dir.clone();
        let app_handle = app_handle.clone();
        let combined = combined.clone();
        async move {
            let url = format!("https://resources.download.minecraft.net/{}/{}", prefix, hash);
            let obj_path = objects_dir.join(&prefix).join(&hash);
            download_file_with_retry(&url, &obj_path).await?;
            if let Some((counter, total)) = &combined {
                let cur = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                let _ = app_handle.emit("asset-download-progress", serde_json::json!({
                    "current": cur,
                    "total": total,
                    "percentage": ((cur as f32 / *total as f32) * 100.0).min(100.0),
                    "current_file": "",
                    "status": "Mojang"
                }));
            }
            Ok(())
        }
    })).buffer_unordered(parallel).collect().await;
    for result in results { if let Err(e) = result { eprintln!("Warning: Mojang asset download error: {}", e); } }
    Ok(ai.id)
}
