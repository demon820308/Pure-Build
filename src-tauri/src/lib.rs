use pcl_base::{
    platform, AppConfig, CacheCleanResult, CommunityManager, ConfigManager, DeviceCodeResponse,
    DownloadManager, EndpointPingResult, FabricMetaSummary, JavaInfo, JavaScanner, LanRoomInfo,
    LinkManager, LoaderVersionItem, MicrosoftAuth, MinecraftLauncher, ModInfo, ModScanner,
    ModpackManager, ModrinthProject, MusicTrack, PlatformInfo, RemoteVersion, ResourceItem,
    ResourceScanner, SaveInfo, UserProfile, VersionInfo, VersionInstaller, VersionScanner,
};
use tauri::{AppHandle, Manager};

#[tauri::command]
fn get_platform_info() -> PlatformInfo {
    platform::get_platform_info()
}

#[tauri::command]
async fn get_installed_java() -> Vec<JavaInfo> {
    tauri::async_runtime::spawn_blocking(JavaScanner::scan_all)
        .await
        .unwrap_or_default()
}

#[tauri::command]
async fn get_minecraft_versions(game_dir: String) -> Vec<VersionInfo> {
    tauri::async_runtime::spawn_blocking(move || VersionScanner::scan_versions(&game_dir))
        .await
        .unwrap_or_default()
}

#[tauri::command]
async fn get_installed_mods(version_path: String, game_dir: String) -> Vec<ModInfo> {
    tauri::async_runtime::spawn_blocking(move || ModScanner::scan_mods(&version_path, &game_dir))
        .await
        .unwrap_or_default()
}

#[tauri::command]
fn toggle_mod_state(mods_dir: String, file_name: String) -> Result<String, String> {
    ModScanner::toggle_mod(&mods_dir, &file_name)
}

#[tauri::command]
fn toggle_instance_item(
    version_path: String,
    game_dir: String,
    folder_name: String,
    file_name: String,
) -> Result<String, String> {
    let target_dir = ResourceScanner::resolve_resource_folder(&version_path, &game_dir, &folder_name);
    ModScanner::toggle_mod(&target_dir, &file_name)
}

#[tauri::command]
fn delete_instance_item(
    version_path: String,
    game_dir: String,
    folder_name: String,
    file_name: String,
) -> Result<(), String> {
    let target_dir = ResourceScanner::resolve_resource_folder(&version_path, &game_dir, &folder_name);
    ModScanner::delete_item(&target_dir, &file_name)
}

#[tauri::command]
fn open_instance_folder(
    version_path: String,
    game_dir: String,
    folder_name: String,
) -> Result<(), String> {
    let target_dir = ResourceScanner::resolve_resource_folder(&version_path, &game_dir, &folder_name);
    ResourceScanner::open_folder(&target_dir)
}

#[tauri::command]
async fn get_resources(version_path: String, game_dir: String, folder_name: String) -> Vec<ResourceItem> {
    tauri::async_runtime::spawn_blocking(move || {
        ResourceScanner::scan_folder(&version_path, &game_dir, &folder_name)
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
async fn get_instance_saves(version_path: String, game_dir: String) -> Vec<SaveInfo> {
    tauri::async_runtime::spawn_blocking(move || {
        ResourceScanner::scan_saves(&version_path, &game_dir)
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
fn open_folder(folder_path: String) -> Result<(), String> {
    ResourceScanner::open_folder(&folder_path)
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    platform::open_url(&url)
}

#[tauri::command]
async fn scan_music_folders(custom_dirs: Vec<String>) -> Vec<MusicTrack> {
    tauri::async_runtime::spawn_blocking(move || ResourceScanner::scan_music_folders(&custom_dirs))
        .await
        .unwrap_or_default()
}

#[tauri::command]
async fn read_music_data_url(file_path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || ResourceScanner::read_music_data_url(&file_path))
        .await
        .map_err(|e| e.to_string())?
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
async fn fetch_optifine_loaders(game_version: String) -> Result<Vec<LoaderVersionItem>, String> {
    DownloadManager::fetch_optifine_loaders(&game_version).await
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
) -> Result<String, String> {
    VersionInstaller::install_server(app, target_dir, version_id, meta_url).await
}

#[tauri::command]
async fn import_modpack(
    app: AppHandle,
    game_dir: String,
    mrpack_path: String,
    custom_instance_name: Option<String>,
) -> Result<String, String> {
    ModpackManager::import_mrpack(app, game_dir, mrpack_path, custom_instance_name).await
}

#[tauri::command]
fn export_modpack(
    version_path: String,
    game_dir: String,
    version_id: String,
    pack_name: String,
    pack_version: String,
    output_file: String,
) -> Result<String, String> {
    ModpackManager::export_instance_to_mrpack(
        &version_path,
        &game_dir,
        &version_id,
        &pack_name,
        &pack_version,
        &output_file,
    )
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
async fn login_yggdrasil_account(
    api_root: String,
    username: String,
    password: String,
) -> Result<UserProfile, String> {
    MicrosoftAuth::login_yggdrasil(&api_root, &username, &password).await
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
fn load_user_profile() -> Option<UserProfile> {
    ConfigManager::load_profile()
}

#[tauri::command]
fn save_user_profile(profile: Option<UserProfile>) -> Result<(), String> {
    ConfigManager::save_profile(profile.as_ref())
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

#[tauri::command]
async fn scan_lan_rooms() -> Result<Vec<LanRoomInfo>, String> {
    LinkManager::scan_lan_rooms(1500).await
}

#[tauri::command]
async fn broadcast_lan_room(
    motd: String,
    host_ip: String,
    port: u16,
) -> Result<LanRoomInfo, String> {
    LinkManager::broadcast_lan_room(&motd, &host_ip, port).await
}

#[tauri::command]
fn decode_lan_invite_code(code: String) -> Result<(String, u16), String> {
    LinkManager::decode_invite_code(&code)
}

#[tauri::command]
async fn ping_download_endpoints() -> Vec<EndpointPingResult> {
    LinkManager::ping_download_endpoints().await
}

#[tauri::command]
fn clean_game_temp_cache(game_dir: String) -> Result<CacheCleanResult, String> {
    LinkManager::clean_game_temp_cache(&game_dir)
}

#[tauri::command]
fn show_main_window(window: tauri::WebviewWindow) -> Result<(), String> {
    let _ = window.show();
    let _ = window.set_focus();
    Ok(())
}

#[tauri::command]
fn minimize_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
fn close_window(app: AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_plugin = tauri_plugin_log::Builder::new()
        .targets([
            tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
            tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
        ])
        .build();

    tauri::Builder::default()
        .plugin(log_plugin)
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_platform_info,
            get_installed_java,
            get_minecraft_versions,
            get_installed_mods,
            toggle_mod_state,
            toggle_instance_item,
            delete_instance_item,
            open_instance_folder,
            get_resources,
            get_instance_saves,
            open_folder,
            open_url,
            scan_music_folders,
            read_music_data_url,
            fetch_remote_versions,
            fetch_fabric_meta,
            fetch_forge_loaders,
            fetch_neoforge_loaders,
            fetch_optifine_games,
            fetch_optifine_loaders,
            start_install_version,
            download_minecraft_server,
            import_modpack,
            export_modpack,
            search_modrinth_resources,
            install_modrinth_resource,
            get_microsoft_device_code,
            poll_microsoft_token,
            login_yggdrasil_account,
            load_app_config,
            save_app_config,
            load_user_profile,
            save_user_profile,
            launch_minecraft,
            kill_game_process,
            scan_lan_rooms,
            broadcast_lan_room,
            decode_lan_invite_code,
            ping_download_endpoints,
            clean_game_temp_cache,
            show_main_window,
            minimize_window,
            close_window
        ])
        .setup(|app| {
            // 注入 Tauri 标准 app_config_dir（遵循规范 §2）
            if let Ok(cfg_dir) = app.path().app_config_dir() {
                platform::set_app_config_dir(cfg_dir);
            }

            // 遵循规范 §3.6：macOS 窗口恢复原生红绿灯装饰（platform 层之外唯一登记的窗口创建 cfg）
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_maximizable(false);
                #[cfg(target_os = "macos")]
                {
                    let _ = window.set_decorations(true);
                }

                // 兜底机制：若前端异常未在 1.5s 内触发首帧 show_main_window，则自动显示窗口防止隐身
                let win_clone = window.clone();
                tauri::async_runtime::spawn(async move {
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                    if !win_clone.is_visible().unwrap_or(true) {
                        let _ = win_clone.show();
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            platform::report_startup_error(&e.to_string());
        });
}
