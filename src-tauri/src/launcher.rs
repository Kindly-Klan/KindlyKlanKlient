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

/// Determina la versión de Java requerida según la versión de Minecraft
pub fn get_required_java_version_for_minecraft(mc_version: &str) -> u8 {
    // Parsear la versión de Minecraft
    let version_parts: Vec<&str> = mc_version.split('.').collect();
    
    if version_parts.len() < 2 {
        return 8; // Por defecto Java 8
    }
    
    let major = version_parts.get(0).and_then(|v| v.parse::<u32>().ok()).unwrap_or(1);
    let minor = version_parts.get(1).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    let patch = version_parts.get(2).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    
    // Minecraft 1.20.5+ requiere Java 21
    if major == 1 && (minor > 20 || (minor == 20 && patch >= 5)) {
        return 21;
    }
    
    // Minecraft 1.18 - 1.20.4 requiere Java 17
    if major == 1 && minor >= 18 && minor <= 20 {
        return 17;
    }
    
    // Minecraft 1.17.x requiere Java 16
    if major == 1 && minor == 17 {
        return 16;
    }
    
    // Minecraft < 1.17 requiere Java 8
    8
}

/// Busca o instala automáticamente el ejecutable de Java requerido para una versión de Minecraft
pub async fn find_java_executable() -> Result<String, String> {
    // Primero intentar encontrar Java en rutas comunes del sistema
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

    // Si no se encuentra, intentar usar Java 8 como fallback desde runtime
    let kindly_dir = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(|p| std::path::PathBuf::from(p))
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(".kindlyklanklient");
    
    let java_8_path = kindly_dir.join("runtime").join("java-8").join("bin").join(if cfg!(target_os = "windows") { "java.exe" } else { "java" });
    
    if java_8_path.exists() {
        return Ok(java_8_path.to_string_lossy().to_string());
    }

    Err("Java executable not found. Please ensure Java is installed.".to_string())
}

/// Busca o instala automáticamente el ejecutable de Java para una versión específica de Minecraft
pub async fn find_or_install_java_for_minecraft(mc_version: &str) -> Result<String, String> {
    let required_java_version = get_required_java_version_for_minecraft(mc_version);
    
    // Verificar si ya existe la versión requerida en runtime
    let kindly_dir = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(|p| std::path::PathBuf::from(p))
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(".kindlyklanklient");
    
    let java_path = kindly_dir
        .join("runtime")
        .join(format!("java-{}", required_java_version))
        .join("bin")
        .join(if cfg!(target_os = "windows") { "java.exe" } else { "java" });
    
    if java_path.exists() {
        log::info!("✅ Java {} encontrado en: {}", required_java_version, java_path.display());
        return Ok(java_path.to_string_lossy().to_string());
    }
    
    log::warn!("⚠️  Java {} no encontrado, se requiere para Minecraft {}", required_java_version, mc_version);
    log::info!("🔽 Descargando Java {} automáticamente...", required_java_version);
    
    // Descargar Java automáticamente sin UI
    download_java_silent(required_java_version).await?;
    
    // Verificar que se instaló correctamente
    if java_path.exists() {
        log::info!("✅ Java {} descargado e instalado correctamente", required_java_version);
        return Ok(java_path.to_string_lossy().to_string());
    }
    
    Err(format!(
        "Error al instalar Java {}. Por favor, intente instalarlo manualmente.",
        required_java_version
    ))
}

/// Descarga e instala Java sin interfaz de usuario
async fn download_java_silent(java_version: u8) -> Result<(), String> {
    let version_str = java_version.to_string();
    
    let kindly_dir = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(|p| std::path::PathBuf::from(p))
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(".kindlyklanklient");
    
    let runtime_dir = kindly_dir.join("runtime");
    let java_dir = runtime_dir.join(format!("java-{}", version_str));
    
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
        version_str, os, arch
    );
    
    log::info!("📥 Descargando Java {} desde: {}", version_str, jre_url);
    
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
        .map_err(|e| format!("Failed to read Java download: {}", e))?;
    
    let temp_file = runtime_dir.join(format!("java-{}.{}", version_str, extension));
    tokio::fs::write(&temp_file, &bytes).await
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    
    log::info!("📦 Extrayendo Java {}...", version_str);
    
    // Extraer el archivo
    if java_dir.exists() {
        let _ = std::fs::remove_dir_all(&java_dir);
    }
    
    if temp_file.extension().map_or(false, |e| e == "zip") {
        let reader = std::fs::File::open(&temp_file)
            .map_err(|e| format!("Open zip failed: {}", e))?;
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|e| format!("Read zip failed: {}", e))?;
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| format!("Zip index failed: {}", e))?;
            let outpath = runtime_dir.join(file.mangled_name());
            
            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)
                    .map_err(|e| format!("Create dir failed: {}", e))?;
            } else {
                if let Some(p) = outpath.parent() {
                    std::fs::create_dir_all(p)
                        .map_err(|e| format!("Create parent failed: {}", e))?;
                }
                let mut outfile = std::fs::File::create(&outpath)
                    .map_err(|e| format!("Create file failed: {}", e))?;
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("Write file failed: {}", e))?;
            }
        }
    }
    
    // Renombrar el directorio extraído al nombre esperado
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
        
        for dir in extracted_dirs.iter().skip(1) {
            let _ = std::fs::remove_dir_all(dir);
        }
    } else {
        return Err("No Java directory found after extraction".to_string());
    }
    
    let _ = std::fs::remove_file(&temp_file);
    
    log::info!("✅ Java {} instalado correctamente", version_str);
    Ok(())
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

pub fn select_main_class(instance_dir: &Path) -> String {
    // Intentar leer el mainClass del version JSON generado por el instalador
    // Los instaladores de Forge/NeoForge/Fabric crean archivos JSON con el mainClass correcto
    
    // Buscar archivos JSON en versions/ que no sean versiones vanilla
    let versions_dir = instance_dir.join("versions");
    if versions_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&versions_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Buscar archivo JSON en este directorio
                    let dir_name = entry.file_name();
                    let json_path = path.join(format!("{}.json", dir_name.to_string_lossy()));
                    
                    if json_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(&json_path) {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                                if let Some(main_class) = json.get("mainClass").and_then(|v| v.as_str()) {
                                    // Detectar el tipo de mod loader por el mainClass
                                    if main_class.contains("neoforge") || main_class.contains("neoforged") {
                                        log::info!("🔨 Detected NeoForge mod loader: {}", main_class);
                                        return main_class.to_string();
                                    } else if main_class.contains("minecraftforge") || main_class.contains("forge") {
                                        log::info!("⚒️  Detected Forge mod loader: {}", main_class);
                                        return main_class.to_string();
                                    } else if main_class.contains("fabricmc") || main_class.contains("fabric") {
                                        log::info!("🧵 Detected Fabric mod loader: {}", main_class);
                                        return main_class.to_string();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Fallback: detectar por directorios de libraries
    let neoforge_loader_dir = instance_dir.join("libraries").join("net").join("neoforged");
    if neoforge_loader_dir.exists() { 
        log::info!("🔨 Detected NeoForge mod loader (fallback)");
        return "cpw.mods.bootstraplauncher.BootstrapLauncher".to_string(); 
    }
    
    let forge_loader_dir = instance_dir.join("libraries").join("net").join("minecraftforge");
    if forge_loader_dir.exists() { 
        log::info!("⚒️  Detected Forge mod loader (fallback)");
        return "cpw.mods.bootstraplauncher.BootstrapLauncher".to_string();
    }
    
	let fabric_loader_dir = instance_dir.join("libraries").join("net").join("fabricmc");
    if fabric_loader_dir.exists() { 
        log::info!("🧵 Detected Fabric mod loader (fallback)");
        return "net.fabricmc.loader.impl.launch.knot.KnotClient".to_string();
    }
    
    log::info!("🎮 Using vanilla Minecraft");
    "net.minecraft.client.main.Main".to_string()
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


