// Kindly Klan Klient
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use tauri::Url;


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
    pub r#type: String, // "fabric", "forge", "neoforge", "vanilla"
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

// Microsoft Authentication structs
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
}

// Original structs from RCraft launcher
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

// --- Structs for libraries and natives ---
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

struct MinecraftLauncher {
    config: LauncherConfig,
}

impl MinecraftLauncher {
    fn new() -> Result<Self> {
        Ok(Self {
            config: LauncherConfig::new()?,
        })
    }

    async fn get_available_versions(&self) -> Result<Vec<MinecraftVersion>> {
        let url = "https://launchermeta.mojang.com/mc/game/version_manifest.json";
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
            eprintln!("Fetching from: {}", url);
            match client.get(url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        let text = response.text().await?;
                        eprintln!("Response text length: {}", text.len());
                        match serde_json::from_str::<VersionManifest>(&text) {
                            Ok(manifest) => {
                                let release_versions: Vec<MinecraftVersion> = manifest
                                    .versions
                                    .into_iter()
                                    .filter(|v| v.version_type == "release")
                                    .collect();
                                eprintln!("Loaded {} release versions", release_versions.len());
                                Ok(release_versions)
                            }
                            Err(e) => {
                                eprintln!("Failed to parse JSON: {}", e);
                                eprintln!("Response text (first 500 chars): {}", &text[0..500.min(text.len())]);
                                Err(e.into())
                            }
                        }
                    } else {
                        eprintln!("API returned error status: {}", response.status());
                        Err(anyhow::anyhow!("API error: {}", response.status()))
                    }
                }
                Err(e) => {
                    eprintln!("Failed to fetch URL: {}", e);
                    Err(e.into())
                }
            }
    }

    async fn download_version(&self, version: &MinecraftVersion) -> Result<()> {
        eprintln!("Creating version directory for: {}", version.id);
        let version_dir = self.config.versions_dir.join(&version.id);
        fs::create_dir_all(&version_dir).await?;
        let natives_dir = version_dir.join("natives");
        fs::create_dir_all(&natives_dir).await?;

        // Download the version file
        eprintln!("Downloading version manifest from: {}", version.url);
        let version_response = reqwest::get(&version.url).await?;
        let version_data = version_response.text().await?;
        let version_file = version_dir.join(format!("{}.json", version.id));
        fs::write(&version_file, &version_data).await?;
        eprintln!("Version manifest downloaded successfully");

        // Download client.jar
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
            eprintln!("Downloading client.jar...");
            let jar_url = client.url;
            let jar_path = version_dir.join(format!("{}.jar", version.id));
            let resp = reqwest::get(&jar_url).await?;
            let bytes = resp.bytes().await?.to_vec();
            let mut out = File::create(&jar_path)?;
            out.write_all(&bytes)?;
            eprintln!("Client.jar downloaded successfully");
        }

        // Download libraries and natives (simplified)
        eprintln!("Downloading libraries...");
        let os_name = "windows";
        let mut lib_count = 0;
        for lib in &version_json.libraries {
            let allowed = is_library_allowed(lib, os_name);
            if !allowed {
                continue;
            }
            // Download normal library
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
                        lib_count += 1;
                        if lib_count % 5 == 0 {
                            eprintln!("Downloaded {} libraries...", lib_count);
                        }
                    }
                }
            }
        }
        eprintln!("All libraries downloaded! Total: {}", lib_count);

        // Download assets (like original RCraft but with progress)
        if let Some(asset_index) = &version_json.asset_index {
            eprintln!("Starting asset download for index: {}", asset_index.id);
            let indexes_dir = self.config.assets_dir.join("indexes");
            fs::create_dir_all(&indexes_dir).await?;
            let index_path = indexes_dir.join(format!("{}.json", asset_index.id));

            let resp = reqwest::get(&asset_index.url).await?;
            let bytes = resp.bytes().await?.to_vec();
            let mut out = File::create(&index_path)?;
            out.write_all(&bytes)?;
            eprintln!("Asset index downloaded");

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

            // Count missing assets first
            let mut missing_assets = Vec::new();
            for (_key, obj) in &asset_index_json.objects {
                let hash_prefix = &obj.hash[0..2];
                let object_dir = self.config.assets_dir.join("objects").join(hash_prefix);
                let object_path = object_dir.join(&obj.hash);
                if !object_path.exists() {
                    missing_assets.push(obj.clone());
                }
            }

            let total_objects = asset_index_json.objects.len();
            let missing_count = missing_assets.len();
            eprintln!("Found {} total assets, {} need to be downloaded", total_objects, missing_count);

            if missing_count > 0 {
                // Create HTTP client for reuse
                let client = reqwest::Client::new();
                let mut downloaded = 0;

                // Download in chunks to avoid overwhelming the server
                for chunk in missing_assets.chunks(50) {
                    let mut tasks = Vec::new();
                    
                    for obj in chunk {
                        let hash_prefix = &obj.hash[0..2];
                        let object_dir = self.config.assets_dir.join("objects").join(hash_prefix);
                        fs::create_dir_all(&object_dir).await?;
                        let object_path = object_dir.join(&obj.hash);
                        let object_url = format!("https://resources.download.minecraft.net/{}/{}", hash_prefix, obj.hash);
                        
                        let client_clone = client.clone();
                        let object_path_clone = object_path.clone();
                        
                        let task = tokio::spawn(async move {
                            match client_clone.get(&object_url).send().await {
                                Ok(resp) => {
                                    match resp.bytes().await {
                                        Ok(bytes) => {
                                            match tokio::fs::File::create(&object_path_clone).await {
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
                    
                    // Wait for all tasks in this chunk to complete
                    for task in tasks {
                        match task.await {
                            Ok(Ok(_)) => {
                                downloaded += 1;
                            }
                            Ok(Err(e)) => {
                                eprintln!("Asset download error: {}", e);
                            }
                            Err(e) => {
                                eprintln!("Task error: {}", e);
                            }
                        }
                    }
                    
                    eprintln!("Downloaded {} / {} assets", downloaded, missing_count);
                }
                
                eprintln!("Asset download completed! Downloaded {} new assets", downloaded);
            } else {
                eprintln!("All assets already exist, skipping download");
            }
        }
        Ok(())
    }

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
            let allowed = is_library_allowed(lib, os_name);
            if !allowed {
                continue;
            }
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

    async fn launch_minecraft(&self, version: &str, username: &str, ram_mb: u32, access_token: Option<&str>, uuid: Option<&str>) -> Result<()> {
        eprintln!("Starting Minecraft launch for version: {}, username: {}", version, username);
        
        let java_path = self.find_java()?;
        eprintln!("Found Java at: {:?}", java_path);
        
        let version_dir = self.config.versions_dir.join(version);
        let jar_path = version_dir.join(format!("{}.jar", version));
        let natives_dir = version_dir.join("natives");

        if !jar_path.exists() {
            return Err(anyhow::anyhow!("Version not downloaded"));
        }
        
        eprintln!("Building classpath...");
        let classpath = self.build_classpath(version).await?;
        eprintln!("Classpath built, length: {}", classpath.len());
        
        let mut command = Command::new(&java_path);
        command
            .arg("-Xmx".to_string() + &ram_mb.to_string() + "M")
            .arg("-Xms".to_string() + &(ram_mb / 2).to_string() + "M")
            .arg("-Djava.library.path=".to_string() + &natives_dir.display().to_string())
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
        
        eprintln!("About to launch command: {:?}", command);
        
        // Launch Minecraft in detached mode (don't wait for it to finish)
        let child = command.spawn()?;
        eprintln!("Minecraft process spawned with PID: {:?}", child.id());
        
        // Don't wait for the process - let Minecraft run independently
        Ok(())
    }

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
    Ok(4096) // Default
}

// Microsoft Authentication functions
const AZURE_CLIENT_ID: &str = "d1538b43-1083-43ac-89d5-c88cb0049ada";

#[tauri::command]
async fn start_microsoft_auth() -> Result<AuthSession, String> {
    eprintln!("Starting Microsoft authentication...");
    
    use std::sync::{Arc, Mutex};
    
    // Create a shared variable to store the captured URL
    let captured_url = Arc::new(Mutex::new(None::<String>));
    let captured_url_clone = captured_url.clone();
    
    // Start OAuth server with custom response
    use tauri_plugin_oauth::start_with_config;
    
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
        <div class="success">Has iniciado sesión correctamente</div>
        <div class="instructions">Ya puedes cerrar esta pestaña y regresar a la aplicación</div>
    </div>
</body>
</html>
        "#.into()),
    };
    
    let port = start_with_config(config, move |url| {
        eprintln!("OAuth callback received: {}", url);
        let mut captured = captured_url_clone.lock().unwrap();
        *captured = Some(url);
    }).map_err(|e| format!("Failed to start OAuth server: {}", e))?;
    
    eprintln!("OAuth server started on port: {}", port);
    
    // Build the Microsoft OAuth URL - use consumers tenant, not common
    let auth_url = format!(
        "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize?client_id={}&response_type=code&redirect_uri=http://localhost:{}&scope=XboxLive.signin%20offline_access&prompt=select_account",
        AZURE_CLIENT_ID, port
    );
    
    eprintln!("Opening auth URL: {}", auth_url);
    
    // Open the browser
    if let Err(e) = open::that(&auth_url) {
        return Err(format!("Failed to open browser: {}", e));
    }
    
    // Wait for the callback (with timeout)
    let start_time = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(300); // 5 minutes
    
    loop {
        let captured_url_option = {
            let captured = captured_url.lock().unwrap();
            captured.clone()
        };
        
        if let Some(url) = captured_url_option {
            // Extract authorization code from the URL
            let auth_code = extract_auth_code_from_url(&url)
                .ok_or_else(|| "No authorization code found in callback URL".to_string())?;
            
            eprintln!("Authorization code received: {}", auth_code);
            
            // Complete the authentication flow
            return complete_microsoft_auth_internal(auth_code, port).await;
        }
        
        if start_time.elapsed() > timeout {
            return Err("Authentication timeout".to_string());
        }
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

async fn complete_microsoft_auth_internal(auth_code: String, port: u16) -> Result<AuthSession, String> {
    eprintln!("Completing Microsoft authentication with code: {}", auth_code);
    
    // Step 1: Exchange authorization code for Microsoft access token
    eprintln!("Step 1: Exchanging authorization code for Microsoft token...");
    let ms_token = exchange_auth_code_for_token(auth_code, port).await
        .map_err(|e| format!("Failed to exchange auth code: {}", e))?;
    eprintln!("Step 1 complete: Microsoft token obtained");
    
    // Step 2: Authenticate with Xbox Live
    eprintln!("Step 2: Authenticating with Xbox Live...");
    let xbox_token = authenticate_xbox_live(&ms_token.access_token).await
        .map_err(|e| format!("Failed Xbox Live auth: {}", e))?;
    eprintln!("Step 2 complete: Xbox Live token obtained");
    
    // Step 3: Authenticate with XSTS
    eprintln!("Step 3: Authenticating with XSTS...");
    let xsts_token = authenticate_xsts(&xbox_token.token).await
        .map_err(|e| format!("Failed XSTS auth: {}", e))?;
    eprintln!("Step 3 complete: XSTS token obtained");
    
    // Step 4: Get Minecraft access token
    eprintln!("Step 4: Getting Minecraft access token...");
    let mc_token = authenticate_minecraft(&xsts_token).await
        .map_err(|e| format!("Failed Minecraft auth: {}", e))?;
    eprintln!("Step 4 complete: Minecraft token obtained: {}...", &mc_token.access_token[..50]);
    
    // Step 5: Get Minecraft profile
    eprintln!("Step 5: Getting Minecraft profile...");
    let access_token = mc_token.access_token.clone(); // Clone for later use
    let profile_json = get_minecraft_profile(access_token.clone()).await
        .map_err(|e| format!("Failed to get profile: {}", e))?;

    // Parse the JSON response
    let profile: serde_json::Value = serde_json::from_str(&profile_json)
        .map_err(|e| format!("Failed to parse profile JSON: {}", e))?;

    let username = profile["name"].as_str().unwrap_or("Unknown");
    let uuid = profile["id"].as_str().unwrap_or("unknown");
    eprintln!("Step 5 complete: Profile obtained - Name: {}, UUID: {}", username, uuid);

    let session = AuthSession {
        access_token: access_token,
        username: username.to_string(),
        uuid: uuid.to_string(),
        user_type: "microsoft".to_string(),
    };
    
    eprintln!("Microsoft authentication completed for user: {}", session.username);
    Ok(session)
}

fn extract_auth_code_from_url(url_str: &str) -> Option<String> {
    // Parse URL and extract the 'code' parameter
    if let Ok(url) = Url::parse(url_str) {
        for (key, value) in url.query_pairs() {
            if key == "code" {
                return Some(value.to_string());
            }
        }
    }
    None
}

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

// Tauri commands
#[tauri::command]
async fn greet(name: String) -> String {
    format!("Hello, {}! Welcome to Kindly Klan Klient!", name)
}

#[tauri::command]
async fn get_versions() -> Result<Vec<MinecraftVersion>, String> {
    eprintln!("get_versions called");
    let launcher = MinecraftLauncher::new().map_err(|e| e.to_string())?;
    launcher.get_available_versions().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn launch_game(version: String, session: AuthSession) -> Result<String, String> {
    eprintln!("launch_game called with version: {}, username: {}", version, session.username);
    
    let launcher = MinecraftLauncher::new().map_err(|e| {
        eprintln!("Failed to create launcher: {}", e);
        e.to_string()
    })?;
    
    launcher.config.ensure_directories().await.map_err(|e| {
        eprintln!("Failed to ensure directories: {}", e);
        e.to_string()
    })?;
    
    let ram_mb = get_total_ram_mb().unwrap_or(4096);
    eprintln!("Using RAM: {}MB", ram_mb);

    // Check if version is downloaded
    let version_dir = launcher.config.versions_dir.join(&version);
    let jar_path = version_dir.join(format!("{}.jar", version));
    
    // Check if we need to download anything (jar, assets, etc.)
    let versions = launcher.get_available_versions().await.map_err(|e| {
        eprintln!("Failed to get versions: {}", e);
        e.to_string()
    })?;
    
    if let Some(target_version) = versions.into_iter().find(|v| v.id == version) {
        // Always check specific asset files to ensure they actually exist
        let assets_dir = launcher.config.assets_dir.join("objects");
        let missing_assets = [
            "5f/5ff04807c356f1beed0b86ccf659b44b9983e3fa",
            "b3/b3305151c36cc6e776f0130e85e8baee7ea06ec9", 
            "b8/b84572b0d91367c41ff73b22edd5a2e9c02eab13",
            "40/402ded0eebd448033ef415e861a17513075f80e7",
            "89/89e4e7c845d442d308a6194488de8bd3397f0791"
        ];
        
        let mut assets_missing = false;
        for asset_path in &missing_assets {
            let full_path = assets_dir.join(asset_path);
            if !full_path.exists() {
                assets_missing = true;
                eprintln!("Missing critical asset: {}", asset_path);
                break;
            }
        }
        
        let need_download = !jar_path.exists() || assets_missing;
        
        if need_download {
            eprintln!("Downloading version files and assets...");
            launcher.download_version(&target_version).await.map_err(|e| {
                eprintln!("Failed to download version: {}", e);
                e.to_string()
            })?;
            eprintln!("Download completed");
        } else {
            eprintln!("Version jar exists, checking assets...");
            // Even if we think assets exist, let's force download to be sure
            eprintln!("Force downloading assets to ensure completeness...");
            launcher.download_version(&target_version).await.map_err(|e| {
                eprintln!("Failed to download version: {}", e);
                e.to_string()
            })?;
            eprintln!("Assets verified and downloaded");
        }
    } else {
        return Err("Version not found".to_string());
    }

    eprintln!("Launching Minecraft...");
    launcher.launch_minecraft(&version, &session.username, ram_mb, Some(&session.access_token), Some(&session.uuid)).await.map_err(|e| {
        eprintln!("Failed to launch Minecraft: {}", e);
        e.to_string()
    })?;
    
    Ok("Minecraft launched successfully!".to_string())
}

// New commands for instance system

#[tauri::command]
async fn load_distribution_manifest(url: String) -> Result<DistributionManifest, String> {
    eprintln!("Loading distribution manifest from: {}", url);

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

    eprintln!("Successfully loaded distribution: {}", manifest.distribution.name);
    eprintln!("Found {} instances", manifest.instances.len());

    Ok(manifest)
}

#[tauri::command]
async fn get_instance_details(base_url: String, instance_url: String) -> Result<InstanceManifest, String> {
    let full_url = if instance_url.starts_with("http") {
        instance_url
    } else {
        format!("{}/{}", base_url.trim_end_matches('/'), instance_url.trim_start_matches('/'))
    };

    eprintln!("Loading instance details from: {}", full_url);
    
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

    eprintln!("Successfully loaded instance: {}", instance.instance.name);

    Ok(instance)
}

#[tauri::command]
async fn download_instance(
    base_url: String, 
    instance: InstanceManifest,
    _session: AuthSession
) -> Result<String, String> {
    eprintln!("Starting download for instance: {}", instance.instance.name);
    
    let launcher = MinecraftLauncher::new().map_err(|e| {
        eprintln!("Failed to create launcher: {}", e);
        e.to_string()
    })?;

    launcher.config.ensure_directories().await.map_err(|e| {
        eprintln!("Failed to ensure directories: {}", e);
        e.to_string()
    })?;

    // Create instance directory
    let instance_dir = launcher.config.versions_dir.join(&instance.instance.id);
    fs::create_dir_all(&instance_dir).await.map_err(|e| {
        eprintln!("Failed to create instance directory: {}", e);
        e.to_string()
    })?;

    // Download Minecraft version first (vanilla)
    eprintln!("Downloading Minecraft {} for instance...", instance.instance.minecraft_version);
    let versions = launcher.get_available_versions().await.map_err(|e| {
        eprintln!("Failed to get Minecraft versions: {}", e);
        e.to_string()
    })?;

    if let Some(mc_version) = versions.into_iter().find(|v| v.id == instance.instance.minecraft_version) {
        launcher.download_version(&mc_version).await.map_err(|e| {
            eprintln!("Failed to download Minecraft version: {}", e);
            e.to_string()
        })?;
    } else {
        return Err(format!("Minecraft version {} not found", instance.instance.minecraft_version));
    }

    // TODO: Download mod loader (Fabric/Forge/NeoForge) if specified
    if let Some(mod_loader) = &instance.instance.mod_loader {
        eprintln!("Mod loader detected: {} {}", mod_loader.r#type, mod_loader.version);
        // Implementation for mod loader installation will go here
    }

    // Download mods
    eprintln!("Downloading {} mods...", instance.files.mods.len());
    for (i, mod_file) in instance.files.mods.iter().enumerate() {
        let file_url = if mod_file.url.starts_with("http") {
            mod_file.url.clone()
        } else {
            format!("{}/{}", base_url.trim_end_matches('/'), mod_file.url.trim_start_matches('/'))
        };
        
        eprintln!("Downloading mod {}/{}: {}", i + 1, instance.files.mods.len(), mod_file.name);
        
        let target_path = launcher.config.minecraft_dir
            .join("instances")
            .join(&instance.instance.id)
            .join("mods")
            .join(&mod_file.name);

        fs::create_dir_all(target_path.parent().unwrap()).await.map_err(|e| {
            eprintln!("Failed to create mod directory: {}", e);
            e.to_string()
        })?;

        download_file(&file_url, &target_path).await.map_err(|e| {
            eprintln!("Failed to download mod {}: {}", mod_file.name, e);
            e.to_string()
        })?;
    }

    // Download configs
    eprintln!("Downloading {} config files...", instance.files.configs.len());
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

        fs::create_dir_all(target_path.parent().unwrap()).await.map_err(|e| {
            eprintln!("Failed to create config directory: {}", e);
            e.to_string()
        })?;

        download_file(&file_url, &target_path).await.map_err(|e| {
            eprintln!("Failed to download config {}: {}", config_file.name, e);
            e.to_string()
        })?;
    }

    eprintln!("Instance {} downloaded successfully!", instance.instance.name);
    Ok(format!("Instance {} ready to launch!", instance.instance.name))
}

async fn download_file(url: &str, path: &Path) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
    }

    let bytes = response.bytes().await?;
    let mut file = fs::File::create(path).await?;
    file.write_all(&bytes).await?;
    
    Ok(())
}

// Java version mapping for Minecraft versions
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

#[tauri::command]
async fn check_java_version(version: String) -> Result<String, String> {
    // Check if Java is installed in our managed location
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

#[tauri::command]
async fn download_java(version: String) -> Result<String, String> {
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;
    use std::process::Command;

    let kindly_dir = std::env::var("USERPROFILE")
        .map(|p| Path::new(&p).join(".kindlyklanklient"))
        .unwrap_or_else(|_| Path::new(".").join(".kindlyklanklient"));

    let runtime_dir = kindly_dir.join("runtime");
    let java_dir = runtime_dir.join(format!("java-{}", version));

    // Create runtime directory if it doesn't exist
    tokio::fs::create_dir_all(&runtime_dir).await
        .map_err(|e| format!("Failed to create runtime directory: {}", e))?;

    // Determine platform and architecture
    let (os, arch, extension) = if cfg!(target_os = "windows") {
        ("windows", "x64", "zip")
    } else if cfg!(target_os = "macos") {
        ("mac", "x64", "tar.gz")
    } else {
        ("linux", "x64", "tar.gz")
    };

    // Get the appropriate JRE URL from Adoptium API
    // Using the correct API endpoint format: https://api.adoptium.net/v3/binary/latest/{feature_version}/{release_type}/{os}/{arch}/{image_type}/{jvm_impl}/{heap_size}/{vendor}
    let jre_url = format!(
        "https://api.adoptium.net/v3/binary/latest/{}/ga/{}/{}/jdk/hotspot/normal/eclipse",
        version, os, arch
    );

    // Download the JDK
    println!("Downloading Java from URL: {}", jre_url);
    let client = reqwest::Client::new();
    let response = client
        .get(&jre_url)
        .header("User-Agent", "KindlyKlanKlient/1.0")
        .header("Accept", "application/octet-stream")
        .send()
        .await
        .map_err(|e| format!("Failed to download Java: {}", e))?;

    println!("Response status: {}", response.status());
    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    println!("Downloaded {} bytes", bytes.len());

    // Save the downloaded file temporarily
    let temp_file = runtime_dir.join(format!("java-{}.{}", version, extension));

    // Create temp file and write bytes
    println!("Creating temp file: {}", temp_file.display());
    let mut file = File::create(&temp_file)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    file.write_all(&bytes)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;

    // Ensure file is fully written and flushed
    file.flush().map_err(|e| format!("Failed to flush file: {}", e))?;
    drop(file); // Close the file explicitly

    println!("Temp file created successfully");

    // Small delay to ensure file is fully written before extraction
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Remove existing java directory if it exists
    if java_dir.exists() {
        println!("Removing existing java directory: {}", java_dir.display());
        let _ = std::fs::remove_dir_all(&java_dir);
    }

    // Extract the archive
    if cfg!(target_os = "windows") {
        // Try 7-Zip first, fallback to PowerShell
        println!("Trying to extract with 7-Zip...");
        let seven_zip_result = Command::new("7z")
            .args(&["x", &temp_file.to_string_lossy(), &format!("-o{}", runtime_dir.display()), "-y"])
            .output();

        match seven_zip_result {
            Ok(output) => {
                println!("7-Zip exit code: {:?}", output.status);
                if output.status.success() {
                    println!("Archive extracted successfully with 7-Zip");
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    println!("7-Zip stderr: {}", error);
                    return Err(format!("7-Zip extraction failed: {}", error));
                }
            },
            Err(_) => {
                println!("7-Zip not found, falling back to PowerShell...");
                // Use PowerShell to extract zip on Windows
                println!("Extracting archive with PowerShell...");
                let output = Command::new("powershell")
                    .args(&["-Command", &format!("Expand-Archive -Path '{}' -DestinationPath '{}' -Force", temp_file.display(), runtime_dir.display())])
                    .output()
                    .map_err(|e| format!("Failed to extract Java archive: {}", e))?;

                println!("PowerShell exit code: {:?}", output.status);
                println!("PowerShell stdout: {}", String::from_utf8_lossy(&output.stdout));
                println!("PowerShell stderr: {}", String::from_utf8_lossy(&output.stderr));

                if !output.status.success() {
                    return Err(format!("PowerShell extraction failed: {}", String::from_utf8_lossy(&output.stderr)));
                }
                println!("Archive extracted successfully with PowerShell");
            }
        }
    } else {
        // Use tar for Unix systems
        println!("Extracting archive with tar...");
        let output = Command::new("tar")
            .args(&["-xzf", &temp_file.to_string_lossy(), "-C", &runtime_dir.to_string_lossy()])
            .output()
            .map_err(|e| format!("Failed to extract Java archive: {}", e))?;

        println!("Tar exit code: {:?}", output.status);
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            println!("Tar stderr: {}", error);
            return Err(format!("Extraction failed: {}", error));
        }
        println!("Archive extracted successfully");
    }

    // Find the extracted directory and move it to the correct location
    println!("Looking for extracted directories in: {}", runtime_dir.display());
    
    // First, let's see what's actually in the directory
    let all_entries = std::fs::read_dir(&runtime_dir)
        .map_err(|e| format!("Failed to read runtime directory: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to read directory entries: {}", e))?;
    
    println!("All entries in runtime directory:");
    for entry in &all_entries {
        println!("  - {} (is_dir: {})", entry.path().display(), entry.path().is_dir());
    }
    
    // Look for directories that are NOT the target java_dir
    let extracted_dirs: Vec<_> = all_entries
        .into_iter()
        .filter(|entry| {
            let path = entry.path();
            let is_dir = path.is_dir();
            let is_not_target = path != java_dir;
            println!("Checking: {} - is_dir: {}, is_not_target: {}", path.display(), is_dir, is_not_target);
            is_dir && is_not_target
        })
        .map(|entry| entry.path())
        .collect();

    println!("Found {} extracted directories", extracted_dirs.len());

    if let Some(extracted_dir) = extracted_dirs.first() {
        println!("Moving directory from {} to {}", extracted_dir.display(), java_dir.display());
        // Move the extracted directory to the correct location
        if java_dir.exists() {
            println!("Removing existing java directory");
            let _ = std::fs::remove_dir_all(&java_dir);
        }

        std::fs::rename(extracted_dir, &java_dir)
            .map_err(|e| format!("Failed to move Java directory: {}", e))?;

        println!("Successfully moved Java directory");

        // Remove any other extracted directories
        for dir in extracted_dirs.iter().skip(1) {
            println!("Removing extra directory: {}", dir.display());
            let _ = std::fs::remove_dir_all(dir);
        }
    } else {
        println!("No extracted directories found!");
        return Err("No Java directory found after extraction".to_string());
    }

    // Clean up temp file AFTER moving directories
    println!("Removing temp file...");
    let _ = std::fs::remove_file(&temp_file);

    Ok(format!("Java {} downloaded and installed successfully", version))
}

#[tauri::command]
async fn get_java_path(version: String) -> Result<String, String> {
    // Get Java from our managed location
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

#[tauri::command]
async fn create_instance_directory(instance_id: String, java_version: String) -> Result<String, String> {
    let kindly_dir = std::env::var("USERPROFILE")
        .map(|p| std::path::Path::new(&p).join(".kindlyklanklient"))
        .unwrap_or_else(|_| std::path::Path::new(".").join(".kindlyklanklient"));

    let instance_dir = kindly_dir.join(&instance_id);
    let runtime_dir = kindly_dir.join("runtime");
    let java_dir = runtime_dir.join(format!("java-{}", java_version));

    // Create directories
    tokio::fs::create_dir_all(&instance_dir).await
        .map_err(|e| format!("Failed to create instance directory: {}", e))?;
    tokio::fs::create_dir_all(&java_dir).await
        .map_err(|e| format!("Failed to create Java directory: {}", e))?;

    Ok(format!("Instance directory created: {}", instance_dir.display()))
}

#[tauri::command]
async fn get_required_java_version_command(minecraft_version: String) -> Result<String, String> {
    Ok(get_required_java_version(&minecraft_version))
}

#[tauri::command]
async fn launch_minecraft_with_java(
    instance_id: String,
    _java_path: String,
    minecraft_version: String,
    java_version: String
) -> Result<String, String> {
    // Verify that the provided Java version matches the required version for this Minecraft version
    let required_version = get_required_java_version(&minecraft_version);
    if java_version != required_version {
        return Err(format!("Java version mismatch: required {}, provided {}", required_version, java_version));
    }
    
    // TODO: Implement actual Minecraft launching with specific Java version
    // For now, return success message
    Ok(format!("Minecraft {} launched with Java {} for instance {}", minecraft_version, java_version, instance_id))
}

#[tauri::command]
async fn stop_minecraft_instance(instance_id: String) -> Result<String, String> {
    // TODO: Implement actual Minecraft process termination
    // For now, return success message
    Ok(format!("Minecraft instance {} stopped", instance_id))
}

#[tauri::command]
async fn restart_application() -> Result<String, String> {
    // For now, we'll use window reload as restart functionality
    // In a real implementation, you might want to use Tauri's restart API
    Ok("Application will be restarted".to_string())
}

#[tauri::command]
async fn upload_skin_to_mojang(file_path: String, variant: String, access_token: String) -> Result<String, String> {
    use std::fs;

    println!("Uploading skin to Mojang API...");

    // Validate file exists and is PNG
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    if path.extension().unwrap_or_default() != "png" {
        return Err("File must be a PNG image".to_string());
    }

    // Read file data
    let file_data = fs::read(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // Validate file size (24KB max for Mojang)
    if file_data.len() > 24 * 1024 {
        return Err("Skin file must be smaller than 24KB".to_string());
    }

    // Upload to Mojang API
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

    println!("Skin uploaded successfully to Mojang");
    Ok("Skin uploaded successfully".to_string())
}

#[tauri::command]
async fn set_skin_variant(file_path: String, variant: String, access_token: String) -> Result<String, String> {
    use std::fs;

    println!("Changing skin variant to: {}", variant);

    // Validate file exists and is PNG
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    if path.extension().unwrap_or_default() != "png" {
        return Err("File must be a PNG image".to_string());
    }

    // Read file data
    let file_data = fs::read(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // Validate file size (24KB max for Mojang)
    if file_data.len() > 24 * 1024 {
        return Err("Skin file must be smaller than 24KB".to_string());
    }

    // Re-upload skin with new variant
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

    println!("Skin variant changed successfully to: {}", variant);
    Ok("Skin variant updated".to_string())
}

#[tauri::command]
async fn create_temp_file(file_name: String, file_data: Vec<u8>) -> Result<String, String> {
    use std::fs::File;
    use std::io::Write;

    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(&file_name);

    // Create temp file and write data
    let mut file = File::create(&file_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    file.write_all(&file_data)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;

    println!("Created temp file: {}", file_path.display());
    Ok(file_path.to_string_lossy().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_oauth::init())
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
            create_temp_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
