mod app;
mod db;
mod message;
mod model;
mod ui;
mod util;

use std::path::PathBuf;

use db::DbPool;

/// 获取数据库路径（应用同目录）
fn db_path() -> PathBuf {
    let exe = std::env::current_exe().expect("无法获取可执行文件路径");
    exe.parent()
        .expect("无法获取可执行文件所在目录")
        .join("notepad.db")
}

fn main() -> iced::Result {
    // 自动安装桌面快捷方式和图标（仅首次）
    if !util::desktop::is_up_to_date() {
        if let Err(e) = util::desktop::install() {
            eprintln!("桌面快捷方式安装失败: {e}");
        }
    }

    let db = DbPool::open(&db_path()).expect("无法打开数据库");

    iced::application(
        move || app::App::new(db.clone()),
        app::App::update,
        app::App::view,
    )
    .title(app::App::title)
    .subscription(app::App::subscription)
    .theme(app::App::theme)
    .window({
        let mut settings = iced::window::Settings {
            size: iced::Size::new(1200.0, 800.0),
            ..Default::default()
        };
        #[cfg(target_os = "linux")]
        {
            settings.platform_specific.application_id = "notepad".to_string();
        }
        settings
    })
    .run()
}
