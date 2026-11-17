use iced::Task;

use crate::db::transfer::TransferReport;
use crate::message::Message;
use crate::model::{folder::TreeNode, note::Note};

use super::App;

#[derive(Debug, Clone)]
pub struct TransferOutcome {
    pub report: TransferReport,
    pub tree: Option<Vec<TreeNode>>,
    pub active_note: Option<Note>,
}

impl App {
    pub(crate) fn start_transfer(&mut self, importing: bool) -> Task<Message> {
        self.transfer_busy = Some(if importing {
            "正在导入 Markdown 目录…"
        } else {
            "正在导出全部笔记…"
        });
        self.notice = None;
        self.error = None;
        self.context_menu = None;
        self.editor_context_menu = false;
        let db = self.db.clone();
        let current = self.active_note.as_ref().map(|active| active.note.clone());
        Task::perform(
            async move {
                let handle = rfd::AsyncFileDialog::new()
                    .set_title(if importing {
                        "导入目录：相同路径和文件名将更新，相同内容跳过"
                    } else {
                        "选择保存位置（自动新建导出文件夹）"
                    })
                    .pick_folder()
                    .await;
                let Some(handle) = handle else {
                    return Ok(None);
                };
                let path = handle.path().to_path_buf();
                db.execute(move |conn| {
                    // 保存和传输在同一后台任务中串行执行；导入期间暂停编辑。
                    if let Some(note) = &current {
                        crate::db::note::save_note(conn, note).map_err(|e| e.to_string())?;
                    }
                    let report = if importing {
                        crate::db::transfer::import_all(conn, &path)?
                    } else {
                        crate::db::transfer::export_all(conn, &path)?
                    };
                    let tree = if importing {
                        Some(crate::db::folder::load_root_tree(conn).map_err(|e| e.to_string())?)
                    } else {
                        None
                    };
                    let active_note = if importing {
                        current
                            .map(|note| crate::db::note::load_note(conn, &note.id))
                            .transpose()
                            .map_err(|e| e.to_string())?
                    } else {
                        None
                    };
                    Ok(Some(TransferOutcome {
                        report,
                        tree,
                        active_note,
                    }))
                })
                .await
            },
            Message::TransferFinished,
        )
    }
}
