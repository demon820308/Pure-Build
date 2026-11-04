use pcl_base::launch::{LaunchArgumentsResult, LaunchVersionJson, MinecraftLauncher};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::{Path, PathBuf};

struct TestTempDir(PathBuf);
impl TestTempDir {
    fn new(prefix: &str) -> Self {
        let p = std::env::temp_dir().join(format!(
            "pcl_launch_snap_{}_{}",
            prefix,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for TestTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct NormalizedSnapshot {
    required_java_major: u32,
    main_class: String,
    classpath_entries: Vec<String>,
    jvm_args: Vec<String>,
    game_args: Vec<String>,
}

fn normalize_args(res: &LaunchArgumentsResult, game_dir: &Path) -> NormalizedSnapshot {
    let game_dir_str = game_dir.to_string_lossy().to_string();
    let normalize_str = |s: &str| -> String {
        s.replace(&game_dir_str, "<GAME_DIR>").replace('\\', "/")
    };

    let cp: Vec<String> = res
        .classpath_entries
        .iter()
        .map(|s| normalize_str(s))
        .collect();

    let jvm: Vec<String> = res
        .jvm_args
        .iter()
        .map(|s| normalize_str(s))
        .collect();

    let game: Vec<String> = res
        .game_args
        .iter()
        .map(|s| normalize_str(s))
        .collect();

    NormalizedSnapshot {
        required_java_major: res.required_java_major,
        main_class: res.main_class.clone(),
        classpath_entries: cp,
        jvm_args: jvm,
        game_args: game,
    }
}

/// 模拟版本依赖库的本地假文件落盘，以便 build_launch_arguments 识别并在 classpath 生效
fn populate_dummy_libraries(game_dir: &Path, ver_json: &LaunchVersionJson) {
    let lib_dir = game_dir.join("libraries");
    for lib in &ver_json.libraries {
        let rel_path = if let Some(path_str) = lib
            .downloads
            .as_ref()
            .and_then(|d| d.artifact.as_ref())
            .and_then(|a| a.path.as_ref())
        {
            MinecraftLauncher::artifact_path_to_pathbuf(path_str)
        } else {
            MinecraftLauncher::maven_to_path(&lib.name)
        };
        let full_path = lib_dir.join(&rel_path);
        if let Some(parent) = full_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = File::create(&full_path);
    }
}

#[test]
fn test_vanilla_1_21_1_launch_args_snapshot() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("vanilla-1.21.1.json");
    let content = fs::read_to_string(&fixture_path).expect("读取 1.21.1 fixture 失败");
    let ver_json: LaunchVersionJson = serde_json::from_str(&content).unwrap();

    let temp = TestTempDir::new("vanilla");
    let game_dir = &temp.0;
    let version_dir = game_dir.join("versions").join("1.21.1");
    fs::create_dir_all(&version_dir).unwrap();
    File::create(version_dir.join("1.21.1.jar")).unwrap();

    populate_dummy_libraries(game_dir, &ver_json);

    let res = MinecraftLauncher::build_launch_arguments(
        &ver_json,
        game_dir,
        "1.21.1",
        "windows",
        "x64",
        Some("10.0.19045"),
        "StevePlayer",
        4096,
        Some("11111111-2222-3333-4444-555555555555"),
        Some("mock_access_token_abc"),
        1280,
        720,
        false,
    )
    .expect("构建原版启动参数失败");

    // 关键项断言
    assert_eq!(res.required_java_major, 21);
    assert_eq!(res.main_class, "net.minecraft.client.main.Main");
    assert!(res.jvm_args.contains(&"-Xmx4096m".to_string()));
    assert!(res.jvm_args.iter().any(|a| a.starts_with("-Djava.library.path=")));
    assert!(res.game_args.contains(&"--username".to_string()));
    assert!(res.game_args.contains(&"StevePlayer".to_string()));
    assert!(res.game_args.contains(&"--version".to_string()));
    assert!(res.game_args.contains(&"1.21.1".to_string()));
    assert!(res.game_args.contains(&"--width".to_string()));
    assert!(res.game_args.contains(&"1280".to_string()));
    assert!(res.game_args.contains(&"--height".to_string()));
    assert!(res.game_args.contains(&"720".to_string()));

    // 验证快照期望文件
    let normalized = normalize_args(&res, game_dir);
    let snapshot_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("expected_vanilla_1.21.1_snapshot.json");

    if !snapshot_path.exists() {
        let serialized = serde_json::to_string_pretty(&normalized).unwrap();
        fs::write(&snapshot_path, serialized).unwrap();
    } else {
        let expected_raw = fs::read_to_string(&snapshot_path).unwrap();
        let expected: NormalizedSnapshot = serde_json::from_str(&expected_raw).unwrap();
        assert_eq!(normalized, expected, "原版 1.21.1 启动参数与黄金快照不符");
    }
}

#[test]
fn test_fabric_1_21_1_launch_args_snapshot() {
    let vanilla_fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("vanilla-1.21.1.json");
    let fabric_fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("fabric-1.21.1.json");

    let parent_json: LaunchVersionJson =
        serde_json::from_str(&fs::read_to_string(vanilla_fixture).unwrap()).unwrap();
    let child_json: LaunchVersionJson =
        serde_json::from_str(&fs::read_to_string(fabric_fixture).unwrap()).unwrap();

    let merged_json = MinecraftLauncher::merge_inherited_json(parent_json, child_json);

    let temp = TestTempDir::new("fabric");
    let game_dir = &temp.0;
    let version_dir = game_dir.join("versions").join("fabric-loader-0.16.5-1.21.1");
    fs::create_dir_all(&version_dir).unwrap();

    // 父版本 jar
    let parent_dir = game_dir.join("versions").join("1.21.1");
    fs::create_dir_all(&parent_dir).unwrap();
    File::create(parent_dir.join("1.21.1.jar")).unwrap();

    populate_dummy_libraries(game_dir, &merged_json);

    let res = MinecraftLauncher::build_launch_arguments(
        &merged_json,
        game_dir,
        "fabric-loader-0.16.5-1.21.1",
        "osx",
        "arm64",
        None,
        "FabricPlayer",
        6144,
        None,
        None,
        0,
        0,
        false,
    )
    .expect("构建 Fabric 启动参数失败");

    // 关键项断言
    assert_eq!(res.required_java_major, 21);
    assert_eq!(res.main_class, "net.fabricmc.loader.impl.launch.knot.KnotClient");
    assert!(res.jvm_args.contains(&"-Xmx6144m".to_string()));
    assert!(res.jvm_args.contains(&"-XstartOnFirstThread".to_string())); // macOS 专属 GLFW 标志
    assert!(res.game_args.contains(&"FabricPlayer".to_string()));

    let normalized = normalize_args(&res, game_dir);
    let snapshot_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("expected_fabric_1.21.1_snapshot.json");

    if !snapshot_path.exists() {
        let serialized = serde_json::to_string_pretty(&normalized).unwrap();
        fs::write(&snapshot_path, serialized).unwrap();
    } else {
        let expected_raw = fs::read_to_string(&snapshot_path).unwrap();
        let expected: NormalizedSnapshot = serde_json::from_str(&expected_raw).unwrap();
        assert_eq!(normalized, expected, "Fabric 1.21.1 启动参数与黄金快照不符");
    }
}

#[test]
fn test_neoforge_1_21_1_launch_args_snapshot() {
    let vanilla_fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("vanilla-1.21.1.json");
    let neoforge_fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("neoforge-1.21.1.json");

    let parent_json: LaunchVersionJson =
        serde_json::from_str(&fs::read_to_string(vanilla_fixture).unwrap()).unwrap();
    let child_json: LaunchVersionJson =
        serde_json::from_str(&fs::read_to_string(neoforge_fixture).unwrap()).unwrap();

    let merged_json = MinecraftLauncher::merge_inherited_json(parent_json, child_json);

    let temp = TestTempDir::new("neoforge");
    let game_dir = &temp.0;
    let version_dir = game_dir.join("versions").join("neoforge-21.1.65");
    fs::create_dir_all(&version_dir).unwrap();

    let parent_dir = game_dir.join("versions").join("1.21.1");
    fs::create_dir_all(&parent_dir).unwrap();
    File::create(parent_dir.join("1.21.1.jar")).unwrap();

    populate_dummy_libraries(game_dir, &merged_json);

    let res = MinecraftLauncher::build_launch_arguments(
        &merged_json,
        game_dir,
        "neoforge-21.1.65",
        "linux",
        "x64",
        None,
        "NeoPlayer",
        8192,
        None,
        None,
        1920,
        1080,
        false,
    )
    .expect("构建 NeoForge 启动参数失败");

    // 关键项断言
    assert_eq!(res.required_java_major, 21);
    assert_eq!(res.main_class, "cpw.mods.bootstraplauncher.BootstrapLauncher");
    assert!(res.jvm_args.contains(&"-Xmx8192m".to_string()));
    assert!(res.jvm_args.contains(&"-Dneoforge.logging.markers=REGISTRIES".to_string()));
    assert!(res.jvm_args.contains(&"-Dneoforge.logging.console.level=debug".to_string()));
    assert!(res.game_args.contains(&"--fml.neoForgeVersion".to_string()));
    assert!(res.game_args.contains(&"21.1.65".to_string()));

    let normalized = normalize_args(&res, game_dir);
    let snapshot_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("expected_neoforge_1.21.1_snapshot.json");

    if !snapshot_path.exists() {
        let serialized = serde_json::to_string_pretty(&normalized).unwrap();
        fs::write(&snapshot_path, serialized).unwrap();
    } else {
        let expected_raw = fs::read_to_string(&snapshot_path).unwrap();
        let expected: NormalizedSnapshot = serde_json::from_str(&expected_raw).unwrap();
        assert_eq!(normalized, expected, "NeoForge 1.21.1 启动参数与黄金快照不符");
    }
}

#[test]
fn test_launch_args_cross_platform_flags_across_os() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("vanilla-1.21.1.json");
    let content = fs::read_to_string(&fixture_path).unwrap();
    let ver_json: LaunchVersionJson = serde_json::from_str(&content).unwrap();

    let temp = TestTempDir::new("platform_vectors");
    let game_dir = &temp.0;
    let version_dir = game_dir.join("versions").join("1.21.1");
    fs::create_dir_all(&version_dir).unwrap();
    File::create(version_dir.join("1.21.1.jar")).unwrap();
    populate_dummy_libraries(game_dir, &ver_json);

    // Vector 1: macOS Apple Silicon
    let mac_res = MinecraftLauncher::build_launch_arguments(
        &ver_json,
        game_dir,
        "1.21.1",
        "osx",
        "arm64",
        None,
        "MacTester",
        4096,
        None,
        None,
        0,
        0,
        false,
    )
    .unwrap();
    assert!(mac_res.jvm_args.contains(&"-XstartOnFirstThread".to_string()));
    assert!(!mac_res.jvm_args.iter().any(|a| a.contains("HeapDumpPath")));
    assert!(mac_res.classpath.contains("lwjgl-3.3.3-natives-macos.jar"));
    assert!(!mac_res.classpath.contains("lwjgl-3.3.3-natives-windows.jar"));

    // Vector 2: Windows 10 x64
    let win_res = MinecraftLauncher::build_launch_arguments(
        &ver_json,
        game_dir,
        "1.21.1",
        "windows",
        "x64",
        Some("10.0.19041"),
        "WinTester",
        4096,
        None,
        None,
        0,
        0,
        false,
    )
    .unwrap();
    assert!(!win_res.jvm_args.contains(&"-XstartOnFirstThread".to_string()));
    assert!(win_res.jvm_args.iter().any(|a| a.contains("HeapDumpPath")));
    assert!(win_res.jvm_args.contains(&"-Dos.name=Windows 10".to_string()));
    assert!(win_res.classpath.contains("lwjgl-3.3.3-natives-windows.jar"));
    assert!(!win_res.classpath.contains("lwjgl-3.3.3-natives-macos.jar"));

    // Vector 3: Linux x64
    let linux_res = MinecraftLauncher::build_launch_arguments(
        &ver_json,
        game_dir,
        "1.21.1",
        "linux",
        "x64",
        None,
        "LinuxTester",
        4096,
        None,
        None,
        0,
        0,
        false,
    )
    .unwrap();
    assert!(!linux_res.jvm_args.contains(&"-XstartOnFirstThread".to_string()));
    assert!(!linux_res.jvm_args.iter().any(|a| a.contains("HeapDumpPath")));
    assert!(linux_res.classpath.contains("lwjgl-3.3.3-natives-linux.jar"));
    assert!(!linux_res.classpath.contains("lwjgl-3.3.3-natives-windows.jar"));
}

#[test]
fn test_load_version_json_inheritance_in_tempdir() {
    let temp = TestTempDir::new("inheritance_io");
    let game_dir = &temp.0;
    let versions_dir = game_dir.join("versions");

    // 创建父版本
    let parent_dir = versions_dir.join("1.21.1");
    fs::create_dir_all(&parent_dir).unwrap();
    let parent_fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("vanilla-1.21.1.json");
    fs::copy(parent_fixture, parent_dir.join("1.21.1.json")).unwrap();

    // 创建子版本（继承自 1.21.1）
    let child_dir = versions_dir.join("fabric-1.21.1");
    fs::create_dir_all(&child_dir).unwrap();
    let fabric_fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("fabric-1.21.1.json");
    fs::copy(fabric_fixture, child_dir.join("fabric-1.21.1.json")).unwrap();

    // 从磁盘实际加载并合并
    let loaded = MinecraftLauncher::load_version_json_with_inheritance(
        game_dir,
        &child_dir,
        "fabric-1.21.1",
    )
    .expect("磁盘继承合并加载失败");

    assert_eq!(loaded.id, "fabric-loader-0.16.5-1.21.1");
    assert_eq!(loaded.main_class, "net.fabricmc.loader.impl.launch.knot.KnotClient");
    assert_eq!(loaded.inherits_from, Some("1.21.1".to_string()));
    assert_eq!(loaded.java_version.and_then(|j| j.major_version), Some(21));
    // 包含父版本库与子版本追加库
    assert!(loaded.libraries.iter().any(|lib| lib.name == "com.mojang:authlib:6.0.54"));
    assert!(loaded.libraries.iter().any(|lib| lib.name == "net.fabricmc:fabric-loader:0.16.5"));
}
