use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

/// 执行数据库迁移
pub fn run(conn: &mut Connection) -> Result<(), rusqlite_migration::Error> {
    let m = [M::up(include_str!("../../migrations/001_initial.sql"))];
    let migrations = Migrations::from_slice(&m);
    migrations.to_latest(conn)
}
