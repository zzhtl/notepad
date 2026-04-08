use rusqlite::Connection;
use std::path::Path;
use std::sync::mpsc as std_mpsc;
use tokio::sync::oneshot;

use super::migration;

type BoxedOp = Box<dyn FnOnce(&Connection) + Send>;

/// 数据库连接池：单后台线程 + mpsc 通道，绝不阻塞 UI 线程
#[derive(Clone)]
pub struct DbPool {
    sender: std_mpsc::Sender<BoxedOp>,
}

impl DbPool {
    /// 打开数据库，设置性能 PRAGMA，执行迁移，启动后台线程
    pub fn open(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut conn = Connection::open(path)?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = -8000;
             PRAGMA foreign_keys = ON;
             PRAGMA temp_store = MEMORY;",
        )?;

        migration::run(&mut conn)?;

        let (tx, rx) = std_mpsc::channel::<BoxedOp>();

        std::thread::spawn(move || {
            while let Ok(op) = rx.recv() {
                op(&conn);
            }
        });

        Ok(DbPool { sender: tx })
    }

    /// 在后台线程执行数据库操作，返回结果
    pub async fn execute<T, F>(&self, op: F) -> T
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> T + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let boxed: BoxedOp = Box::new(move |conn| {
            let result = op(conn);
            let _ = tx.send(result);
        });
        let _ = self.sender.send(boxed);
        rx.await.expect("数据库线程意外退出")
    }
}
