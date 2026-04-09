use std::fs;
use std::path::PathBuf;

const APP_NAME: &str = "notepad";
const ICON_PNG: &[u8] = include_bytes!("../../assets/icon.png");

// ==================== Linux ====================

#[cfg(target_os = "linux")]
fn desktop_file_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("applications").join(format!("{APP_NAME}.desktop")))
}

#[cfg(target_os = "linux")]
fn icon_install_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| {
        d.join("icons")
            .join("hicolor")
            .join("256x256")
            .join("apps")
            .join(format!("{APP_NAME}.png"))
    })
}

/// 检查桌面快捷方式是否已安装且路径一致
#[cfg(target_os = "linux")]
pub fn is_up_to_date() -> bool {
    let Some(desktop_path) = desktop_file_path() else {
        return false;
    };
    let Ok(content) = fs::read_to_string(&desktop_path) else {
        return false;
    };
    let Ok(exe_path) = std::env::current_exe() else {
        return false;
    };
    let expected = format!("Exec={}", exe_path.display());
    content.lines().any(|line| line == expected)
}

/// 生成 .desktop 文件并安装图标
#[cfg(target_os = "linux")]
pub fn install() -> Result<(), String> {
    let exe_path = std::env::current_exe().map_err(|e| format!("无法获取可执行文件路径: {e}"))?;

    // 安装图标
    let icon_path = icon_install_path().ok_or("无法获取图标安装路径")?;
    if let Some(parent) = icon_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("无法创建图标目录: {e}"))?;
    }
    fs::write(&icon_path, ICON_PNG).map_err(|e| format!("无法写入图标文件: {e}"))?;

    // 生成 .desktop 文件
    let desktop_path = desktop_file_path().ok_or("无法获取 .desktop 文件路径")?;
    if let Some(parent) = desktop_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("无法创建目录: {e}"))?;
    }

    let content = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Notepad\n\
         Comment=高性能 Markdown 记事本\n\
         Exec={exe}\n\
         Icon={icon}\n\
         Terminal=false\n\
         StartupWMClass=notepad\n",
        exe = exe_path.display(),
        icon = icon_path.display(),
    );
    fs::write(&desktop_path, content).map_err(|e| format!("无法写入 .desktop 文件: {e}"))?;

    Ok(())
}

// ==================== macOS ====================

/// 获取 .app 包路径: ~/Applications/Notepad.app
#[cfg(target_os = "macos")]
fn app_bundle_path() -> Option<PathBuf> {
    dirs::home_dir().map(|d| d.join("Applications").join("Notepad.app"))
}

/// 获取启动脚本路径
#[cfg(target_os = "macos")]
fn launcher_path() -> Option<PathBuf> {
    app_bundle_path().map(|d| d.join("Contents").join("MacOS").join(APP_NAME))
}

/// 检查 .app 包是否已安装且启动脚本指向当前可执行文件
#[cfg(target_os = "macos")]
pub fn is_up_to_date() -> bool {
    let Some(launcher) = launcher_path() else {
        return false;
    };
    let Ok(script) = fs::read_to_string(&launcher) else {
        return false;
    };
    let Ok(exe_path) = std::env::current_exe() else {
        return false;
    };
    script.contains(&exe_path.display().to_string())
}

/// 生成 macOS .app 包（包含 Info.plist、启动脚本和 .icns 图标）
#[cfg(target_os = "macos")]
pub fn install() -> Result<(), String> {
    let exe_path = std::env::current_exe().map_err(|e| format!("无法获取可执行文件路径: {e}"))?;
    let bundle = app_bundle_path().ok_or("无法获取 .app 包路径")?;

    let contents = bundle.join("Contents");
    let macos_dir = contents.join("MacOS");
    let resources_dir = contents.join("Resources");

    // 创建目录结构
    fs::create_dir_all(&macos_dir).map_err(|e| format!("无法创建 MacOS 目录: {e}"))?;
    fs::create_dir_all(&resources_dir).map_err(|e| format!("无法创建 Resources 目录: {e}"))?;

    // 生成 .icns 图标
    let icns_path = resources_dir.join(format!("{APP_NAME}.icns"));
    generate_icns(&icns_path)?;

    // 创建启动脚本
    let launcher = macos_dir.join(APP_NAME);
    let script = format!("#!/bin/bash\nexec \"{}\" \"$@\"\n", exe_path.display());
    fs::write(&launcher, &script).map_err(|e| format!("无法写入启动脚本: {e}"))?;

    // 设置可执行权限
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o755);
    fs::set_permissions(&launcher, perms).map_err(|e| format!("无法设置执行权限: {e}"))?;

    // 生成 Info.plist
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>{name}</string>
    <key>CFBundleIconFile</key>
    <string>{name}</string>
    <key>CFBundleIdentifier</key>
    <string>com.{name}.app</string>
    <key>CFBundleName</key>
    <string>Notepad</string>
    <key>CFBundleDisplayName</key>
    <string>Notepad</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>"#,
        name = APP_NAME
    );
    fs::write(contents.join("Info.plist"), &plist)
        .map_err(|e| format!("无法写入 Info.plist: {e}"))?;

    Ok(())
}

/// 使用 macOS 系统工具 (sips + iconutil) 将 PNG 转换为 .icns
#[cfg(target_os = "macos")]
fn generate_icns(icns_path: &std::path::Path) -> Result<(), String> {
    use std::process::Command;

    let temp_dir = std::env::temp_dir().join("notepad_iconset");
    let iconset_dir = temp_dir.join(format!("{APP_NAME}.iconset"));

    // 清理并创建 iconset 目录
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&iconset_dir).map_err(|e| format!("无法创建 iconset 目录: {e}"))?;

    // 写入源 PNG
    let src_png = temp_dir.join("icon_src.png");
    fs::write(&src_png, ICON_PNG).map_err(|e| format!("无法写入源图标: {e}"))?;

    // 各尺寸图标（源为 256x256，仅缩小不放大以保证清晰度）
    let sizes: &[(&str, u32)] = &[
        ("icon_16x16.png", 16),
        ("icon_16x16@2x.png", 32),
        ("icon_32x32.png", 32),
        ("icon_32x32@2x.png", 64),
        ("icon_128x128.png", 128),
        ("icon_128x128@2x.png", 256),
        ("icon_256x256.png", 256),
    ];

    for (filename, size) in sizes {
        let dest = iconset_dir.join(filename);
        fs::copy(&src_png, &dest).map_err(|e| format!("无法复制图标: {e}"))?;

        let output = Command::new("sips")
            .args(["-z", &size.to_string(), &size.to_string()])
            .arg(&dest)
            .output()
            .map_err(|e| format!("sips 命令执行失败: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "sips 生成 {filename} 失败: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    // 使用 iconutil 生成 .icns
    let output = Command::new("iconutil")
        .args(["-c", "icns"])
        .arg(&iconset_dir)
        .args(["-o"])
        .arg(icns_path)
        .output()
        .map_err(|e| format!("iconutil 命令执行失败: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "iconutil 生成 .icns 失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // 清理临时文件
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

// ==================== 其他平台（无操作）====================

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn is_up_to_date() -> bool {
    true
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn install() -> Result<(), String> {
    Ok(())
}
