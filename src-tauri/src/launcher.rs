use anyhow::Result;
use tokio::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::io::AsyncWriteExt;
use crate::versions::{Library, MinecraftVersion, VersionManifest};
use reqwest;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

pub struct MinecraftLauncher {
    pub config: LauncherConfig,
}

impl MinecraftLauncher {
    pub fn new() -> Result<Self> {
        Ok(Self { config: LauncherConfig::new()? })
    }

    // Find Java executable in common locations
    pub fn find_java(&self) -> Result<PathBuf> {
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
        anyhow::bail!("Java not found")
    }

    pub async fn build_classpath(&self, version: &str) -> Result<String> {
        let version_dir = self.config.versions_dir.join(version);
        let version_file = version_dir.join(format!("{}.json", version));
        let version_data = fs::read_to_string(&version_file).await?;
        #[derive(serde::Deserialize)]
        struct VersionJson {
            libraries: Vec<Library>,
        }
        let version_json: VersionJson = serde_json::from_str(&version_data)?;
        let os_name = "windows";
        let mut classpath = Vec::new();
        for lib in &version_json.libraries {
            if !crate::versions::is_library_allowed(lib, os_name) { continue; }
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

    pub async fn get_available_versions(&self) -> Result<Vec<MinecraftVersion>> {
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

    pub async fn download_version(&self, version: &MinecraftVersion) -> Result<()> {
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
        #[derive(serde::Deserialize)]
        struct VersionJson {
            downloads: VersionJsonDownloads,
            libraries: Vec<Library>,
            #[serde(rename = "assetIndex")]
            asset_index: Option<AssetIndex>,
        }
        #[derive(serde::Deserialize)]
        struct VersionJsonDownloads {
            client: Option<DownloadInfo>,
        }
        #[derive(serde::Deserialize)]
        struct DownloadInfo {
            url: String,
        }
        #[derive(serde::Deserialize)]
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
            if !crate::versions::is_library_allowed(lib, os_name) { continue; }
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
            #[derive(serde::Deserialize)]
            struct AssetIndexJson {
                objects: HashMap<String, AssetObject>,
            }
            #[derive(serde::Deserialize, Clone)]
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

    pub async fn launch_minecraft(
        &self,
        version: &str,
        username: &str,
        ram_mb: u32,
        access_token: Option<&str>,
        uuid: Option<&str>
    ) -> Result<()> {
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
        #[derive(serde::Deserialize)]
        struct VersionJson {
            #[serde(rename = "assetIndex")]
            asset_index: Option<AssetIndex>,
        }
        #[derive(serde::Deserialize)]
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
}

pub fn get_total_ram_mb() -> anyhow::Result<u32> {
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
    Ok(4096)
}

pub async fn find_java_executable() -> Result<String, String> {
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

pub fn build_minecraft_classpath(instance_dir: &Path) -> Result<String, String> {
	let mut jars: Vec<String> = Vec::new();
	let libs_dir = instance_dir.join("libraries");
	if libs_dir.exists() { collect_jars_recursively(&libs_dir, &mut jars)?; }
	let versions_dir = instance_dir.join("versions");
	if versions_dir.exists() { collect_jars_recursively(&versions_dir, &mut jars)?; }
	let mods_dir = instance_dir.join("mods");
	if mods_dir.exists() { collect_jars_recursively(&mods_dir, &mut jars)?; }
	if jars.is_empty() { return Err("No jars found for classpath".to_string()); }
	Ok(jars.join(if cfg!(target_os = "windows") { ";" } else { ":" }))
}

fn collect_jars_recursively(dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
	for entry in walkdir::WalkDir::new(dir) {
		let entry = entry.map_err(|e| e.to_string())?;
		if entry.file_type().is_file() {
			let p = entry.into_path();
			if p.extension().map_or(false, |e| e == "jar") { out.push(p.to_string_lossy().to_string()); }
		}
	}
	Ok(())
}

pub fn select_main_class(instance_dir: &Path) -> &'static str {
	let fabric_loader_dir = instance_dir.join("libraries").join("net").join("fabricmc");
    if fabric_loader_dir.exists() { return "net.fabricmc.loader.impl.launch.knot.KnotClient"; }
    "net.minecraft.client.main.Main"
}

pub fn build_minecraft_jvm_args(
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
	match garbage_collector {
		"G1" => { args.extend(vec!["-XX:+UseG1GC".into(), "-XX:G1NewSizePercent=20".into(), "-XX:G1ReservePercent=20".into(), "-XX:MaxGCPauseMillis=50".into(), "-XX:G1HeapRegionSize=32M".into()]); },
		"ZGC" => { args.extend(vec!["-XX:+UseZGC".into(), "-XX:+UnlockExperimentalVMOptions".into()]); },
		"Parallel" => { args.extend(vec!["-XX:+UseParallelGC".into(), "-XX:ParallelGCThreads=4".into()]); },
		_ => { args.extend(vec!["-XX:+UseG1GC".into(), "-XX:G1NewSizePercent=20".into(), "-XX:G1ReservePercent=20".into(), "-XX:MaxGCPauseMillis=50".into(), "-XX:G1HeapRegionSize=32M".into()]); }
	}
	if !additional_jvm_args.trim().is_empty() {
		let additional_args: Vec<&str> = additional_jvm_args.split_whitespace().collect();
		for arg in additional_args { if !arg.is_empty() { args.push(arg.to_string()); } }
	}
	args.push("-Dminecraft.api.auth.host=https://api.minecraftservices.com".to_string());
	args.push("-Dminecraft.api.session.host=https://api.minecraftservices.com".to_string());
	args.push("-Dminecraft.api.services.host=https://api.minecraftservices.com".to_string());
	args.push(format!("-Dminecraft.api.accessToken={}", access_token));
	Ok(args)
}

pub fn get_instance_directory(instance_id: &str) -> PathBuf {
	let base = std::env::var("USERPROFILE")
		.map(|p| std::path::Path::new(&p).join(".kindlyklanklient"))
		.unwrap_or_else(|_| std::path::Path::new(".").join(".kindlyklanklient"));
	base.join(instance_id)
}

// Launcher directory configuration
pub struct LauncherConfig {
    pub minecraft_dir: PathBuf,
    pub versions_dir: PathBuf,
    pub assets_dir: PathBuf,
    pub libraries_dir: PathBuf,
}

impl LauncherConfig {
    pub fn new() -> Result<Self> {
        let home = env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".to_string());
        let minecraft_dir = PathBuf::from(home).join(".kindlyklanklient");
        Ok(Self {
            versions_dir: minecraft_dir.join("versions"),
            assets_dir: minecraft_dir.join("assets"),
            libraries_dir: minecraft_dir.join("libraries"),
            minecraft_dir,
        })
    }

    pub async fn ensure_directories(&self) -> Result<()> {
        fs::create_dir_all(&self.minecraft_dir).await?;
        fs::create_dir_all(&self.versions_dir).await?;
        fs::create_dir_all(&self.assets_dir).await?;
        fs::create_dir_all(&self.libraries_dir).await?;
        Ok(())
    }
}


