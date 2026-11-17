#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod db;
mod message;
mod model;
mod ui;
mod util;

use db::DbPool;

fn main() -> iced::Result {
    let paths = util::paths::resolve().expect("无法确定数据库路径");
    // 自动安装桌面快捷方式和图标（仅首次）
    if !paths.installed && !util::desktop::is_up_to_date() {
        if let Err(e) = util::desktop::install() {
            eprintln!("桌面快捷方式安装失败: {e}");
        }
    }

    let db = DbPool::open(&paths.database).expect("无法打开数据库");

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
            min_size: Some(iced::Size::new(800.0, 600.0)),
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
