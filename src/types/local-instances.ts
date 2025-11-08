// Local instances types

export interface LocalInstance {
  id: string;
  name: string;
  minecraft_version: string;
  fabric_version: string;
  created_at: string;
  is_local: boolean;
  background?: string | null;
}

export interface LocalInstanceMetadata {
  id: string;
  name: string;
  minecraft_version: string;
  fabric_version: string;
  created_at: string;
}

// Minecraft version types
export interface MinecraftVersionManifest {
  latest: LatestVersions;
  versions: MinecraftVersionInfo[];
}

export interface LatestVersions {
  release: string;
  snapshot: string;
}

export interface MinecraftVersionInfo {
  id: string;
  type: string;
  url: string;
  time: string;
  releaseTime: string;
}

// Fabric Loader version types
export interface FabricLoaderVersion {
  loader: FabricLoaderInfo;
}

export interface FabricLoaderInfo {
  version: string;
  stable: boolean;
}

// Progress events
export interface LocalInstanceProgress {
  instance_id: string;
  stage: 'starting' | 'minecraft_client' | 'minecraft_libraries' | 'fabric_loader' | 'minecraft_assets' | 'saving_metadata' | 'completed';
  percentage: number;
  message: string;
}

export interface ModSyncProgress {
  local_id: string;
  remote_id: string;
  stage: 'loading_remote' | 'clearing_mods' | 'downloading_mods' | 'completed';
  percentage: number;
  message: string;
}

