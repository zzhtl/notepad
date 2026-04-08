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
    let db = DbPool::open(&db_path()).expect("无法打开数据库");

    iced::application(
        move || app::App::new(db.clone()),
        app::App::update,
        app::App::view,
    )
    .title(app::App::title)
    .subscription(app::App::subscription)
    .theme(app::App::theme)
    .window_size((1200.0, 800.0))
    .run()
}
