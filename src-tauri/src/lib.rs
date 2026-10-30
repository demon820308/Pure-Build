use pcl_base::{AppConfig, CommunityManager, ConfigManager, DeviceCodeResponse, DownloadManager, FabricMetaSummary, JavaInfo, JavaScanner, LoaderVersionItem, MicrosoftAuth, MinecraftLauncher, ModInfo, ModScanner, ModrinthProject, RemoteVersion, ResourceItem, ResourceScanner, UserProfile, VersionInfo, VersionInstaller, VersionScanner};
use tauri::{AppHandle, Manager};

#[tauri::command]
fn get_installed_java() -> Vec<JavaInfo> {
    JavaScanner::scan_all()
}

#[tauri::command]
fn get_minecraft_versions(game_dir: String) -> Vec<VersionInfo> {
    VersionScanner::scan_versions(&game_dir)
}

#[tauri::command]
fn get_installed_mods(version_path: String, game_dir: String) -> Vec<ModInfo> {
    ModScanner::scan_mods(&version_path, &game_dir)
}

#[tauri::command]
fn toggle_mod_state(mods_dir: String, file_name: String) -> Result<String, String> {
    ModScanner::toggle_mod(&mods_dir, &file_name)
}

#[tauri::command]
fn get_resources(version_path: String, game_dir: String, folder_name: String) -> Vec<ResourceItem> {
    ResourceScanner::scan_folder(&version_path, &game_dir, &folder_name)
}

#[tauri::command]
fn open_in_explorer(folder_path: String) -> Result<(), String> {
    ResourceScanner::open_folder(&folder_path)
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &url])
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[tauri::command]
async fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
    DownloadManager::fetch_version_manifest().await
}

#[tauri::command]
async fn fetch_fabric_meta() -> Result<FabricMetaSummary, String> {
    DownloadManager::fetch_fabric_meta().await
}

#[tauri::command]
async fn fetch_forge_loaders(game_version: String) -> Result<Vec<LoaderVersionItem>, String> {
    DownloadManager::fetch_forge_loaders(&game_version).await
}

#[tauri::command]
async fn fetch_neoforge_loaders(game_version: String) -> Result<Vec<LoaderVersionItem>, String> {
    DownloadManager::fetch_neoforge_loaders(&game_version).await
}

#[tauri::command]
async fn fetch_optifine_games() -> Result<Vec<String>, String> {
    DownloadManager::fetch_optifine_games().await
}

#[tauri::command]
async fn start_install_version(
    app: AppHandle,
    game_dir: String,
    vanilla_version: String,
    target_name: Option<String>,
    meta_url: String,
    loader_type: Option<String>,
    loader_version: Option<String>,
) -> Result<(), String> {
    VersionInstaller::install_version(
        app,
        game_dir,
        vanilla_version,
        target_name,
        meta_url,
        loader_type,
        loader_version,
    )
    .await
}

#[tauri::command]
async fn download_minecraft_server(
    app: AppHandle,
    target_dir: String,
    version_id: String,
    meta_url: String,
) -> Result<(), String> {
    VersionInstaller::install_server(app, target_dir, version_id, meta_url).await
}

#[tauri::command]
async fn get_microsoft_device_code() -> Result<DeviceCodeResponse, String> {
    MicrosoftAuth::get_device_code().await
}

#[tauri::command]
async fn poll_microsoft_token(device_code: String) -> Result<UserProfile, String> {
    MicrosoftAuth::poll_for_token(device_code).await
}

#[tauri::command]
async fn search_modrinth_resources(
    query: String,
    project_type: String,
    game_version: Option<String>,
    loader: Option<String>,
) -> Result<Vec<ModrinthProject>, String> {
    CommunityManager::search_resources(
        &query,
        &project_type,
        game_version.as_deref(),
        loader.as_deref(),
    )
    .await
}

#[tauri::command]
async fn install_modrinth_resource(
    app: AppHandle,
    game_dir: String,
    version_path: Option<String>,
    project_id: String,
    project_type: String,
    game_version: Option<String>,
    loader: Option<String>,
) -> Result<String, String> {
    CommunityManager::install_resource(
        app,
        game_dir,
        version_path,
        project_id,
        project_type,
        game_version,
        loader,
    )
    .await
}

#[tauri::command]
fn load_app_config() -> AppConfig {
    ConfigManager::load()
}

#[tauri::command]
fn save_app_config(config: AppConfig) -> Result<(), String> {
    ConfigManager::save(&config)
}

#[tauri::command]
fn launch_minecraft(
    app: AppHandle,
    game_dir: String,
    version_id: String,
    java_path: String,
    player_name: String,
    max_memory_mb: u32,
    uuid: Option<String>,
    access_token: Option<String>,
) -> Result<String, String> {
    MinecraftLauncher::launch_with_auth(
        app,
        &game_dir,
        &version_id,
        &java_path,
        &player_name,
        max_memory_mb,
        uuid.as_deref(),
        access_token.as_deref(),
    )
}

#[tauri::command]
fn kill_game_process(pid: u32) -> Result<(), String> {
    MinecraftLauncher::kill_process(pid)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_installed_java,
            get_minecraft_versions,
            get_installed_mods,
            toggle_mod_state,
            get_resources,
            open_in_explorer,
            open_url,
            fetch_remote_versions,
            fetch_fabric_meta,
            fetch_forge_loaders,
            fetch_neoforge_loaders,
            fetch_optifine_games,
            start_install_version,
            download_minecraft_server,
            search_modrinth_resources,
            install_modrinth_resource,
            get_microsoft_device_code,
            poll_microsoft_token,
            load_app_config,
            save_app_config,
            launch_minecraft,
            kill_game_process
        ])
        .setup(|app| {
            if let Some(_window) = app.get_webview_window("main") {
                #[cfg(target_os = "windows")]
                {
                    // Windows 原生窗口支持
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
