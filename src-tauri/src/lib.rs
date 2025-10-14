// Kindly Klan Klient
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use std::collections::HashMap;
use reqwest;
use std::fs::File;
use std::io::Write;
use tauri::{Url, Emitter};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

mod logging;
mod sessions;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionManifest {
    pub distribution: DistributionInfo,
    pub instances: Vec<InstanceSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub base_url: String,
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub minecraft_version: String,
    pub icon: Option<String>,
    pub background: Option<String>,
    pub last_updated: Option<String>,
    pub instance_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceManifest {
    pub instance: InstanceInfo,
    pub files: InstanceFiles,
    pub launch_settings: LaunchSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceAsset {
    pub name: String,
    pub path: String,
    pub url: String,
    pub sha256: String,
    pub md5: Option<String>,
    pub size: Option<u64>,
    pub required: Option<bool>,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateState {
    pub last_check: String,
    pub available_version: Option<String>,
    pub current_version: String,
    pub downloaded: bool,
    pub download_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhitelistEntry {
    pub minecraft_username: String,
    pub global_access: bool,
    pub allowed_instances: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessCheck {
    pub has_access: bool,
    pub allowed_instances: Vec<String>,
    pub global_access: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDownloadProgress {
    pub current: u64,
    pub total: u64,
    pub percentage: f32,
    pub current_file: String,
    pub status: String,
}

// Fabric Meta API structures for version and library information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricInstallerMeta {
    pub version: String,
    pub stable: bool,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricLoaderMeta {
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricVersionMeta {
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricLibrary {
    pub name: String,
    pub url: Option<String>,
    pub sha1: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricProfileJson {
    pub id: String,
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub time: String,
    pub r#type: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub arguments: FabricArguments,
    pub libraries: Vec<FabricLibrary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricArguments {
    #[serde(default)]
    pub game: Vec<String>,
    #[serde(default)]
    pub jvm: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub minecraft_version: String,
    pub mod_loader: Option<ModLoader>,
    pub icon: Option<String>,
    pub background: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModLoader {
    pub r#type: String, // Supported types: "fabric", "forge", "neoforge", "vanilla"
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceFiles {
    pub mods: Vec<FileEntry>,
    pub configs: Vec<FileEntry>,
    pub resourcepacks: Option<Vec<FileEntry>>,
    pub shaderpacks: Option<Vec<FileEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub url: String,
    pub sha256: String,
    pub md5: Option<String>,
    pub size: Option<u64>,
    pub required: Option<bool>,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchSettings {
    pub min_ram: u32,
    pub recommended_ram: u32,
    pub jvm_args: Option<Vec<String>>,
}

// Microsoft authentication flow structures
#[derive(Debug, Serialize, Deserialize, Clone)]
struct MicrosoftAuthResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
    scope: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct XboxLiveAuthResponse {
    #[serde(rename = "IssueInstant")]
    issue_instant: String,
    #[serde(rename = "NotAfter")]
    not_after: String,
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: HashMap<String, Vec<HashMap<String, String>>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct XstsAuthResponse {
    #[serde(rename = "IssueInstant")]
    issue_instant: String,
    #[serde(rename = "NotAfter")]
    not_after: String,
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: HashMap<String, Vec<HashMap<String, String>>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MinecraftAuthResponse {
    username: String,
    roles: Vec<String>,
    access_token: String,
    token_type: String,
    expires_in: u64,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub access_token: String,
    pub username: String,
    pub uuid: String,
    pub user_type: String, // "microsoft" or "offline"
    pub expires_at: Option<i64>,
    pub refresh_token: Option<String>,
}

// Minecraft version structures from Mojang API
#[derive(Debug, Serialize, Deserialize, Clone)]
struct MinecraftVersion {
    id: String,
    #[serde(rename = "type")]
    version_type: String,
    url: String,
    time: String,
    #[serde(rename = "releaseTime")]
    release_time: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct VersionManifest {
    versions: Vec<MinecraftVersion>,
}

// Library and rule structures for Minecraft version parsing
#[derive(Deserialize, Debug, Clone)]
struct Extract {
    #[allow(dead_code)]
    exclude: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct Rule {
    action: String,
    os: Option<OsRule>,
}

#[derive(Deserialize, Debug, Clone)]
struct OsRule {
    name: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct Library {
    #[allow(dead_code)]
    name: String,
    downloads: Option<LibraryDownloads>,
    #[allow(dead_code)]
    natives: Option<HashMap<String, String>>,
    rules: Option<Vec<Rule>>,
    #[serde(default)]
    #[allow(dead_code)]
    extract: Option<Extract>,
}

impl Library {
    #[allow(dead_code)]
    fn get_extract(&self) -> Option<&Extract> {
        self.extract.as_ref()
    }
}

#[derive(Deserialize, Debug, Clone)]
struct LibraryDownloads {
    artifact: Option<LibraryArtifact>,
    #[allow(dead_code)]
    classifiers: Option<HashMap<String, LibraryArtifact>>,
}

#[derive(Deserialize, Debug, Clone)]
struct LibraryArtifact {
    url: String,
    path: String,
}

// Check if a library is allowed for the current operating system based on rules
fn is_library_allowed(lib: &Library, os_name: &str) -> bool {
    let rules = match &lib.rules {
        Some(r) => r,
        None => return true,
    };
    let mut allowed = false;
    for rule in rules {
        let matches = if let Some(os) = &rule.os {
            if let Some(name) = &os.name {
                name == os_name
            } else {
                true
            }
        } else {
            true
        };
        if matches {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

// Launcher directory configuration
struct LauncherConfig {
    minecraft_dir: PathBuf,
    versions_dir: PathBuf,
    assets_dir: PathBuf,
    libraries_dir: PathBuf,
}

impl LauncherConfig {
    fn new() -> Result<Self> {
        let home = env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".to_string());
        let minecraft_dir = PathBuf::from(home).join(".kindlyklanklient");
        Ok(Self {
            versions_dir: minecraft_dir.join("versions"),
            assets_dir: minecraft_dir.join("assets"),
            libraries_dir: minecraft_dir.join("libraries"),
            minecraft_dir,
        })
    }

    async fn ensure_directories(&self) -> Result<()> {
        fs::create_dir_all(&self.minecraft_dir).await?;
        fs::create_dir_all(&self.versions_dir).await?;
        fs::create_dir_all(&self.assets_dir).await?;
        fs::create_dir_all(&self.libraries_dir).await?;
        Ok(())
    }
}

// Minecraft launcher implementation with version management
struct MinecraftLauncher {
    config: LauncherConfig,
}

impl MinecraftLauncher {
    fn new() -> Result<Self> {
        Ok(Self {
            config: LauncherConfig::new()?,
        })
    }

// Fetch available Minecraft versions from Mojang API
async fn get_available_versions(&self) -> Result<Vec<MinecraftVersion>> {
    let url = "https://launchermeta.mojang.com/mc/game/version_manifest.json";
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    match client.get(url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let text = response.text().await?;
                match serde_json::from_str::<VersionManifest>(&text) {
                    Ok(manifest) => {
                        let release_versions: Vec<MinecraftVersion> = manifest
                            .versions
                            .into_iter()
                            .filter(|v| v.version_type == "release")
                            .collect();
                        Ok(release_versions)
                    }
                    Err(e) => Err(e.into())
                }
            } else {
                Err(anyhow::anyhow!("API error: {}", response.status()))
            }
        }
        Err(e) => Err(e.into())
    }
}

// Download Minecraft version files (manifest, client.jar, libraries, assets)
async fn download_version(&self, version: &MinecraftVersion) -> Result<()> {
    let version_dir = self.config.versions_dir.join(&version.id);
    fs::create_dir_all(&version_dir).await?;
    let natives_dir = version_dir.join("natives");
    fs::create_dir_all(&natives_dir).await?;

    // Download version manifest
    let version_response = reqwest::get(&version.url).await?;
    let version_data = version_response.text().await?;
    let version_file = version_dir.join(format!("{}.json", version.id));
    fs::write(&version_file, &version_data).await?;

    // Parse version JSON and download client.jar
    #[derive(Deserialize)]
    struct VersionJson {
        downloads: VersionJsonDownloads,
        libraries: Vec<Library>,
        #[serde(rename = "assetIndex")]
        asset_index: Option<AssetIndex>,
    }
    #[derive(Deserialize)]
    struct VersionJsonDownloads {
        client: Option<DownloadInfo>,
    }
    #[derive(Deserialize)]
    struct DownloadInfo {
        url: String,
    }
    #[derive(Deserialize)]
    struct AssetIndex {
        id: String,
        url: String,
    }

    let version_json: VersionJson = serde_json::from_str(&version_data)?;
    if let Some(client) = version_json.downloads.client {
        let jar_url = client.url;
        let jar_path = version_dir.join(format!("{}.jar", version.id));
        let resp = reqwest::get(&jar_url).await?;
        let bytes = resp.bytes().await?.to_vec();
        let mut out = File::create(&jar_path)?;
        out.write_all(&bytes)?;
    }

    // Download libraries for Windows
    let os_name = "windows";
    for lib in &version_json.libraries {
        if !is_library_allowed(lib, os_name) { continue; }
        if let Some(downloads) = &lib.downloads {
            if let Some(artifact) = &downloads.artifact {
                let lib_path = self.config.libraries_dir.join(&artifact.path);
                if !lib_path.exists() {
                    if let Some(parent) = lib_path.parent() {
                        fs::create_dir_all(parent).await?;
                    }
                    let resp = reqwest::get(&artifact.url).await?;
                    let bytes = resp.bytes().await?.to_vec();
                    let mut out = File::create(&lib_path)?;
                    out.write_all(&bytes)?;
                }
            }
        }
    }

    // Download assets if asset index is present
    if let Some(asset_index) = &version_json.asset_index {
        let indexes_dir = self.config.assets_dir.join("indexes");
        fs::create_dir_all(&indexes_dir).await?;
        let index_path = indexes_dir.join(format!("{}.json", asset_index.id));

        let resp = reqwest::get(&asset_index.url).await?;
        let bytes = resp.bytes().await?.to_vec();
        let mut out = File::create(&index_path)?;
        out.write_all(&bytes)?;

        let index_data = String::from_utf8(bytes)?;
        #[derive(Deserialize)]
        struct AssetIndexJson {
            objects: HashMap<String, AssetObject>,
        }
        #[derive(Deserialize, Clone)]
        struct AssetObject {
            hash: String,
        }

        let asset_index_json: AssetIndexJson = serde_json::from_str(&index_data)?;

        // Download missing asset objects in chunks
        let mut missing_assets = Vec::new();
        for (_key, obj) in &asset_index_json.objects {
            let hash_prefix = &obj.hash[0..2];
            let object_dir = self.config.assets_dir.join("objects").join(hash_prefix);
            let object_path = object_dir.join(&obj.hash);
            if !object_path.exists() {
                missing_assets.push(obj.clone());
            }
        }

        if !missing_assets.is_empty() {
            let client = reqwest::Client::new();
            for chunk in missing_assets.chunks(50) {
                let mut tasks = Vec::new();
                for obj in chunk {
                    let hash_prefix = &obj.hash[0..2];
                    let object_dir = self.config.assets_dir.join("objects").join(hash_prefix);
                    fs::create_dir_all(&object_dir).await?;
                    let object_path = object_dir.join(&obj.hash);
                    let object_url = format!("https://resources.download.minecraft.net/{}/{}", hash_prefix, obj.hash);

                    let client_clone = client.clone();
                    let task = tokio::spawn(async move {
                        match client_clone.get(&object_url).send().await {
                            Ok(resp) => {
                                match resp.bytes().await {
                                    Ok(bytes) => {
                                        match tokio::fs::File::create(&object_path).await {
                                            Ok(mut out) => {
                                                match out.write_all(&bytes).await {
                                                    Ok(_) => Ok(()),
                                                    Err(e) => Err(anyhow::anyhow!("Write failed: {}", e))
                                                }
                                            }
                                            Err(e) => Err(anyhow::anyhow!("File create failed: {}", e))
                                        }
                                    }
                                    Err(e) => Err(anyhow::anyhow!("Bytes failed: {}", e))
                                }
                            }
                            Err(e) => Err(anyhow::anyhow!("Request failed: {}", e))
                        }
                    });
                    tasks.push(task);
                }

                for task in tasks {
                    if let Err(e) = task.await {
                        eprintln!("Asset download task failed: {}", e);
                    }
                }
            }
        }
    }
    Ok(())
}

// Build classpath string for Minecraft launch
async fn build_classpath(&self, version: &str) -> Result<String> {
    let version_dir = self.config.versions_dir.join(version);
    let version_file = version_dir.join(format!("{}.json", version));
    let version_data = fs::read_to_string(&version_file).await?;
    #[derive(Deserialize)]
    struct VersionJson {
        libraries: Vec<Library>,
    }
    let version_json: VersionJson = serde_json::from_str(&version_data)?;
    let os_name = "windows";
    let mut classpath = Vec::new();
    for lib in &version_json.libraries {
        if !is_library_allowed(lib, os_name) { continue; }
        if let Some(downloads) = &lib.downloads {
            if let Some(artifact) = &downloads.artifact {
                let lib_path = self.config.libraries_dir.join(&artifact.path);
                classpath.push(lib_path);
            }
        }
    }
    let jar_path = version_dir.join(format!("{}.jar", version));
    classpath.push(jar_path);
    let cp = classpath.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(";");
    Ok(cp)
}

// Launch Minecraft with specified parameters
async fn launch_minecraft(&self, version: &str, username: &str, ram_mb: u32, access_token: Option<&str>, uuid: Option<&str>) -> Result<()> {
    let java_path = self.find_java()?;
    let version_dir = self.config.versions_dir.join(version);
    let jar_path = version_dir.join(format!("{}.jar", version));
    let natives_dir = version_dir.join("natives");

    if !jar_path.exists() {
        return Err(anyhow::anyhow!("Version not downloaded"));
    }

    let classpath = self.build_classpath(version).await?;

    let mut command = Command::new(&java_path);
    command
        .arg(format!("-Xmx{}M", ram_mb))
        .arg(format!("-Xms{}M", ram_mb / 2))
        .arg(format!("-Djava.library.path={}", natives_dir.display()))
        .arg("-cp")
        .arg(classpath)
        .arg("net.minecraft.client.main.Main")
        .arg("--username")
        .arg(username)
        .arg("--version")
        .arg(version)
        .arg("--gameDir")
        .arg(&self.config.minecraft_dir)
        .arg("--assetsDir")
        .arg(&self.config.assets_dir);

    let version_file = version_dir.join(format!("{}.json", version));
    let version_data = fs::read_to_string(&version_file).await?;
    #[derive(Deserialize)]
    struct VersionJson {
        #[serde(rename = "assetIndex")]
        asset_index: Option<AssetIndex>,
    }
    #[derive(Deserialize)]
    struct AssetIndex {
        id: String,
    }
    let version_json: VersionJson = serde_json::from_str(&version_data)?;
    if let Some(asset_index) = version_json.asset_index {
        command.arg("--assetIndex").arg(asset_index.id);
    }
    command.arg("--accessToken").arg(access_token.unwrap_or("0"))
           .arg("--uuid").arg(uuid.unwrap_or("00000000-0000-0000-0000-000000000000"))
           .arg("--userType").arg("msa")
           .arg("--userProperties").arg("{}");

    // Launch Minecraft in detached mode
    let _child = command.spawn()?;
    Ok(())
}

// Find Java executable in common locations
fn find_java(&self) -> Result<PathBuf> {
    if let Ok(output) = Command::new("java").arg("-version").output() {
        if output.status.success() {
            return Ok(PathBuf::from("java"));
        }
    }
    let common_paths = vec![
        "C:\\Program Files\\Java\\jdk-8\\bin\\java.exe",
        "C:\\Program Files\\Java\\jdk-11\\bin\\java.exe",
        "C:\\Program Files\\Java\\jdk-17\\bin\\java.exe",
    ];
    for path in common_paths {
        if Path::new(path).exists() {
            return Ok(PathBuf::from(path));
        }
    }
    anyhow::bail!("Java not found");
}
}

// Get total system RAM in MB using WMIC
fn get_total_ram_mb() -> Result<u32> {
    if let Ok(output) = Command::new("wmic").arg("OS").arg("get").arg("TotalVisibleMemorySize").output() {
        if output.status.success() {
            let stdout = String::from_utf8(output.stdout)?;
            for line in stdout.lines() {
                if let Ok(kb) = line.trim().parse::<u64>() {
                    return Ok((kb / 1024) as u32);
                }
            }
        }
    }
    Ok(4096) // Default 4GB
}

// Microsoft OAuth authentication constants and functions
const AZURE_CLIENT_ID: &str = "d1538b43-1083-43ac-89d5-c88cb0049ada";

// Initiate Microsoft OAuth authentication flow
#[tauri::command]
async fn start_microsoft_auth() -> Result<AuthSession, String> {
    use std::sync::{Arc, Mutex};
    use tauri_plugin_oauth::start_with_config;

    // Shared variable to store OAuth callback URL
    let captured_url = Arc::new(Mutex::new(None::<String>));
    let captured_url_clone = captured_url.clone();

    let config = tauri_plugin_oauth::OauthConfig {
        ports: None,
        response: Some(r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Kindly Klan Klient</title>
</head>
<body>
    <div class="container">
        <h1>Kindly Klan Klient</h1>
        <div class="success">Inicio de sesión exitoso</div>
        <div class="instructions">Puedes cerrar esta pestaña y volver al Kliente.</div>
    </div>
</body>
</html>
        "#.into()),
    };

    let port = start_with_config(config, move |url| {
        let mut captured = captured_url_clone.lock().unwrap();
        *captured = Some(url);
    }).map_err(|e| format!("Failed to start OAuth server: {}", e))?;

    // Build Microsoft OAuth URL using consumers tenant
    let auth_url = format!(
        "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize?client_id={}&response_type=code&redirect_uri=http://localhost:{}&scope=XboxLive.signin%20offline_access&prompt=select_account",
        AZURE_CLIENT_ID, port
    );

    // Open browser for authentication
    if let Err(e) = open::that(&auth_url) {
        return Err(format!("Failed to open browser: {}", e));
    }

    // Wait for OAuth callback with timeout
    let start_time = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(180);

    loop {
        let captured_url_option = {
            let captured = captured_url.lock().unwrap();
            captured.clone()
        };

        if let Some(url) = captured_url_option {
            let auth_code = extract_auth_code_from_url(&url)
                .ok_or_else(|| "No authorization code found in callback URL".to_string())?;
            return complete_microsoft_auth_internal(auth_code, port).await;
        }

        if start_time.elapsed() > timeout {
            return Err("Authentication timeout".to_string());
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

// Complete Microsoft OAuth authentication flow
async fn complete_microsoft_auth_internal(auth_code: String, port: u16) -> Result<AuthSession, String> {
    // Exchange authorization code for Microsoft access token
    let ms_token = exchange_auth_code_for_token(auth_code, port).await
        .map_err(|e| format!("Failed to exchange auth code: {}", e))?;

    // Authenticate with Xbox Live
    let xbox_token = authenticate_xbox_live(&ms_token.access_token).await
        .map_err(|e| format!("Failed Xbox Live auth: {}", e))?;

    // Authenticate with XSTS
    let xsts_token = authenticate_xsts(&xbox_token.token).await
        .map_err(|e| format!("Failed XSTS auth: {}", e))?;

    // Get Minecraft access token
    let mc_token = authenticate_minecraft(&xsts_token).await
        .map_err(|e| format!("Failed Minecraft auth: {}", e))?;

    // Get Minecraft profile
    let access_token = mc_token.access_token.clone();
    let profile_json = get_minecraft_profile(access_token.clone()).await
        .map_err(|e| format!("Failed to get profile: {}", e))?;

    let profile: serde_json::Value = serde_json::from_str(&profile_json)
        .map_err(|e| format!("Failed to parse profile JSON: {}", e))?;

    let username = profile["name"].as_str().unwrap_or("Unknown");
    let uuid = profile["id"].as_str().unwrap_or("unknown");

    let expires_at = (Utc::now().timestamp() + ms_token.expires_in as i64).into();
    let session = AuthSession {
        access_token: access_token,
        username: username.to_string(),
        uuid: uuid.to_string(),
        user_type: "microsoft".to_string(),
        expires_at: Some(expires_at),
        refresh_token: ms_token.refresh_token.clone(),
    };

    Ok(session)
}

// Extract authorization code from OAuth callback URL
fn extract_auth_code_from_url(url_str: &str) -> Option<String> {
    if let Ok(url) = Url::parse(url_str) {
        for (key, value) in url.query_pairs() {
            if key == "code" {
                return Some(value.to_string());
            }
        }
    }
    None
}

// Exchange authorization code for Microsoft access token
async fn exchange_auth_code_for_token(auth_code: String, port: u16) -> Result<MicrosoftAuthResponse> {
    let client = reqwest::Client::new();

    let redirect_uri = format!("http://localhost:{}", port);
    let params = [
        ("client_id", AZURE_CLIENT_ID),
        ("scope", "XboxLive.signin offline_access"),
        ("code", &auth_code),
        ("redirect_uri", &redirect_uri),
        ("grant_type", "authorization_code"),
    ];

    let response = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("Token exchange failed: {}", error_text));
    }

    let token: MicrosoftAuthResponse = response.json().await?;
    Ok(token)
}

// Refresh Microsoft access token using refresh_token
async fn refresh_ms_token(refresh_token: String) -> Result<MicrosoftAuthResponse> {
    let client = reqwest::Client::new();
    let params = [
        ("client_id", AZURE_CLIENT_ID),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token.as_str()),
        ("scope", "XboxLive.signin offline_access"),
    ];

    let response = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("Token refresh failed: {}", error_text));
    }

    let token: MicrosoftAuthResponse = response.json().await?;
    Ok(token)
}

// Refresh full session using stored refresh_token and return updated DB session
#[tauri::command]
async fn refresh_session(app_handle: tauri::AppHandle, username: String) -> Result<sessions::Session, String> {
    let session_manager = sessions::SessionManager::new(&app_handle)
        .map_err(|e| format!("Failed to initialize session manager: {}", e))?;

    let existing = session_manager.get_session(&username)
        .map_err(|e| format!("Failed to get session: {}", e))?;

    let Some(existing_session) = existing else {
        return Err("No existing session".to_string());
    };

    let Some(refresh_token) = existing_session.refresh_token.clone() else {
        return Err("No refresh token stored".to_string());
    };

    // 1) Refresh Microsoft token
    let ms_token = refresh_ms_token(refresh_token)
        .await
        .map_err(|e| format!("Failed to refresh MS token: {}", e))?;

    // 2) Re-auth with Xbox Live / XSTS / Minecraft
    let xbox_token = authenticate_xbox_live(&ms_token.access_token).await
        .map_err(|e| format!("Failed Xbox Live auth: {}", e))?;
    let xsts_token = authenticate_xsts(&xbox_token.token).await
        .map_err(|e| format!("Failed XSTS auth: {}", e))?;
    let mc_token = authenticate_minecraft(&xsts_token).await
        .map_err(|e| format!("Failed Minecraft auth: {}", e))?;

    // 3) Compute new expiry and update DB session
    let new_expires_at = Utc::now().timestamp() + ms_token.expires_in as i64;
    let mut updated = existing_session.clone();
    updated.access_token = mc_token.access_token.clone();
    updated.refresh_token = ms_token.refresh_token.clone();
    updated.expires_at = new_expires_at;
    updated.updated_at = Utc::now().timestamp();

    session_manager.update_session(&updated)
        .map_err(|e| format!("Failed to update session: {}", e))?;

    Ok(updated)
}

// Authenticate with Xbox Live using Microsoft token
async fn authenticate_xbox_live(access_token: &str) -> Result<XboxLiveAuthResponse> {
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": &format!("d={}", access_token)
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let response = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("Xbox Live auth failed: {}", error_text));
    }

    let xbox_response: XboxLiveAuthResponse = response.json().await?;
    Ok(xbox_response)
}

// Authenticate with XSTS using Xbox Live token
async fn authenticate_xsts(xbox_token: &str) -> Result<XstsAuthResponse> {
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbox_token]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });

    let response = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("XSTS auth failed: {}", error_text));
    }

    let xsts_response: XstsAuthResponse = response.json().await?;
    Ok(xsts_response)
}

// Authenticate with Minecraft services using XSTS token
async fn authenticate_minecraft(xsts_response: &XstsAuthResponse) -> Result<MinecraftAuthResponse> {
    let client = reqwest::Client::new();

    // Extract user hash from XSTS response
    let user_hash = xsts_response
        .display_claims
        .get("xui")
        .and_then(|claims| claims.first())
        .and_then(|claim| claim.get("uhs"))
        .ok_or_else(|| anyhow::anyhow!("Failed to extract user hash from XSTS response"))?;

    let identity_token = format!("XBL3.0 x={};{}", user_hash, xsts_response.token);

    let payload = serde_json::json!({
        "identityToken": identity_token
    });

    let response = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("Minecraft auth failed: {}", error_text));
    }

    let mc_response: MinecraftAuthResponse = response.json().await?;
    Ok(mc_response)
}

// Get Minecraft profile using access token
#[tauri::command]
async fn get_minecraft_profile(access_token: String) -> Result<String, String> {
    let client = reqwest::Client::new();

    let response = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.map_err(|e| format!("Failed to read error response: {}", e))?;
        return Err(format!("Failed to get Minecraft profile: {}", error_text));
    }

    let response_text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;
    Ok(response_text)
}

// Safe profile fetch with structured error
#[derive(Serialize, Deserialize)]
struct ProfileResponse {
    status: String,
    profile: Option<serde_json::Value>,
    code: Option<String>,
    message: Option<String>,
}

#[tauri::command]
async fn get_minecraft_profile_safe(access_token: String) -> Result<ProfileResponse, String> {
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

    let json = response.json::<serde_json::Value>().await
        .map_err(|e| format!("Failed to parse profile json: {}", e))?;

    Ok(ProfileResponse { status: "Ok".into(), profile: Some(json), code: None, message: None })
}

// Validate access token by trying to get Minecraft profile
async fn validate_access_token(access_token: &str) -> Result<bool, String> {
    match get_minecraft_profile(access_token.to_string()).await {
        Ok(_) => Ok(true),
        Err(e) => {
            // Check if it's an authentication error (401) vs other errors
            if e.contains("401") || e.contains("Unauthorized") {
                Ok(false)
            } else {
                Err(e)
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
enum EnsureSessionResponse {
    Ok { session: sessions::Session, refreshed: bool },
    Err { code: String, message: String },
}

// Validate and refresh session if needed (structured response)
#[tauri::command]
async fn validate_and_refresh_token(app_handle: tauri::AppHandle, username: String) -> Result<EnsureSessionResponse, String> {
    let session_manager = sessions::SessionManager::new(&app_handle)
        .map_err(|e| format!("Failed to initialize session manager: {}", e))?;

    let existing = session_manager.get_session(&username)
        .map_err(|e| format!("Failed to get session: {}", e))?;

    let Some(mut session) = existing else {
        return Ok(EnsureSessionResponse::Err { code: "NO_SESSION".into(), message: "No existing session".into() });
    };

    // First try to validate current access token
    match validate_access_token(&session.access_token).await {
        Ok(true) => {
            session.updated_at = Utc::now().timestamp();
            session_manager.update_session(&session)
                .map_err(|e| format!("Failed to update session: {}", e))?;
            return Ok(EnsureSessionResponse::Ok { session, refreshed: false });
        },
        Ok(false) => {
            // Token is invalid, try to refresh
            if let Some(refresh_token) = session.refresh_token.clone() {
                match refresh_ms_token(refresh_token).await {
                    Ok(ms_token) => {
                        // Re-authenticate with Xbox Live / XSTS / Minecraft
                        match authenticate_xbox_live(&ms_token.access_token).await {
                            Ok(xbox_token) => {
                                match authenticate_xsts(&xbox_token.token).await {
                                    Ok(xsts_token) => {
                                        match authenticate_minecraft(&xsts_token).await {
                                            Ok(mc_token) => {
                                                // Update session with new tokens
                                                let new_expires_at = Utc::now().timestamp() + ms_token.expires_in as i64;
                                                session.access_token = mc_token.access_token;
                                                session.refresh_token = ms_token.refresh_token;
                                                session.expires_at = new_expires_at;
                                                session.updated_at = Utc::now().timestamp();

                                                session_manager.update_session(&session)
                                                    .map_err(|e| format!("Failed to update session: {}", e))?;

                                                return Ok(EnsureSessionResponse::Ok { session, refreshed: true });
                                            },
                                            Err(e) => return Ok(EnsureSessionResponse::Err { code: "MC_PROFILE".into(), message: e.to_string() })
                                        }
                                    },
                                    Err(e) => return Ok(EnsureSessionResponse::Err { code: "XSTS".into(), message: e.to_string() })
                                }
                            },
                            Err(e) => return Ok(EnsureSessionResponse::Err { code: "XBL".into(), message: e.to_string() })
                        }
                    },
                    Err(e) => return Ok(EnsureSessionResponse::Err { code: "REFRESH_FAILED".into(), message: e.to_string() })
                }
            } else {
                return Ok(EnsureSessionResponse::Err { code: "NO_REFRESH".into(), message: "No refresh token available".into() })
            }
        },
        Err(e) => return Ok(EnsureSessionResponse::Err { code: "NETWORK".into(), message: e })
    }
}

// Ensure valid session wrapper
#[tauri::command]
async fn ensure_valid_session(app_handle: tauri::AppHandle, username: String) -> Result<EnsureSessionResponse, String> {
    validate_and_refresh_token(app_handle, username).await
}

// Tauri command handlers
#[tauri::command]
async fn greet(name: String) -> String {
    format!("Hello, {}! Welcome to Kindly Klan Klient!", name)
}

// Get available Minecraft versions
#[tauri::command]
async fn get_versions() -> Result<Vec<MinecraftVersion>, String> {
    let launcher = MinecraftLauncher::new().map_err(|e| e.to_string())?;
    launcher.get_available_versions().await.map_err(|e| e.to_string())
}

// Launch Minecraft game with authentication
#[tauri::command]
async fn launch_game(version: String, session: AuthSession) -> Result<String, String> {
    let launcher = MinecraftLauncher::new().map_err(|e| e.to_string())?;
    launcher.config.ensure_directories().await.map_err(|e| e.to_string())?;

    let ram_mb = get_total_ram_mb().unwrap_or(4096);

    let version_dir = launcher.config.versions_dir.join(&version);
    let jar_path = version_dir.join(format!("{}.jar", version));

    let versions = launcher.get_available_versions().await.map_err(|e| e.to_string())?;

    if let Some(target_version) = versions.into_iter().find(|v| v.id == version) {
        // Check for critical asset files
        let assets_dir = launcher.config.assets_dir.join("objects");
        let missing_assets = [
            "5f/5ff04807c356f1beed0b86ccf659b44b9983e3fa",
            "b3/b3305151c36cc6e776f0130e85e8baee7ea06ec9",
            "b8/b84572b0d91367c41ff73b22edd5a2e9c02eab13",
            "40/402ded0eebd448033ef415e861a17513075f80e7",
            "89/89e4e7c845d442d308a6194488de8bd3397f0791"
        ];

        let need_download = !jar_path.exists() || missing_assets.iter().any(|asset_path| {
            !assets_dir.join(asset_path).exists()
        });

        if need_download {
            launcher.download_version(&target_version).await.map_err(|e| e.to_string())?;
        }
    } else {
        return Err("Version not found".to_string());
    }

    launcher.launch_minecraft(&version, &session.username, ram_mb, Some(&session.access_token), Some(&session.uuid)).await.map_err(|e| e.to_string())?;

    Ok("Minecraft launched successfully!".to_string())
}

// Instance distribution management commands
#[tauri::command]
async fn load_distribution_manifest(url: String) -> Result<DistributionManifest, String> {
    let client = reqwest::Client::new();
    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch manifest: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let manifest: DistributionManifest = response.json()
        .await
        .map_err(|e| format!("Failed to parse manifest JSON: {}", e))?;

    Ok(manifest)
}

// Get instance manifest details
#[tauri::command]
async fn get_instance_details(base_url: String, instance_url: String) -> Result<InstanceManifest, String> {
    let full_url = if instance_url.starts_with("http") {
        instance_url
    } else {
        format!("{}/{}", base_url.trim_end_matches('/'), instance_url.trim_start_matches('/'))
    };

    let client = reqwest::Client::new();
    let response = client.get(&full_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch instance details: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let instance: InstanceManifest = response.json()
        .await
        .map_err(|e| format!("Failed to parse instance JSON: {}", e))?;

    Ok(instance)
}

// Download and setup Minecraft instance with mods and configs
#[tauri::command]
async fn download_instance(
    base_url: String,
    instance: InstanceManifest,
    _session: AuthSession
) -> Result<String, String> {
    let launcher = MinecraftLauncher::new().map_err(|e| e.to_string())?;
    launcher.config.ensure_directories().await.map_err(|e| e.to_string())?;

    let instance_dir = launcher.config.versions_dir.join(&instance.instance.id);
    fs::create_dir_all(&instance_dir).await.map_err(|e| e.to_string())?;

    // Download Minecraft version first
    let versions = launcher.get_available_versions().await.map_err(|e| e.to_string())?;

    if let Some(mc_version) = versions.into_iter().find(|v| v.id == instance.instance.minecraft_version) {
        launcher.download_version(&mc_version).await.map_err(|e| e.to_string())?;
    } else {
        return Err(format!("Minecraft version {} not found", instance.instance.minecraft_version));
    }

    // TODO: Download mod loader if specified
    if let Some(_mod_loader) = &instance.instance.mod_loader {
        // Implementation for mod loader installation will go here
    }

    // Download mods
    for mod_file in &instance.files.mods {
        let file_url = if mod_file.url.starts_with("http") {
            mod_file.url.clone()
        } else {
            format!("{}/{}", base_url.trim_end_matches('/'), mod_file.url.trim_start_matches('/'))
        };

        let target_path = launcher.config.minecraft_dir
            .join("instances")
            .join(&instance.instance.id)
            .join("mods")
            .join(&mod_file.name);

        fs::create_dir_all(target_path.parent().unwrap()).await.map_err(|e| e.to_string())?;
        download_file(&file_url, &target_path).await.map_err(|e| e.to_string())?;
    }

    // Download configs
    for config_file in &instance.files.configs {
        let file_url = if config_file.url.starts_with("http") {
            config_file.url.clone()
        } else {
            format!("{}/{}", base_url.trim_end_matches('/'), config_file.url.trim_start_matches('/'))
        };

        let target_path = launcher.config.minecraft_dir
            .join("instances")
            .join(&instance.instance.id)
            .join(config_file.target.as_ref().unwrap_or(&config_file.path));

        fs::create_dir_all(target_path.parent().unwrap()).await.map_err(|e| e.to_string())?;
        download_file(&file_url, &target_path).await.map_err(|e| e.to_string())?;
    }

    Ok(format!("Instance {} ready to launch!", instance.instance.name))
}


// Determine required Java version based on Minecraft version
fn get_required_java_version(minecraft_version: &str) -> String {
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

// Check if specific Java version is installed
#[tauri::command]
async fn check_java_version(version: String) -> Result<String, String> {
    let kindly_dir = std::env::var("USERPROFILE")
        .map(|p| std::path::Path::new(&p).join(".kindlyklanklient"))
        .unwrap_or_else(|_| std::path::Path::new(".").join(".kindlyklanklient"));

    let java_dir = kindly_dir.join("runtime").join(format!("java-{}", version));

    let java_path = if cfg!(target_os = "windows") {
        java_dir.join("bin").join("java.exe")
    } else {
        java_dir.join("bin").join("java")
    };

    if java_path.exists() {
        Ok("installed".to_string())
    } else {
        Ok("not_installed".to_string())
    }
}

// Download and install Java runtime for the specified version
#[tauri::command]
async fn download_java(version: String) -> Result<String, String> {
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    let kindly_dir = std::env::var("USERPROFILE")
        .map(|p| Path::new(&p).join(".kindlyklanklient"))
        .unwrap_or_else(|_| Path::new(".").join(".kindlyklanklient"));

    let runtime_dir = kindly_dir.join("runtime");
    let java_dir = runtime_dir.join(format!("java-{}", version));

    tokio::fs::create_dir_all(&runtime_dir).await
        .map_err(|e| format!("Failed to create runtime directory: {}", e))?;

    let (os, arch, extension) = if cfg!(target_os = "windows") {
        ("windows", "x64", "zip")
    } else if cfg!(target_os = "macos") {
        ("mac", "x64", "tar.gz")
    } else {
        ("linux", "x64", "tar.gz")
    };

    let jre_url = format!(
        "https://api.adoptium.net/v3/binary/latest/{}/ga/{}/{}/jdk/hotspot/normal/eclipse",
        version, os, arch
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&jre_url)
        .header("User-Agent", "KindlyKlanKlient/1.0")
        .header("Accept", "application/octet-stream")
        .send()
        .await
        .map_err(|e| format!("Failed to download Java: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    let bytes = response.bytes().await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let temp_file = runtime_dir.join(format!("java-{}.{}", version, extension));

    let mut file = File::create(&temp_file)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    file.write_all(&bytes)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;

    file.flush().map_err(|e| format!("Failed to flush file: {}", e))?;
    drop(file);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    if java_dir.exists() {
        let _ = std::fs::remove_dir_all(&java_dir);
    }

    // Extract archive natively without spawning console windows
    if temp_file.extension().map_or(false, |e| e == "zip") {
        // Windows ZIP
        let reader = std::fs::File::open(&temp_file).map_err(|e| format!("Open zip failed: {}", e))?;
        let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("Read zip failed: {}", e))?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| format!("Zip index failed: {}", e))?;
            let outpath = runtime_dir.join(file.mangled_name());
            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath).map_err(|e| format!("Create dir failed: {}", e))?;
            } else {
                if let Some(p) = outpath.parent() { std::fs::create_dir_all(p).map_err(|e| format!("Create parent failed: {}", e))?; }
                let mut outfile = std::fs::File::create(&outpath).map_err(|e| format!("Create file failed: {}", e))?;
                std::io::copy(&mut file, &mut outfile).map_err(|e| format!("Write file failed: {}", e))?;
            }
        }
    } else {
        // tar.gz (Unix) – intentar extraer con la crate `zip` no aplica; para simplificar, devolver error claro
        #[cfg(not(target_os = "windows"))]
        {
            return Err("Unsupported archive format on this OS without external tools".to_string());
        }
    }

    // Find and move extracted directory
    let all_entries = std::fs::read_dir(&runtime_dir)
        .map_err(|e| format!("Failed to read runtime directory: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to read directory entries: {}", e))?;

    let extracted_dirs: Vec<_> = all_entries
        .into_iter()
        .filter(|entry| {
            let path = entry.path();
            path.is_dir() && path != java_dir
        })
        .map(|entry| entry.path())
        .collect();

    if let Some(extracted_dir) = extracted_dirs.first() {
        if java_dir.exists() {
            let _ = std::fs::remove_dir_all(&java_dir);
        }

        std::fs::rename(extracted_dir, &java_dir)
            .map_err(|e| format!("Failed to move Java directory: {}", e))?;

        // Remove any other extracted directories
        for dir in extracted_dirs.iter().skip(1) {
            let _ = std::fs::remove_dir_all(dir);
        }
    } else {
        return Err("No Java directory found after extraction".to_string());
    }

    let _ = std::fs::remove_file(&temp_file);

    Ok(format!("Java {} downloaded and installed successfully", version))
}

// Get path to installed Java executable
#[tauri::command]
async fn get_java_path(version: String) -> Result<String, String> {
    let kindly_dir = std::env::var("USERPROFILE")
        .map(|p| std::path::Path::new(&p).join(".kindlyklanklient"))
        .unwrap_or_else(|_| std::path::Path::new(".").join(".kindlyklanklient"));

    let java_dir = kindly_dir.join("runtime").join(format!("java-{}", version));

    let java_path = if cfg!(target_os = "windows") {
        java_dir.join("bin").join("java.exe")
    } else {
        java_dir.join("bin").join("java")
    };

    if java_path.exists() {
        Ok(java_path.to_string_lossy().to_string())
    } else {
        Err(format!("Java executable not found at: {}", java_path.display()))
    }
}

// Create instance and Java directories
#[tauri::command]
async fn create_instance_directory(instance_id: String, java_version: String) -> Result<String, String> {
    let kindly_dir = std::env::var("USERPROFILE")
        .map(|p| std::path::Path::new(&p).join(".kindlyklanklient"))
        .unwrap_or_else(|_| std::path::Path::new(".").join(".kindlyklanklient"));

    let instance_dir = kindly_dir.join(&instance_id);
    let runtime_dir = kindly_dir.join("runtime");
    let java_dir = runtime_dir.join(format!("java-{}", java_version));

    tokio::fs::create_dir_all(&instance_dir).await
        .map_err(|e| format!("Failed to create instance directory: {}", e))?;
    tokio::fs::create_dir_all(&java_dir).await
        .map_err(|e| format!("Failed to create Java directory: {}", e))?;

    Ok(format!("Instance directory created: {}", instance_dir.display()))
}

// Get required Java version for Minecraft version
#[tauri::command]
async fn get_required_java_version_command(minecraft_version: String) -> Result<String, String> {
    Ok(get_required_java_version(&minecraft_version))
}

// Stop Minecraft instance (placeholder implementation)
#[tauri::command]
async fn stop_minecraft_instance(instance_id: String) -> Result<String, String> {
    Ok(format!("Minecraft instance {} stopped", instance_id))
}

// Restart application (placeholder implementation)
#[tauri::command]
async fn restart_application() -> Result<String, String> {
    Ok("Application will be restarted".to_string())
}

// Upload skin to Mojang profile API
#[tauri::command]
async fn upload_skin_to_mojang(file_path: String, variant: String, access_token: String) -> Result<String, String> {
    use std::fs;

    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    if path.extension().unwrap_or_default() != "png" {
        return Err("File must be a PNG image".to_string());
    }

    let file_data = fs::read(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    if file_data.len() > 24 * 1024 {
        return Err("Skin file must be smaller than 24KB".to_string());
    }

    let client = reqwest::Client::new();

    let form = reqwest::multipart::Form::new()
        .part("file", reqwest::multipart::Part::bytes(file_data)
            .file_name("skin.png")
            .mime_str("image/png").unwrap())
        .text("variant", variant);

    let response = client
        .post("https://api.minecraftservices.com/minecraft/profile/skins")
        .header("Authorization", format!("Bearer {}", access_token))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Failed to upload skin: {}", e))?;

    let status = response.status();
    let response_text = response.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("Mojang API error ({}): {}", status, response_text));
    }

    Ok("Skin uploaded successfully".to_string())
}

// Set skin variant on Mojang profile
#[tauri::command]
async fn set_skin_variant(file_path: String, variant: String, access_token: String) -> Result<String, String> {
    use std::fs;

    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    if path.extension().unwrap_or_default() != "png" {
        return Err("File must be a PNG image".to_string());
    }

    let file_data = fs::read(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    if file_data.len() > 24 * 1024 {
        return Err("Skin file must be smaller than 24KB".to_string());
    }

    let client = reqwest::Client::new();

    let form = reqwest::multipart::Form::new()
        .part("file", reqwest::multipart::Part::bytes(file_data)
            .file_name("skin.png")
            .mime_str("image/png").unwrap())
        .text("variant", variant.clone());

    let response = client
        .post("https://api.minecraftservices.com/minecraft/profile/skins")
        .header("Authorization", format!("Bearer {}", access_token))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Failed to upload skin: {}", e))?;

    let status = response.status();
    let response_text = response.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("Mojang API error ({}): {}", status, response_text));
    }

    Ok("Skin variant updated".to_string())
}

// Create temporary file with provided data
#[tauri::command]
async fn create_temp_file(file_name: String, file_data: Vec<u8>) -> Result<String, String> {
    use std::fs::File;
    use std::io::Write;

    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(&file_name);

    let mut file = File::create(&file_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    file.write_all(&file_data)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
}

// Check for available application updates
#[tauri::command]
async fn check_for_updates(app_handle: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app_handle.updater().map_err(|e| format!("Failed to get updater: {}", e))?;
    
    // Load current state
    let mut state = load_update_state().await;
    state.last_check = chrono::Utc::now().to_rfc3339();
    
    match updater.check().await {
        Ok(update) => {
            if let Some(update) = update {
                // Update state with available version
                state.available_version = Some(update.version.clone());
                state.downloaded = false;
                state.download_ready = false;
                save_update_state(&state).await?;
                
                Ok(format!("Update available: {}", update.version))
            } else {
                // No updates available
                state.available_version = None;
                state.downloaded = false;
                state.download_ready = false;
                save_update_state(&state).await?;
                
                Ok("No updates available".to_string())
            }
        }
        Err(e) => {
            // Save state even on error
            save_update_state(&state).await?;
            Err(format!("Failed to check for updates: {}", e))
        }
    }
}

// Install available application update
#[tauri::command]
async fn install_update(app_handle: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_updater::UpdaterExt;

    // Check if we have a downloaded update ready
    let state = load_update_state().await;
    if !state.download_ready {
        return Ok("No update ready to install. Please download the update first.".to_string());
    }

    let updater = app_handle.updater().map_err(|e| format!("Failed to get updater: {}", e))?;
    match updater.check().await {
        Ok(update) => {
            if let Some(update) = update {
                // Emit install start event
                app_handle.emit("update-install-start", ()).unwrap_or_default();
                
                update.download_and_install(
                    |chunk_length, content_length| {
                        println!("Installing update: {} of {:?}", chunk_length, content_length);
                    },
                    || {
                        println!("Install finished");
                        // Emit install complete event
                        app_handle.emit("update-install-complete", ()).unwrap_or_default();
                    }
                ).await.map_err(|e| format!("Failed to install update: {}", e))?;
                
                // Clear update state after successful install and update current version
                let new_state = UpdateState {
                    last_check: chrono::Utc::now().to_rfc3339(),
                    available_version: None,
                    current_version: update.version.clone(), 
                    downloaded: false,
                    download_ready: false,
                };
                save_update_state(&new_state).await?;
                
                Ok("Update installed successfully. The application will restart.".to_string())
            } else {
                Ok("No updates available to install".to_string())
            }
        }
        Err(e) => Err(format!("Failed to check for updates: {}", e))
    }
}

// Helper function to get update state file path
fn get_update_state_path() -> PathBuf {
    let kindly_dir = std::env::var("USERPROFILE")
        .map(|p| std::path::Path::new(&p).join(".kindlyklanklient"))
        .unwrap_or_else(|_| std::path::Path::new(".").join(".kindlyklanklient"));
    
    kindly_dir.join("update_state.json")
}

// Removed: get_current_version_from_github - now using env!("CARGO_PKG_VERSION") directly

async fn load_update_state() -> UpdateState {
    let state_path = get_update_state_path();

    if let Ok(content) = fs::read_to_string(&state_path).await {
        if let Ok(state) = serde_json::from_str::<UpdateState>(&content) {
            return state;
        }
    }

    UpdateState {
        last_check: "1970-01-01T00:00:00Z".to_string(),
        available_version: None,
        current_version: env!("CARGO_PKG_VERSION").to_string(), 
        downloaded: false,
        download_ready: false,
    }
}

// Helper function to save update state
async fn save_update_state(state: &UpdateState) -> Result<(), String> {
    let state_path = get_update_state_path();
    
    // Ensure directory exists
    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent).await
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    
    let content = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Failed to serialize state: {}", e))?;
    
    fs::write(&state_path, content).await
        .map_err(|e| format!("Failed to write state file: {}", e))?;
    
    Ok(())
}

// Get current update state
#[tauri::command]
async fn get_update_state() -> Result<UpdateState, String> {
    Ok(load_update_state().await)
}

// Save update state
#[tauri::command]
async fn save_update_state_command(state: UpdateState) -> Result<String, String> {
    save_update_state(&state).await?;
    Ok("Update state saved successfully".to_string())
}

// Download update silently (without installing)
#[tauri::command]
async fn download_update_silent(app_handle: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app_handle.updater().map_err(|e| format!("Failed to get updater: {}", e))?;

    match updater.check().await {
        Ok(update) => {
            if let Some(update) = update {
                // Emit download start event
                app_handle.emit("update-download-start", ()).unwrap_or_default();
                
                // Download the update
                update.download_and_install(
                    |chunk_length, content_length| {
                        let progress = if let Some(total) = content_length {
                            (chunk_length as f64 / total as f64 * 100.0) as u32
                        } else {
                            0
                        };
                        
                        // Emit progress event
                        app_handle.emit("update-download-progress", progress).unwrap_or_default();
                    },
                    || {
                        // Emit download complete event
                        app_handle.emit("update-download-complete", ()).unwrap_or_default();
                    }
                ).await.map_err(|e| format!("Failed to download update: {}", e))?;
                
                // Update state
                let mut state = load_update_state().await;
                state.downloaded = true;
                state.download_ready = true;
                state.available_version = Some(update.version.clone());
                save_update_state(&state).await?;

                Ok(format!("Update {} downloaded successfully", update.version))
            } else {
                Ok("No updates available to download".to_string())
            }
        }
        Err(e) => Err(format!("Failed to check for updates: {}", e))
    }
}

// Whitelist functions
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::LazyLock;

// Cache for whitelist entries (5 minutes TTL)
static WHITELIST_CACHE: LazyLock<Mutex<HashMap<String, (AccessCheck, u64)>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

// Get current timestamp
fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// Check if cache entry is still valid (5 minutes = 300 seconds)
fn is_cache_valid(timestamp: u64) -> bool {
    get_current_timestamp() - timestamp < 300
}

// Get Supabase configuration (uses compile-time embedded values)
fn get_supabase_config() -> (String, String) {
    // First try runtime environment (for development)
    let url = std::env::var("SUPABASE_URL")
        .unwrap_or_else(|_| env!("SUPABASE_URL").to_string());
    let key = std::env::var("SUPABASE_ANON_KEY")
        .unwrap_or_else(|_| env!("SUPABASE_ANON_KEY").to_string());
    
    // Log configuration status (without exposing keys)
    if url == "https://your-project.supabase.co" || key == "your-anon-key" {
        log::warn!("⚠️  Supabase not configured - using fallback values");
        log::warn!("   Configure SUPABASE_URL and SUPABASE_ANON_KEY environment variables");
    } else {
        log::info!("✅ Supabase configured successfully");
    }
    
    (url, key)
}

// Check whitelist access for a username
#[tauri::command]
async fn check_whitelist_access(username: String) -> Result<AccessCheck, String> {
    log::info!("🔍 Checking whitelist access for user: {}", username);
    
    let (supabase_url, supabase_key) = get_supabase_config();
    
    // If Supabase is not configured, allow access by default (for development)
    if supabase_url == "https://your-project.supabase.co" || supabase_key == "your-anon-key" {
        log::warn!("⚠️  Whitelist disabled - allowing access for user: {}", username);
        return Ok(AccessCheck {
            has_access: true,
            allowed_instances: Vec::new(),
            global_access: true,
        });
    }

    // Check cache first
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
    
    // Query Supabase API
    let client = reqwest::Client::new();
    let url = format!("{}/rest/v1/whitelist?minecraft_username=eq.{}", supabase_url, username);
    
    log::info!("🔗 Supabase URL: {}", url);
    log::info!("🔑 Using API key (length: {})", supabase_key.len());
    
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
        // User not in whitelist
        log::warn!("❌ User not found in whitelist: {}", username);
        AccessCheck {
            has_access: false,
            allowed_instances: Vec::new(),
            global_access: false,
        }
    } else {
        let entry = &entries[0];
        log::info!("✅ User found in whitelist: {}", username);
        log::info!("   Global access: {}", entry.global_access);
        log::info!("   Allowed instances: {:?}", entry.allowed_instances);
        AccessCheck {
            has_access: true,
            allowed_instances: entry.allowed_instances.clone().unwrap_or_default(),
            global_access: entry.global_access,
        }
    };

    // Cache the result
    {
        let mut cache = WHITELIST_CACHE.lock().unwrap();
        cache.insert(username, (result.clone(), get_current_timestamp()));
    }

    Ok(result)
}

// Get accessible instances for a user
#[tauri::command]
async fn get_accessible_instances(
    username: String,
    all_instances: Vec<String>
) -> Result<Vec<String>, String> {
    let access_check = check_whitelist_access(username).await?;
    
    if !access_check.has_access {
        return Ok(Vec::new());
    }

    if access_check.global_access {
        // User has access to all instances
        Ok(all_instances)
    } else {
        // User has access only to specific instances
        let accessible: Vec<String> = all_instances
            .into_iter()
            .filter(|instance| access_check.allowed_instances.contains(instance))
            .collect();
        Ok(accessible)
    }
}

// Clear whitelist cache
#[tauri::command]
async fn clear_whitelist_cache() -> Result<String, String> {
    let mut cache = WHITELIST_CACHE.lock().unwrap();
    cache.clear();
    Ok("Whitelist cache cleared".to_string())
}

// Open URL in default browser
#[tauri::command]
async fn open_url(url: String) -> Result<String, String> {
    open::that(&url).map_err(|e| format!("Failed to open URL: {}", e))?;
    Ok("URL opened successfully".to_string())
}

#[tauri::command]
async fn debug_env_vars() -> Result<String, String> {
    let (url, key) = get_supabase_config();
    let runtime_url = std::env::var("SUPABASE_URL").unwrap_or_else(|_| "NOT SET".to_string());
    let runtime_key = std::env::var("SUPABASE_ANON_KEY").unwrap_or_else(|_| "NOT SET".to_string());
    
    let result = format!(
        "Environment Variables Debug:\n\
        Runtime SUPABASE_URL: {}\n\
        Runtime SUPABASE_ANON_KEY: {}\n\
        Compile-time SUPABASE_URL: {}\n\
        Compile-time SUPABASE_ANON_KEY: {}\n\
        Final URL: {}\n\
        Final Key: {}\n\
        URL contains supabase.co: {}\n\
        Key length: {}\n\
        Is production build: {}\n\
        Current working directory: {:?}",
        runtime_url,
        if runtime_key.len() > 20 { "SET (length > 20)" } else { "NOT SET or too short" },
        env!("SUPABASE_URL"),
        if env!("SUPABASE_ANON_KEY").len() > 20 { "SET (length > 20)" } else { "NOT SET or too short" },
        url,
        if key.len() > 20 { "SET (length > 20)" } else { "NOT SET or too short" },
        url.contains("supabase.co"),
        key.len(),
        !cfg!(debug_assertions),
        std::env::current_dir().unwrap_or_default()
    );
    
    log::info!("🔍 Environment debug info:\n{}", result);
    Ok(result)
}

// Session management commands
#[tauri::command]
async fn save_session(
    app_handle: tauri::AppHandle,
    username: String,
    access_token: String,
    refresh_token: Option<String>,
    expires_at: i64
) -> Result<String, String> {
    let session_manager = sessions::SessionManager::new(&app_handle)
        .map_err(|e| format!("Failed to initialize session manager: {}", e))?;

    let session = sessions::Session::new(username.clone(), access_token, refresh_token, expires_at);
    session_manager.save_session(&session)
        .map_err(|e| format!("Failed to save session: {}", e))?;

    log::info!("Session saved for user: {}", username);
    Ok("Session saved successfully".to_string())
}

#[tauri::command]
async fn get_session(app_handle: tauri::AppHandle, username: String) -> Result<Option<sessions::Session>, String> {
    let session_manager = sessions::SessionManager::new(&app_handle)
        .map_err(|e| format!("Failed to initialize session manager: {}", e))?;

    let session = session_manager.get_session(&username)
        .map_err(|e| format!("Failed to get session: {}", e))?;

    Ok(session)
}

#[tauri::command]
async fn get_active_session(app_handle: tauri::AppHandle) -> Result<Option<sessions::Session>, String> {
    let session_manager = sessions::SessionManager::new(&app_handle)
        .map_err(|e| format!("Failed to initialize session manager: {}", e))?;

    let session = session_manager.get_active_session()
        .map_err(|e| format!("Failed to get active session: {}", e))?;

    Ok(session)
}

#[tauri::command]
async fn update_session(
    app_handle: tauri::AppHandle,
    session: sessions::Session
) -> Result<String, String> {
    let session_manager = sessions::SessionManager::new(&app_handle)
        .map_err(|e| format!("Failed to initialize session manager: {}", e))?;

    session_manager.update_session(&session)
        .map_err(|e| format!("Failed to update session: {}", e))?;

    log::info!("Session updated for user: {}", session.username);
    Ok("Session updated successfully".to_string())
}

#[tauri::command]
async fn delete_session(app_handle: tauri::AppHandle, username: String) -> Result<String, String> {
    let session_manager = sessions::SessionManager::new(&app_handle)
        .map_err(|e| format!("Failed to initialize session manager: {}", e))?;

    session_manager.delete_session(&username)
        .map_err(|e| format!("Failed to delete session: {}", e))?;

    log::info!("Session deleted for user: {}", username);
    Ok("Session deleted successfully".to_string())
}

#[tauri::command]
async fn clear_all_sessions(app_handle: tauri::AppHandle) -> Result<String, String> {
    let session_manager = sessions::SessionManager::new(&app_handle)
        .map_err(|e| format!("Failed to initialize session manager: {}", e))?;

    session_manager.clear_all_sessions()
        .map_err(|e| format!("Failed to clear sessions: {}", e))?;

    log::info!("All sessions cleared");
    Ok("All sessions cleared successfully".to_string())
}

#[tauri::command]
async fn cleanup_expired_sessions(app_handle: tauri::AppHandle) -> Result<usize, String> {
    let session_manager = sessions::SessionManager::new(&app_handle)
        .map_err(|e| format!("Failed to initialize session manager: {}", e))?;

    let cleaned = session_manager.cleanup_expired_sessions()
        .map_err(|e| format!("Failed to cleanup sessions: {}", e))?;

    log::info!("Cleaned up {} expired sessions", cleaned);
    Ok(cleaned)
}

// Debug session database
#[tauri::command]
async fn debug_sessions(app_handle: tauri::AppHandle) -> Result<String, String> {
    let session_manager = sessions::SessionManager::new(&app_handle)
        .map_err(|e| format!("Failed to initialize session manager: {}", e))?;

    let db_path = session_manager.db_path.clone();
    let sessions = session_manager.get_all_sessions()
        .map_err(|e| format!("Failed to get sessions: {}", e))?;

    let result = format!(
        "Session Database Debug:\n\
        Database path: {:?}\n\
        Total sessions: {}\n\
        Sessions:\n{}",
        db_path,
        sessions.len(),
        sessions.iter().map(|s| format!(
            "  - {}: expires_at={}, is_expired={}",
            s.username,
            s.expires_at,
            s.is_expired()
        )).collect::<Vec<_>>().join("\n")
    );

    log::info!("🔍 Session debug info:\n{}", result);
    Ok(result)
}

// Expose session database path for debugging
#[tauri::command]
async fn get_db_path(app_handle: tauri::AppHandle) -> Result<String, String> {
    let manager = sessions::SessionManager::new(&app_handle)
        .map_err(|e| format!("Failed to initialize session manager: {}", e))?;
    Ok(manager.db_path.to_string_lossy().to_string())
}
#[tauri::command]
async fn clear_update_state() -> Result<String, String> {
    let state_path = get_update_state_path();

    if state_path.exists() {
        fs::remove_file(&state_path).await
            .map_err(|e| format!("Failed to remove update state file: {}", e))?;
        Ok("Update state cleared".to_string())
    } else {
        Ok("No update state file to clear".to_string())
    }
}

// Test manifest URL accessibility
#[tauri::command]
async fn test_manifest_url(
    distribution_url: String,
    instance_id: String
) -> Result<String, String> {
    let base_url = build_distribution_url(&distribution_url);
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

// Download instance assets and install mod loader if needed
#[tauri::command]
async fn download_instance_assets(
    app_handle: tauri::AppHandle,
    instance_id: String,
    distribution_url: String
) -> Result<String, String> {
    let _ = app_handle.emit(
        "asset-download-progress",
        serde_json::json!({
            "current": 0u64,
            "total": 100u64,
            "percentage": 0.0,
            "current_file": "",
            "status": "Initializing"
        })
    );

    let instance_dir = create_instance_directory_safe(&instance_id, &app_handle).await?;
    let instance_manifest = load_instance_manifest(&distribution_url, &instance_id).await?;
    let checksums = load_checksums(&distribution_url, &instance_id).await?;

    ensure_minecraft_client_present(&instance_dir, &instance_manifest.instance.minecraft_version).await?;

    let instance_files_total = count_instance_files(&instance_manifest) as u64;
    let mojang_pending_total = count_mojang_assets_pending(&instance_dir, &instance_manifest.instance.minecraft_version).await? as u64;
    let combined_total = instance_files_total + mojang_pending_total;
    let combined_current = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

    let _ = app_handle.emit(
        "asset-download-progress",
        serde_json::json!({
            "current": 0u64,
            "total": combined_total,
            "percentage": if combined_total==0 { 100.0 } else { 0.0 },
            "current_file": "",
            "status": "Preparing"
        })
    );

    download_all_assets(
        &app_handle,
        &instance_manifest,
        &checksums,
        &instance_dir,
        &distribution_url,
        Some((combined_current.clone(), combined_total))
    ).await?;

    if let Some(mod_loader) = &instance_manifest.instance.mod_loader {
        install_mod_loader(&instance_manifest.instance.minecraft_version, mod_loader, &instance_dir).await?;
    }

    let _asset_index_id = ensure_assets_present_with_progress(
        &app_handle,
        &instance_dir,
        &instance_manifest.instance.minecraft_version,
        Some((combined_current.clone(), combined_total))
    ).await?;

    let _ = app_handle.emit("asset-download-completed", serde_json::json!({ "phase": "all" }));

    Ok("All assets and mod loader installed successfully".to_string())
}

// Load instance manifest from distribution URL
async fn load_instance_manifest(distribution_url: &str, instance_id: &str) -> Result<InstanceManifest, String> {
    let base_url = build_distribution_url(distribution_url);
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

// Load checksums for file integrity verification
async fn load_checksums(distribution_url: &str, instance_id: &str) -> Result<HashMap<String, String>, String> {
    let base_url = build_distribution_url(distribution_url);
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

// Download all instance assets with smart skip logic
async fn download_all_assets(
    app_handle: &tauri::AppHandle,
    manifest: &InstanceManifest,
    checksums: &HashMap<String, String>,
    instance_dir: &Path,
    distribution_url: &str,
    combined: Option<(std::sync::Arc<std::sync::atomic::AtomicU64>, u64)>
) -> Result<(), String> {
    let mut all_assets = Vec::new();

    // Collect all files to download
    for mod_file in &manifest.files.mods {
        all_assets.push(create_asset_from_file_entry(mod_file, &manifest.instance.id, distribution_url));
    }

    for config_file in &manifest.files.configs {
        all_assets.push(create_asset_from_file_entry(config_file, &manifest.instance.id, distribution_url));
    }

    if let Some(resourcepacks) = &manifest.files.resourcepacks {
        for rp_file in resourcepacks {
            all_assets.push(create_asset_from_file_entry(rp_file, &manifest.instance.id, distribution_url));
        }
    }

    if let Some(shaderpacks) = &manifest.files.shaderpacks {
        for sp_file in shaderpacks {
            all_assets.push(create_asset_from_file_entry(sp_file, &manifest.instance.id, distribution_url));
        }
    }

    let total = if let Some((_, t)) = &combined { *t } else { all_assets.len() as u64 };
    let progress = if let Some((p, _)) = &combined { p.clone() } else { std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)) };
    let client_app = app_handle.clone();
    let client_checksums = checksums.clone();

    use futures_util::{stream, StreamExt};
    let parallel = (num_cpus::get().saturating_sub(1)).max(4);
    stream::iter(all_assets.into_iter())
        .map(|asset| {
            let progress = progress.clone();
            let client_app = client_app.clone();
            let instance_dir = instance_dir.to_path_buf();
            let client_checksums = client_checksums.clone();
            async move {
                // Emit starting file
                let _ = client_app.emit(
                    "asset-download-progress",
                    serde_json::json!({
                        "current": progress.load(std::sync::atomic::Ordering::Relaxed),
                        "total": total,
                        "percentage": ((progress.load(std::sync::atomic::Ordering::Relaxed) as f32 / total as f32) * 100.0).min(100.0),
                        "current_file": asset.name,
                        "status": "Instance"
                    })
                );

                let file_path = get_local_file_path(&instance_dir, &asset.path)?;
                if let Some(parent) = file_path.parent() {
                    tokio::fs::create_dir_all(parent).await
                        .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
                }

                // Smart skip logic: MD5 -> size -> SHA256
                let mut should_skip = false;
                if file_path.exists() {
                    if let Some(expected_md5) = &asset.md5 {
                        if verify_file_md5(&file_path, expected_md5).is_ok() {
                            should_skip = true;
                        }
                    } else if let Some(expected_size) = asset.size {
                        if let Ok(meta) = std::fs::metadata(&file_path) {
                            if meta.len() == expected_size { should_skip = true; }
                        }
                    } else if let Some(expected) = client_checksums.get(&asset.path) {
                        if verify_file_checksum(&file_path, expected).is_ok() {
                            should_skip = true;
                        }
                    }
                }

                if !should_skip {
                    download_file_with_retry(&asset.url, &file_path).await?;
                    if let Some(checksum) = client_checksums.get(&asset.path) {
                        let _ = verify_file_checksum(&file_path, checksum);
                    }
                } else {
                    let cur = progress.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    let _ = client_app.emit(
                        "asset-download-progress",
                        serde_json::json!({
                            "current": cur,
                            "total": total,
                            "percentage": ((cur as f32 / total as f32) * 100.0).min(100.0),
                            "current_file": asset.name,
                            "status": "Instance"
                        })
                    );
                    return Ok(())
                }

                let cur = progress.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                let _ = client_app.emit(
                    "asset-download-progress",
                    serde_json::json!({
                        "current": cur,
                        "total": total,
                        "percentage": ((cur as f32 / total as f32) * 100.0).min(100.0),
                        "current_file": asset.name,
                        "status": "Instance"
                    })
                );
                Ok::<(), String>(())
            }
        })
        .buffer_unordered(parallel)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    Ok(())
}

// Create asset object from file entry with proper URL resolution
fn create_asset_from_file_entry(file_entry: &FileEntry, instance_id: &str, distribution_url: &str) -> InstanceAsset {
    let base_url = build_distribution_url(distribution_url);

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

// Map manifest file paths to local instance structure
fn get_local_file_path(instance_dir: &Path, file_path: &str) -> Result<PathBuf, String> {
    let normalized = file_path.trim_start_matches('/');
    let without_files = if normalized.starts_with("files/") { &normalized[6..] } else { normalized };

    let mut parts: Vec<&str> = without_files.split('/').collect();
    if parts.is_empty() {
        return Err(format!("Invalid file path: {}", file_path));
    }

    let file_name = parts.last().copied().unwrap_or("");

    // Special handling for root-level config files
    if (without_files.starts_with("config/") || without_files.starts_with("config/config/"))
        && (file_name.eq_ignore_ascii_case("options.txt") || file_name.eq_ignore_ascii_case("servers.dat"))
    {
        return Ok(instance_dir.join(file_name));
    }

    // Collapse config/config -> config
    if parts.len() >= 2 && parts[0] == "config" && parts[1] == "config" {
        parts.remove(1);
    }

    let target_path = PathBuf::from(parts.join("/"));
    Ok(instance_dir.join(target_path))
}

// Download file with atomic write using temporary file
async fn download_file(url: &str, file_path: &Path) -> Result<(), String> {
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
        .map_err(|e| format!("Failed to move temp file into place {} -> {}: {}", tmp_path.display(), file_path.display(), e))?;

    Ok(())
}

// Download file with retry logic
async fn download_file_with_retry(url: &str, file_path: &Path) -> Result<(), String> {
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



// Verify file SHA256 checksum
fn verify_file_checksum(file_path: &Path, expected_sha256: &str) -> Result<(), String> {
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

// Verify file MD5 hash
fn verify_file_md5(file_path: &Path, expected_md5: &str) -> Result<(), String> {
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

// Get instance directory path
fn get_instance_directory(instance_id: &str) -> PathBuf {
    let mut data_dir = if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
    } else if let Ok(home) = std::env::var("USERPROFILE") {
        PathBuf::from(home)
    } else {
        PathBuf::from(".")
    };

    data_dir.push(".kindlyklanklient");
    data_dir.push(instance_id);
    data_dir
}

// Create instance directory with proper permissions
async fn create_instance_directory_safe(instance_id: &str, _app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
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

// Install mod loader based on type and version
async fn install_mod_loader(minecraft_version: &str, mod_loader: &ModLoader, instance_dir: &Path) -> Result<(), String> {
    match mod_loader.r#type.as_str() {
        "fabric" => install_fabric(minecraft_version, &mod_loader.version, instance_dir).await,
        "neoforge" => install_neoforge(minecraft_version, &mod_loader.version, instance_dir).await,
        "vanilla" => Ok(()),
        _ => Err(format!("Unsupported mod loader type: {}", mod_loader.r#type))
    }
}

async fn install_fabric(minecraft_version: &str, fabric_version: &str, instance_dir: &Path) -> Result<(), String> {
    // Skip if loader already installed
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

    // Create libraries directory following Maven structure
    let libraries_dir = instance_dir.join("libraries");
    fs::create_dir_all(&libraries_dir).await
        .map_err(|e| format!("Failed to create libraries directory: {}", e))?;

    // Get installer info from Fabric Meta API
    let installer_info = get_fabric_installer_info().await?;
    

    // Download Fabric installer
    let installer_path = download_fabric_installer(&installer_info, &libraries_dir).await?;

    // Get Fabric profile.json
    let profile_json = get_fabric_profile_json(minecraft_version, fabric_version).await?;
    

    // Download all required libraries
    download_fabric_libraries(&profile_json, &libraries_dir).await?;

    // Run Fabric installer
    
    run_fabric_installer(&installer_path, instance_dir, minecraft_version, fabric_version).await?;

    

    // Ensure client.jar exists
    ensure_minecraft_client_present(instance_dir, minecraft_version).await?;
    Ok(())
}

// Download client.jar from Mojang to instance versions directory if missing
async fn ensure_minecraft_client_present(instance_dir: &Path, mc_version: &str) -> Result<(), String> {
    let version_dir = instance_dir.join("versions").join(mc_version);
    let jar_path = version_dir.join(format!("{}.jar", mc_version));
    let json_path = version_dir.join(format!("{}.json", mc_version));

    tokio::fs::create_dir_all(&version_dir).await
        .map_err(|e| format!("Failed to create version dir {}: {}", version_dir.display(), e))?;

    // Ensure version JSON exists (download if missing)
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

    let vjson_text = tokio::fs::read_to_string(&json_path).await
        .map_err(|e| format!("Failed to read version json from disk: {}", e))?;

    #[derive(serde::Deserialize)]
    struct DlInfo { url: String }
    #[derive(serde::Deserialize)]
    struct VDownloads { client: Option<DlInfo> }
    #[derive(serde::Deserialize)]
    struct VJson { downloads: Option<VDownloads>, libraries: Vec<Library> }
    let vj: VJson = serde_json::from_str(&vjson_text)
        .map_err(|e| format!("Failed to parse version json: {}", e))?;
    let Some(client_url) = vj.downloads.and_then(|d| d.client.map(|c| c.url)) else {
        return Err("Client download URL not found in version json".to_string());
    };

    // Ensure client.jar exists
    if !jar_path.exists() {
        download_file_with_retry(&client_url, &jar_path).await?;
    }

    // Download vanilla libraries (including LWJGL) to instance_dir/libraries
    let os_name = "windows"; // Running on Windows
    let mut _total_libs = 0usize;
    let mut _lwjgl_libs = 0usize;
    for lib in &vj.libraries {
        if !is_library_allowed(lib, os_name) { continue; }
        if let Some(downloads) = &lib.downloads {
            if let Some(artifact) = &downloads.artifact {
                let lib_path = instance_dir.join("libraries").join(&artifact.path);
                if let Some(parent) = lib_path.parent() { tokio::fs::create_dir_all(parent).await.map_err(|e| format!("Failed to create library dir: {}", e))?; }
                if !lib_path.exists() {
                    download_file_with_retry(&artifact.url, &lib_path).await?;
                }
                _total_libs += 1;
                if lib_path.to_string_lossy().contains("lwjgl") { _lwjgl_libs += 1; }
            }
            // Download Windows natives if present
            if let Some(classifiers) = &downloads.classifiers {
                for (classifier, artifact) in classifiers {
                    if classifier.contains("natives-windows") {
                        let nat_path = instance_dir.join("libraries").join(&artifact.path);
                        if let Some(parent) = nat_path.parent() { tokio::fs::create_dir_all(parent).await.map_err(|e| format!("Failed to create natives dir: {}", e))?; }
                        if !nat_path.exists() {
                            download_file_with_retry(&artifact.url, &nat_path).await?;
                        }
                    }
                }
            }
        }
    }
    

    Ok(())
}

// Download assets (asset index + objects) to instance_dir/assets
async fn ensure_assets_present(app_handle: &tauri::AppHandle, instance_dir: &Path, mc_version: &str) -> Result<String, String> {
    let version_dir = instance_dir.join("versions").join(mc_version);
    let json_path = version_dir.join(format!("{}.json", mc_version));
    if !json_path.exists() {
        return Err(format!("Version json not found: {}", json_path.display()));
    }

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
    if !index_path.exists() {
        download_file_with_retry(&ai.url, &index_path).await?;
    }

    // Read index and download missing objects
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
        if !obj_path.exists() {
            pending.push((prefix, obj.hash));
        }
    }

    if pending.is_empty() {
        return Ok(ai.id);
    }

    let parallel = num_cpus::get().saturating_mul(8).max(50);
    let progress = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let total_count = pending.len() as u64;
    
    let client = std::sync::Arc::new(
        reqwest::Client::builder()
            .user_agent("KindlyKlanKlient/1.0")
            .timeout(std::time::Duration::from_secs(30))
            .pool_max_idle_per_host(parallel)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .build()
            .map_err(|e| e.to_string())?
    );

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
            if !resp.status().is_success() {
                return Err(format!("Asset HTTP {} for {}", resp.status(), url));
            }
            
            let tmp = obj_path.with_extension("kk.tmp");
            let bytes = resp.bytes().await.map_err(|e| format!("Download failed: {}", e))?;
            tokio::fs::write(&tmp, &bytes).await.map_err(|e| format!("Write failed: {}", e))?;
            tokio::fs::rename(&tmp, &obj_path).await.map_err(|e| format!("Rename failed: {}", e))?;

            let cur = progress.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            let _ = app_handle.emit(
                "asset-download-progress",
                serde_json::json!({
                    "current": cur,
                    "total": total_count,
                    "percentage": ((cur as f32 / total_count as f32) * 100.0).min(100.0),
                    "current_file": "",
                    "status": "Mojang"
                })
            );
            
            Ok(())
        }
    }))
    .buffer_unordered(parallel)
    .collect()
    .await;

    // Check for errors
    for result in results {
        if let Err(e) = result {
            eprintln!("Warning: Mojang asset download error: {}", e);
        }
    }
    
    let _ = app_handle.emit("asset-download-completed", serde_json::json!({ "phase": "mojang" }));

    Ok(ai.id)
}

// Same as ensure_assets_present pero sumando al contador combinado
async fn ensure_assets_present_with_progress(
    app_handle: &tauri::AppHandle,
    instance_dir: &Path,
    mc_version: &str,
    combined: Option<(std::sync::Arc<std::sync::atomic::AtomicU64>, u64)>
) -> Result<String, String> {
    let version_dir = instance_dir.join("versions").join(mc_version);
    let json_path = version_dir.join(format!("{}.json", mc_version));
    if !json_path.exists() {
        return Err(format!("Version json not found: {}", json_path.display()));
    }

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

    if pending.is_empty() {
        return Ok(ai.id);
    }

    let (progress, total) = if let Some((p, t)) = combined { 
        (p, t) 
    } else { 
        (std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)), pending.len() as u64) 
    };

    let parallel = num_cpus::get().saturating_mul(8).max(50);
    let client = std::sync::Arc::new(
        reqwest::Client::builder()
            .user_agent("KindlyKlanKlient/1.0")
            .timeout(std::time::Duration::from_secs(30))
            .pool_max_idle_per_host(parallel)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .build()
            .map_err(|e| e.to_string())?
    );

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
            if !resp.status().is_success() {
                return Err(format!("Asset HTTP {} for {}", resp.status(), url));
            }
            
            let tmp = obj_path.with_extension("kk.tmp");
            let bytes = resp.bytes().await.map_err(|e| format!("Download failed: {}", e))?;
            tokio::fs::write(&tmp, &bytes).await.map_err(|e| format!("Write failed: {}", e))?;
            tokio::fs::rename(&tmp, &obj_path).await.map_err(|e| format!("Rename failed: {}", e))?;

            let cur = progress.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            let _ = app_handle.emit(
                "asset-download-progress",
                serde_json::json!({
                    "current": cur,
                    "total": total,
                    "percentage": ((cur as f32 / total as f32) * 100.0).min(100.0),
                    "current_file": "",
                    "status": "Mojang"
                })
            );
            
            Ok(())
        }
    }))
    .buffer_unordered(parallel)
    .collect()
    .await;

    // Check for errors
    for result in results {
        if let Err(e) = result {
            eprintln!("Warning: Mojang asset download error: {}", e);
        }
    }

    Ok(ai.id)
}

async fn install_neoforge(minecraft_version: &str, neoforge_version: &str, instance_dir: &Path) -> Result<(), String> {
    println!("Installing NeoForge {} for Minecraft {}", neoforge_version, minecraft_version);

    // Create installer directory
    let installer_dir = instance_dir.join("installer");
    fs::create_dir_all(&installer_dir).await
        .map_err(|e| format!("Failed to create installer directory: {}", e))?;

    // Download NeoForge installer
    let installer_url = format!(
        "https://maven.neoforged.net/releases/net/neoforged/forge/{}-{}/forge-{}-{}-installer.jar",
        minecraft_version, neoforge_version, minecraft_version, neoforge_version
    );

    let installer_path = installer_dir.join(format!("forge-{}-{}-installer.jar", minecraft_version, neoforge_version));

    println!("Downloading NeoForge installer from: {}", installer_url);
    download_file_with_retry(&installer_url, &installer_path).await?;

    // Run installer
    println!("Running NeoForge installer...");
    run_neoforge_installer(&installer_path, instance_dir, minecraft_version, neoforge_version).await?;

    println!("NeoForge installation completed successfully");
    Ok(())
}

async fn run_fabric_installer(installer_path: &Path, instance_dir: &Path, minecraft_version: &str, fabric_version: &str) -> Result<(), String> {
    // Find installed Java
    let java_path = find_java_executable().await?;

    // Run Fabric installer
    let mut cmd = Command::new(&java_path);
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd
        .args(&[
            "-jar",
            &installer_path.to_string_lossy(),
            "client",
            "-noprofile",
            "-dir",
            &instance_dir.to_string_lossy(),
            "-mcversion",
            minecraft_version,
            "-loader",
            fabric_version
        ])
        .output()
        .map_err(|e| format!("Failed to run Fabric installer: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Fabric installer failed: {}", stderr));
    }

    println!("Fabric installer completed successfully");
    Ok(())
}

// Fabric Meta API functions
async fn get_fabric_installer_info() -> Result<FabricInstallerMeta, String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://meta.fabricmc.net/v2/versions/installer")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Fabric installer info: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let installers: Vec<FabricInstallerMeta> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse installer info: {}", e))?;

    // Find the latest stable version
    let stable_installer = installers
        .into_iter()
        .find(|i| i.stable)
        .ok_or("No stable Fabric installer found")?;

    Ok(stable_installer)
}

// Get Fabric profile JSON for specific version combination
async fn get_fabric_profile_json(minecraft_version: &str, fabric_version: &str) -> Result<FabricProfileJson, String> {
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

    let profile: FabricProfileJson = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Fabric profile: {}", e))?;

    Ok(profile)
}

// Download Fabric installer JAR
async fn download_fabric_installer(installer_info: &FabricInstallerMeta, libraries_dir: &Path) -> Result<PathBuf, String> {
    let installer_path = libraries_dir.join(format!("fabric-installer-{}.jar", installer_info.version));

    download_file_with_retry(&installer_info.url, &installer_path).await?;

    Ok(installer_path)
}

// Download all Fabric libraries from profile
async fn download_fabric_libraries(profile: &FabricProfileJson, libraries_dir: &Path) -> Result<(), String> {
    for library in profile.libraries.iter() {
        let library_path = resolve_maven_path(&library.name, libraries_dir)?;

        if let Some(parent) = library_path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| format!("Failed to create library directory: {}", e))?;
        }

        let library_url = build_library_url(&library)?;
        download_file_with_retry(&library_url, &library_path).await?;
    }

    Ok(())
}

// Convert Maven ID to local file path following Maven structure
fn resolve_maven_path(maven_id: &str, libraries_dir: &Path) -> Result<PathBuf, String> {
    let parts: Vec<&str> = maven_id.split(':').collect();
    if parts.len() < 3 {
        return Err(format!("Invalid Maven ID: {}", maven_id));
    }

    let group_id = parts[0].replace('.', "/");
    let artifact_id = parts[1];
    let version = parts[2];

    let filename = format!("{}-{}.jar", artifact_id, version);
    let path = libraries_dir
        .join(group_id)
        .join(artifact_id)
        .join(version)
        .join(filename);

    Ok(path)
}

// Convert Maven ID to Maven Central URL
#[allow(dead_code)]
fn resolve_maven_url(maven_id: &str) -> Result<String, String> {
    let parts: Vec<&str> = maven_id.split(':').collect();
    if parts.len() < 3 {
        return Err(format!("Invalid Maven ID: {}", maven_id));
    }

    let group_id = parts[0].replace('.', "/");
    let artifact_id = parts[1];
    let version = parts[2];

    let filename = format!("{}-{}.jar", artifact_id, version);
    let url = format!(
        "https://repo1.maven.org/maven2/{}/{}/{}/{}",
        group_id, artifact_id, version, filename
    );

    Ok(url)
}

// Build complete library download URL from base URL or default to Maven Central
fn build_library_url(library: &FabricLibrary) -> Result<String, String> {
    let parts: Vec<&str> = library.name.split(':').collect();
    if parts.len() < 3 {
        return Err(format!("Invalid Maven ID: {}", library.name));
    }

    let group_id_path = parts[0].replace('.', "/");
    let artifact_id = parts[1];
    let version = parts[2];
    let filename = format!("{}-{}.jar", artifact_id, version);

    let base = library
        .url
        .as_ref()
        .map(|u| u.trim_end_matches('/').to_string())
        .unwrap_or_else(|| "https://repo1.maven.org/maven2".to_string());

    Ok(format!(
        "{}/{}/{}/{}/{}",
        base, group_id_path, artifact_id, version, filename
    ))
}

async fn run_neoforge_installer(installer_path: &Path, instance_dir: &Path, _minecraft_version: &str, _neoforge_version: &str) -> Result<(), String> {
    // Find installed Java
    let java_path = find_java_executable().await?;

    // Run NeoForge installer
    let mut cmd = Command::new(&java_path);
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd
        .args(&[
            "-jar",
            &installer_path.to_string_lossy(),
            "--installClient",
            &instance_dir.to_string_lossy()
        ])
        .output()
        .map_err(|e| format!("Failed to run NeoForge installer: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("NeoForge installer failed: {}", stderr));
    }

    println!("NeoForge installer completed successfully");
    Ok(())
}

// Find Java executable in common locations or use managed installation
async fn find_java_executable() -> Result<String, String> {
    let common_paths = [
        "java",
        "/usr/bin/java",
        "/usr/local/bin/java",
        "C:\\Program Files\\Java\\bin\\java.exe",
        "C:\\Program Files (x86)\\Java\\bin\\java.exe",
    ];

    for path in &common_paths {
        if let Ok(output) = Command::new(path).arg("-version").output() {
            if output.status.success() {
                return Ok(path.to_string());
            }
        }
    }

    let java_path = get_java_path_from_env();
    if !java_path.is_empty() {
        return Ok(java_path);
    }

    Err("Java executable not found. Please ensure Java is installed.".to_string())
}

// Get Java path from environment variables or managed installation paths
fn get_java_path_from_env() -> String {
    std::env::var("JAVA_HOME")
        .map(|java_home| format!("{}/bin/java", java_home))
        .unwrap_or_else(|_| {
            let possible_paths = [
                ".kindlyklanklient/java/bin/java",
                "C:\\Users\\{username}\\.kindlyklanklient\\java\\bin\\java",
            ];

            for path in &possible_paths {
                if std::fs::metadata(path).is_ok() {
                    return path.to_string();
                }
            }

            String::new()
        })
}

// Build correct distribution base URL, avoiding duplicate /dist paths
fn build_distribution_url(distribution_url: &str) -> String {
    if distribution_url.trim_end_matches('/').ends_with("/dist") {
        distribution_url.trim_end_matches('/').to_string()
    } else {
        distribution_url.trim_end_matches('/').to_string()
    }
}

// Count total instance files in manifest (mods + configs + packs)
fn count_instance_files(manifest: &InstanceManifest) -> usize {
    let mut n = manifest.files.mods.len() + manifest.files.configs.len();
    if let Some(rp) = &manifest.files.resourcepacks { n += rp.len(); }
    if let Some(sp) = &manifest.files.shaderpacks { n += sp.len(); }
    n
}

// Count pending Mojang asset objects (not present locally)
async fn count_mojang_assets_pending(instance_dir: &Path, mc_version: &str) -> Result<usize, String> {
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

// Get system RAM in GB
#[tauri::command]
fn get_system_ram() -> Result<u32, String> {
    Ok(get_system_ram_gb())
}

// Save RAM configuration
#[tauri::command]
async fn save_ram_config(min_ram: f64, max_ram: f64) -> Result<(), String> {
    use std::fs;
    
    let config_dir = dirs::config_dir()
        .ok_or("Could not find config directory")?
        .join("KindlyKlanKlient");
    
    // Create config directory if it doesn't exist
    fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;
    
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

// Load RAM configuration
#[tauri::command]
async fn load_ram_config() -> Result<(f64, f64), String> {
    use std::fs;
    
    let config_dir = dirs::config_dir()
        .ok_or("Could not find config directory")?
        .join("KindlyKlanKlient");
    
    let config_file = config_dir.join("ram_config.json");
    
    if !config_file.exists() {
        // Return default values if config doesn't exist
        return Ok((2.0, 4.0));
    }
    
    let config_content = fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    
    let config: serde_json::Value = serde_json::from_str(&config_content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;
    
    let min_ram = config["min_ram"].as_f64().unwrap_or(2.0);
    let max_ram = config["max_ram"].as_f64().unwrap_or(4.0);
    
    Ok((min_ram, max_ram))
}

// Save advanced configuration
#[tauri::command]
async fn save_advanced_config(
    jvm_args: String,
    garbage_collector: String,
    window_width: u32,
    window_height: u32
) -> Result<(), String> {
    use std::fs;
    
    let config_dir = dirs::config_dir()
        .ok_or("Could not find config directory")?
        .join("KindlyKlanKlient");
    
    // Create config directory if it doesn't exist
    fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;
    
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

// Load advanced configuration
#[tauri::command]
async fn load_advanced_config() -> Result<(String, String, u32, u32), String> {
    use std::fs;
    
    let config_dir = dirs::config_dir()
        .ok_or("Could not find config directory")?
        .join("KindlyKlanKlient");
    
    let config_file = config_dir.join("advanced_config.json");
    
    if !config_file.exists() {
        // Return default values if config doesn't exist
        return Ok((
            String::new(), // jvm_args
            "G1".to_string(), // garbage_collector
            1280, // window_width
            720   // window_height
        ));
    }
    
    let config_content = fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    
    let config: serde_json::Value = serde_json::from_str(&config_content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;
    
    let jvm_args = config["jvm_args"].as_str().unwrap_or("").to_string();
    let garbage_collector = config["garbage_collector"].as_str().unwrap_or("G1").to_string();
    let window_width = config["window_width"].as_u64().unwrap_or(1280) as u32;
    let window_height = config["window_height"].as_u64().unwrap_or(720) as u32;
    
    Ok((jvm_args, garbage_collector, window_width, window_height))
}

// Launch Minecraft with Java and authentication (validate session first, then pass token)
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
    let instance_dir = get_instance_directory(&instance_id);
    if !instance_dir.exists() {
        return Err(format!("Instance directory does not exist: {}", instance_dir.display()));
    }

    // Validate token before launch (non-fatal if network error)
    if let Ok(res) = ensure_valid_session(app_handle.clone(), "".into()).await {
        match res {
            EnsureSessionResponse::Ok { session, .. } => {
                return launch_minecraft_with_auth(&app_handle, &instance_id, &minecraft_version, &java_path, &session.access_token, min_ram_gb, max_ram_gb).await;
            },
            EnsureSessionResponse::Err { .. } => {
                // Fall back to provided token
            }
        }
    }

    launch_minecraft_with_auth(&app_handle, &instance_id, &minecraft_version, &java_path, &access_token, min_ram_gb, max_ram_gb).await
}

// Launch Minecraft with authentication and proper classpath
async fn launch_minecraft_with_auth(
    app_handle: &tauri::AppHandle,
    instance_id: &str,
    minecraft_version: &str,
    java_path: &str,
    access_token: &str,
    min_ram_gb: Option<f64>,
    max_ram_gb: Option<f64>
) -> Result<String, String> {
    let instance_dir = get_instance_directory(instance_id);

    // Ensure client.jar and vanilla libraries exist
    ensure_minecraft_client_present(&instance_dir, minecraft_version).await?;

    // Build classpath for mods and libraries
    let classpath = build_minecraft_classpath(&instance_dir)?;
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

    // Build JVM arguments with RAM parameters
    let min_ram = min_ram_gb.unwrap_or(2.0);
    let max_ram = max_ram_gb.unwrap_or(4.0);
    
    // Load advanced configuration for JVM args and window settings
    let (jvm_args_config, gc_config, window_width, window_height) = load_advanced_config().await.unwrap_or((
        String::new(), "G1".to_string(), 1280, 720
    ));
    
    let jvm_args = build_minecraft_jvm_args(access_token, min_ram, max_ram, &gc_config, &jvm_args_config)?;

    // Assets ya preparados en fase previa. Asegura asset index ID pero sin re-emitir progreso independiente
    let asset_index_id = ensure_assets_present(app_handle, &instance_dir, minecraft_version).await?;

    // Get Minecraft profile for username/uuid
    let profile_json = get_minecraft_profile(access_token.to_string()).await?;
    let profile: serde_json::Value = serde_json::from_str(&profile_json).map_err(|e| e.to_string())?;
    let username = profile["name"].as_str().unwrap_or("Player");
    let uuid = profile["id"].as_str().unwrap_or("00000000000000000000000000000000");

    // Build Minecraft arguments
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

    // Add window resolution arguments 
    mc_args.push("--width".to_string());
    mc_args.push(window_width.to_string());
    mc_args.push("--height".to_string());
    mc_args.push(window_height.to_string());

    // Execute Minecraft
    let main_class = select_main_class(&instance_dir);
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

    // Notify when Minecraft process exits
    let app = app_handle.clone();
    std::thread::spawn(move || {
        let _ = child.wait();
        let _ = app.emit("minecraft_exited", serde_json::json!({ "status": "exited" }));
    });

    Ok("Minecraft launched".to_string())
}

// Build Minecraft classpath from libraries, versions, and mods directories
fn build_minecraft_classpath(instance_dir: &Path) -> Result<String, String> {
    let mut jars: Vec<String> = Vec::new();

    let libs_dir = instance_dir.join("libraries");
    if libs_dir.exists() {
        collect_jars_recursively(&libs_dir, &mut jars)?;
    }

    let versions_dir = instance_dir.join("versions");
    if versions_dir.exists() {
        collect_jars_recursively(&versions_dir, &mut jars)?;
    }

    let mods_dir = instance_dir.join("mods");
    if mods_dir.exists() {
        collect_jars_recursively(&mods_dir, &mut jars)?;
    }

    if jars.is_empty() {
        return Err("No jars found for classpath".to_string());
    }

    Ok(jars.join(if cfg!(target_os = "windows") { ";" } else { ":" }))
}

// Recursively collect JAR files from directory
fn collect_jars_recursively(dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().is_file() {
            let p = entry.into_path();
            if p.extension().map_or(false, |e| e == "jar") {
                out.push(p.to_string_lossy().to_string());
            }
        }
    }
    Ok(())
}

// Select appropriate main class based on mod loader presence
fn select_main_class(instance_dir: &Path) -> &'static str {
    let fabric_loader_dir = instance_dir.join("libraries").join("net").join("fabricmc");
    if fabric_loader_dir.exists() {
        return "net.fabricmc.loader.impl.launch.knot.KnotClient";
    }
    "net.minecraft.client.Main"
}

// Get system RAM in GB using sysinfo (cross-platform)
fn get_system_ram_gb() -> u32 {
    use sysinfo::System;
    
    let mut system = System::new_all();
    system.refresh_memory();
    
    let total_memory_bytes = system.total_memory();
    let total_memory_gb = (total_memory_bytes / (1024 * 1024 * 1024)) as u32;
    
    println!("System RAM detected: {} GB ({} bytes)", total_memory_gb, total_memory_bytes);
    
    // Ensure minimum of 4GB for safety
    std::cmp::max(total_memory_gb, 4)
}

// Build JVM arguments for Minecraft launch
fn build_minecraft_jvm_args(
    access_token: &str, 
    min_ram_gb: f64, 
    max_ram_gb: f64,
    garbage_collector: &str,
    additional_jvm_args: &str
) -> Result<Vec<String>, String> {
    let mut args = vec![
        format!("-Xmx{}G", max_ram_gb as u32),
        format!("-Xms{}G", min_ram_gb as u32),
        "-XX:+UnlockExperimentalVMOptions".to_string(),
    ];

    // Add garbage collector specific arguments
    match garbage_collector {
        "G1" => {
            args.extend(vec![
                "-XX:+UseG1GC".to_string(),
                "-XX:G1NewSizePercent=20".to_string(),
                "-XX:G1ReservePercent=20".to_string(),
                "-XX:MaxGCPauseMillis=50".to_string(),
                "-XX:G1HeapRegionSize=32M".to_string(),
            ]);
        },
        "ZGC" => {
            args.extend(vec![
                "-XX:+UseZGC".to_string(),
                "-XX:+UnlockExperimentalVMOptions".to_string(),
            ]);
        },
        "Parallel" => {
            args.extend(vec![
                "-XX:+UseParallelGC".to_string(),
                "-XX:ParallelGCThreads=4".to_string(),
            ]);
        },
        _ => {
            // Default to G1
            args.extend(vec![
                "-XX:+UseG1GC".to_string(),
                "-XX:G1NewSizePercent=20".to_string(),
                "-XX:G1ReservePercent=20".to_string(),
                "-XX:MaxGCPauseMillis=50".to_string(),
                "-XX:G1HeapRegionSize=32M".to_string(),
            ]);
        }
    }

    // Note: Window mode (fullscreen/borderless) is handled by Minecraft's internal settings
    // No special JVM arguments are needed for window mode configuration

    // Add additional JVM arguments if provided
    if !additional_jvm_args.trim().is_empty() {
        let additional_args: Vec<&str> = additional_jvm_args.split_whitespace().collect();
        for arg in additional_args {
            if !arg.is_empty() {
                args.push(arg.to_string());
            }
        }
    }

    // Add authentication properties
    args.push(format!("-Dminecraft.api.auth.host=https://api.minecraftservices.com"));
    args.push(format!("-Dminecraft.api.session.host=https://api.minecraftservices.com"));
    args.push(format!("-Dminecraft.api.services.host=https://api.minecraftservices.com"));

    args.push(format!("-Dminecraft.api.accessToken={}", access_token));

    Ok(args)
}

// Build Minecraft game arguments (deprecated - use direct construction in launch function)
#[allow(dead_code)]
fn build_minecraft_args(_instance_id: &str, minecraft_version: &str) -> Result<Vec<String>, String> {
    Ok(vec![
        "--version".to_string(),
        minecraft_version.to_string(),
        "--accessToken".to_string(),
        "{ACCESS_TOKEN}".to_string(),
        "--userType".to_string(),
        "msa".to_string(),
        "--versionType".to_string(),
        "release".to_string(),
    ])
}

// Main Tauri application entry point
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    // Initialize logging system
    if let Err(e) = logging::init_logging() {
        eprintln!("Error initializing logging: {}", e);
    }
    
    log::info!("Starting KindlyKlanKlient...");
    
    tauri::Builder::default()
        .plugin(tauri_plugin_oauth::init())
        .plugin(tauri_plugin_updater::Builder::default().build())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_versions,
            launch_game,
            start_microsoft_auth,
            load_distribution_manifest,
            get_instance_details,
            download_instance,
            check_java_version,
            download_java,
            get_java_path,
            create_instance_directory,
            launch_minecraft_with_java,
            get_required_java_version_command,
            stop_minecraft_instance,
            restart_application,
            upload_skin_to_mojang,
            set_skin_variant,
            get_minecraft_profile,
            create_temp_file,
            check_for_updates,
            get_system_ram,
            save_ram_config,
            load_ram_config,
            save_advanced_config,
            load_advanced_config,
            install_update,
            get_update_state,
            save_update_state_command,
            download_update_silent,
            check_whitelist_access,
            get_accessible_instances,
            clear_whitelist_cache,
            open_url,
            debug_env_vars,
            // Session management commands
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
            test_manifest_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
