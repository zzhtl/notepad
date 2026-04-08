use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use iced::Task;
use iced::widget::{markdown, scrollable, text_editor};

use crate::message::Message;
use crate::model::{folder::TreeNode, note::Note};

use super::tree_ops;
use super::{ActiveNote, App, PendingCreate, PendingNoteJump, PreviewSyncMode, PreviewSyncTarget};

/// pending_create 内联输入框 ID
pub fn pending_input_id() -> iced::widget::Id {
    iced::widget::Id::new("pending-create-input")
}

/// 预览面板 scrollable 的 ID
fn preview_scrollable_id() -> iced::widget::Id {
    iced::widget::Id::new("preview-scrollable")
}

const PREVIEW_SYNC_TOLERANCE: f32 = 4.0;

/// 查看模式下：聚焦编辑器让 text_editor 自动滚动到光标位置
fn focus_readonly_editor() -> Task<Message> {
    iced::widget::operation::focus(crate::ui::editor::editor_id())
}

/// 预处理内容用于预览：每行末尾补两个空格，使单个换行在 markdown 中渲染为硬换行
/// 代码块内和空行不处理
fn preprocess_for_preview(content: &str) -> String {
    let mut result = String::with_capacity(content.len() + content.len() / 10);
    let mut in_code_block = false;

    for (i, line) in content.split('\n').enumerate() {
        if i > 0 {
            result.push('\n');
        }

        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            result.push_str(line);
            continue;
        }

        result.push_str(line);

        // 非代码块、非空行：补两个尾部空格触发 HardBreak
        if !in_code_block && !trimmed.is_empty() && !line.ends_with("  ") {
            result.push_str("  ");
        }
    }

    result
}

/// 重建内容衍生数据
fn rebuild_content_derived(active: &mut ActiveNote) {
    let preview_content = preprocess_for_preview(&active.note.content);
    active.markdown_items = markdown::parse(&preview_content).collect();
}

fn preview_sync_mode(editing: bool) -> PreviewSyncMode {
    if editing {
        PreviewSyncMode::FollowCursor
    } else {
        PreviewSyncMode::Once
    }
}

fn clamped_position(
    content: &text_editor::Content,
    line: usize,
    column: usize,
) -> text_editor::Position {
    let total_lines = content.line_count().max(1);
    let line = line.min(total_lines.saturating_sub(1));
    let max_column = content
        .line(line)
        .map(|current| current.text.chars().count())
        .unwrap_or(0);

    text_editor::Position {
        line,
        column: column.min(max_column),
    }
}

fn move_cursor_to_line(
    active: &mut ActiveNote,
    line: usize,
    column: usize,
    match_len: usize,
    mode: PreviewSyncMode,
) {
    let position = clamped_position(&active.content, line, column);

    // 高亮选中匹配文本
    let selection = if match_len > 0 {
        Some(clamped_position(
            &active.content,
            line,
            column + match_len,
        ))
    } else {
        None
    };

    active.content.move_to(text_editor::Cursor {
        position,
        selection,
    });
    active.preview_target = Some(PreviewSyncTarget {
        line: position.line,
        mode,
    });
}

fn sync_preview_to_cursor(active: &mut ActiveNote) {
    let line = active.content.cursor().position.line;
    active.preview_target = Some(PreviewSyncTarget {
        line,
        mode: PreviewSyncMode::FollowCursor,
    });
}

fn preview_offset_for_line(
    line: usize,
    total_lines: usize,
    content_height: f32,
    viewport_height: f32,
) -> Option<scrollable::AbsoluteOffset> {
    let scrollable_height = content_height - viewport_height;

    if scrollable_height <= 0.0 || content_height <= 0.0 || viewport_height <= 0.0 {
        return None;
    }

    let total_lines = total_lines.max(1);
    let clamped_line = line.min(total_lines.saturating_sub(1));
    let ratio = if total_lines <= 1 {
        0.0
    } else {
        (clamped_line as f32 + 0.5) / total_lines as f32
    };

    let y = (ratio * content_height - viewport_height / 2.0).clamp(0.0, scrollable_height);

    Some(scrollable::AbsoluteOffset { x: 0.0, y })
}

fn sync_preview_task(active: &ActiveNote) -> Task<Message> {
    let Some(target) = active.preview_target else {
        return Task::none();
    };

    let Some(offset) = preview_offset_for_line(
        target.line,
        active.content.line_count(),
        active.preview_content_height,
        active.preview_viewport_height,
    ) else {
        return Task::none();
    };

    iced::widget::operation::scroll_to(preview_scrollable_id(), offset)
}

impl App {
    fn take_pending_note_jump(&mut self, note_id: &str) -> Option<PendingNoteJump> {
        match self.pending_note_jump.take() {
            Some(jump) if jump.note_id == note_id => Some(jump),
            Some(jump) => {
                self.pending_note_jump = Some(jump);
                None
            }
            None => None,
        }
    }

    fn open_note(&mut self, note: Note, editing: bool) -> Task<Message> {
        let note_id = note.id.clone();
        let jump = self.take_pending_note_jump(&note_id);
        let mut content = text_editor::Content::with_text(&note.content);

        // 搜索导航：提取高亮查询词和目标行（在 jump 被消费前）
        let (highlight_query, highlight_line) = if let Some(ref j) = jump {
            (
                if self.search_active {
                    Some(self.search_query.clone())
                } else {
                    None
                },
                Some(j.line),
            )
        } else {
            (None, None)
        };

        let preview_target = if let Some(jump) = jump {
            let position = clamped_position(&content, jump.line, jump.column);
            let selection = if jump.match_len > 0 {
                Some(clamped_position(
                    &content,
                    jump.line,
                    jump.column + jump.match_len,
                ))
            } else {
                None
            };
            content.move_to(text_editor::Cursor {
                position,
                selection,
            });
            Some(PreviewSyncTarget {
                line: position.line,
                mode: preview_sync_mode(editing),
            })
        } else if editing {
            Some(PreviewSyncTarget {
                line: content.cursor().position.line,
                mode: PreviewSyncMode::FollowCursor,
            })
        } else {
            None
        };
        let preview_content = preprocess_for_preview(&note.content);
        let items = markdown::parse(&preview_content).collect();

        self.active_note = Some(ActiveNote {
            content,
            markdown_items: items,
            note,
            images: HashMap::new(),
            dirty: false,
            last_edit: Instant::now(),
            preview_content_height: 0.0,
            preview_viewport_height: 0.0,
            preview_target,
            editing,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_undo_push: Instant::now() - std::time::Duration::from_secs(10),
            highlight_query,
            highlight_line,
        });

        // 异步加载图片
        let db = self.db.clone();
        let load_images_task = Task::perform(
            async move {
                db.execute(move |conn| crate::db::image::load_images(conn, &note_id))
                    .await
            },
            |result| match result {
                Ok(images) => Message::ImagesLoaded(images),
                Err(e) => Message::DbError(e.to_string()),
            },
        );

        // 查看模式下从搜索导航：滚动到目标行
        if !editing
            && highlight_line.is_some()
        {
            return Task::batch([load_images_task, focus_readonly_editor()]);
        }

        load_images_task
    }
}

impl App {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TreeLoaded(tree) => {
                self.tree = tree;
                Task::none()
            }

            Message::ToggleFolder(id) => {
                // 点击文件夹同时选中，便于「+ 笔记」在该文件夹下创建
                self.selected_id = Some(id.clone());
                let db = self.db.clone();
                tree_ops::toggle_in_tree(&mut self.tree, &id, &db).unwrap_or(Task::none())
            }

            Message::FolderChildrenLoaded(folder_id, children) => {
                tree_ops::set_children(&mut self.tree, &folder_id, children);
                Task::none()
            }

            Message::SelectNote(id) => {
                // 双击检测
                let is_double_click =
                    self.last_note_click
                        .as_ref()
                        .is_some_and(|(last_id, time)| {
                            last_id == &id && time.elapsed().as_millis() < 400
                        });
                self.last_note_click = Some((id.clone(), Instant::now()));

                // 双击进入编辑模式
                if is_double_click {
                    if let Some(active) = &mut self.active_note
                        && active.note.id == id
                    {
                        active.editing = true;
                        sync_preview_to_cursor(active);
                        return sync_preview_task(active);
                    }
                    // 不同笔记的双击：加载并直接进入编辑
                    self.pending_note_jump = None;
                    self.search_active = false;
                    self.search_query.clear();
                    self.search_results.clear();
                    self.selected_id = Some(id.clone());
                    let save_task = self.save_if_dirty();
                    let db = self.db.clone();
                    let load_task = Task::perform(
                        async move {
                            db.execute(move |conn| crate::db::note::load_note(conn, &id))
                                .await
                        },
                        |result| match result {
                            Ok(note) => Message::NoteLoadedEditing(note),
                            Err(e) => Message::DbError(e.to_string()),
                        },
                    );
                    return Task::batch([save_task, load_task]);
                }

                // 单击同一已激活笔记：切回查看模式
                if let Some(active) = &mut self.active_note
                    && active.note.id == id
                {
                    self.pending_note_jump = None;
                    self.selected_id = Some(id);
                    if active.editing {
                        active.editing = false;
                        active.preview_target = None;
                        active.highlight_query = None;
                        active.highlight_line = None;
                    }
                    return Task::none();
                }

                // 清空搜索状态
                self.pending_note_jump = None;
                self.search_active = false;
                self.search_query.clear();
                self.search_results.clear();

                self.selected_id = Some(id.clone());
                let save_task = self.save_if_dirty();
                let db = self.db.clone();
                let load_task = Task::perform(
                    async move {
                        db.execute(move |conn| crate::db::note::load_note(conn, &id))
                            .await
                    },
                    |result| match result {
                        Ok(note) => Message::NoteLoaded(note),
                        Err(e) => Message::DbError(e.to_string()),
                    },
                );
                Task::batch([save_task, load_task])
            }

            Message::OpenSearchResult(result) => {
                let jump = PendingNoteJump {
                    note_id: result.note_id.clone(),
                    line: result.match_line.unwrap_or(0),
                    column: result.match_column.unwrap_or(0),
                    match_len: result.match_len,
                };

                self.selected_id = Some(result.note_id.clone());
                self.context_menu = None;

                if let Some(active) = &mut self.active_note
                    && active.note.id == result.note_id
                {
                    // 设置搜索高亮状态
                    active.highlight_query = if self.search_active {
                        Some(self.search_query.clone())
                    } else {
                        None
                    };
                    active.highlight_line = Some(jump.line);

                    move_cursor_to_line(
                        active,
                        jump.line,
                        jump.column,
                        jump.match_len,
                        preview_sync_mode(active.editing),
                    );
                    return if active.editing {
                        sync_preview_task(active)
                    } else {
                        focus_readonly_editor()
                    };
                }

                self.pending_note_jump = Some(jump);
                let save_task = self.save_if_dirty();
                let db = self.db.clone();
                let note_id = result.note_id;
                let load_task = Task::perform(
                    async move {
                        db.execute(move |conn| crate::db::note::load_note(conn, &note_id))
                            .await
                    },
                    |result| match result {
                        Ok(note) => Message::NoteLoaded(note),
                        Err(e) => Message::DbError(e.to_string()),
                    },
                );

                Task::batch([save_task, load_task])
            }

            Message::NoteLoaded(note) => self.open_note(note, false),

            Message::NoteLoadedEditing(note) => self.open_note(note, true),

            Message::ImagesLoaded(attachments) => {
                if let Some(active) = &mut self.active_note {
                    for img in attachments {
                        let handle = iced::widget::image::Handle::from_bytes(img.data);
                        active.images.insert(img.id, handle);
                    }
                    return sync_preview_task(active);
                }
                Task::none()
            }

            Message::ToggleEditMode => {
                if let Some(active) = &mut self.active_note {
                    active.editing = !active.editing;
                    if active.editing {
                        // 进入编辑模式时清空搜索高亮
                        active.highlight_query = None;
                        active.highlight_line = None;
                        sync_preview_to_cursor(active);
                        return sync_preview_task(active);
                    }
                    active.preview_target = None;
                }
                Task::none()
            }

            Message::EditNote(id) => {
                self.context_menu = None;
                self.pending_note_jump = None;
                self.selected_id = Some(id.clone());
                let save_task = self.save_if_dirty();
                let db = self.db.clone();
                let load_task = Task::perform(
                    async move {
                        db.execute(move |conn| crate::db::note::load_note(conn, &id))
                            .await
                    },
                    |result| match result {
                        Ok(note) => Message::NoteLoadedEditing(note),
                        Err(e) => Message::DbError(e.to_string()),
                    },
                );
                Task::batch([save_task, load_task])
            }

            Message::EditorAction(action) => {
                if let Some(active) = &mut self.active_note {
                    let is_edit = action.is_edit();

                    // 查看模式：只允许光标移动和选中，禁止编辑
                    if !active.editing && is_edit {
                        return Task::none();
                    }

                    if is_edit {
                        // 防抖 Undo 快照
                        let now = Instant::now();
                        if now.duration_since(active.last_undo_push).as_millis() >= 500 {
                            let current = active.note.content.clone();
                            active.undo_stack.push(current);
                            if active.undo_stack.len() > 100 {
                                active.undo_stack.remove(0);
                            }
                            active.last_undo_push = now;
                        }
                        active.redo_stack.clear();
                    }

                    active.content.perform(action);

                    if is_edit {
                        active.dirty = true;
                        active.last_edit = Instant::now();
                        active.note.content = active.content.text();
                        rebuild_content_derived(active);
                    }

                    sync_preview_to_cursor(active);
                    return sync_preview_task(active);
                }
                Task::none()
            }

            Message::PreviewScrolled(viewport) => {
                if let Some(active) = &mut self.active_note {
                    active.preview_content_height = viewport.content_bounds().height;
                    active.preview_viewport_height = viewport.bounds().height;

                    if let Some(target) = active.preview_target {
                        let Some(offset) = preview_offset_for_line(
                            target.line,
                            active.content.line_count(),
                            active.preview_content_height,
                            active.preview_viewport_height,
                        ) else {
                            return Task::none();
                        };

                        let current_y = viewport.absolute_offset().y;
                        if (current_y - offset.y).abs() > PREVIEW_SYNC_TOLERANCE {
                            return iced::widget::operation::scroll_to(
                                preview_scrollable_id(),
                                offset,
                            );
                        }

                        if matches!(target.mode, PreviewSyncMode::Once) {
                            active.preview_target = None;
                        }
                    }
                }
                Task::none()
            }

            #[allow(dead_code)]
            Message::MarkdownParsed(items) => {
                if let Some(active) = &mut self.active_note {
                    active.markdown_items = items;
                }
                Task::none()
            }

            Message::Undo => {
                if let Some(active) = &mut self.active_note
                    && let Some(prev) = active.undo_stack.pop()
                {
                    active.redo_stack.push(active.note.content.clone());
                    active.note.content = prev;
                    active.content = text_editor::Content::with_text(&active.note.content);
                    rebuild_content_derived(active);
                    active.dirty = true;
                    active.last_edit = Instant::now();
                    sync_preview_to_cursor(active);
                    return sync_preview_task(active);
                }
                Task::none()
            }

            Message::Redo => {
                if let Some(active) = &mut self.active_note
                    && let Some(next) = active.redo_stack.pop()
                {
                    active.undo_stack.push(active.note.content.clone());
                    active.note.content = next;
                    active.content = text_editor::Content::with_text(&active.note.content);
                    rebuild_content_derived(active);
                    active.dirty = true;
                    active.last_edit = Instant::now();
                    sync_preview_to_cursor(active);
                    return sync_preview_task(active);
                }
                Task::none()
            }

            Message::SaveTick => {
                if let Some(active) = &self.active_note
                    && active.dirty
                    && active.last_edit.elapsed().as_millis() >= 500
                {
                    return self.save_current_note();
                }
                Task::none()
            }

            Message::SaveNote => self.save_current_note(),

            Message::NoteSaved => {
                if let Some(active) = &mut self.active_note {
                    active.dirty = false;
                }
                Task::none()
            }

            // 内联创建
            Message::StartCreateFolder => {
                self.pending_create = Some(PendingCreate {
                    parent_id: None,
                    is_folder: true,
                    input: String::new(),
                });
                iced::widget::operation::focus(pending_input_id())
            }

            Message::StartCreateNote => {
                let folder_id = self.find_selected_folder_id();
                if let Some(fid) = folder_id {
                    self.pending_create = Some(PendingCreate {
                        parent_id: Some(fid),
                        is_folder: false,
                        input: String::new(),
                    });
                    iced::widget::operation::focus(pending_input_id())
                } else {
                    Task::none()
                }
            }

            Message::StartCreateSubFolder(parent_id) => {
                self.context_menu = None;
                // 确保父文件夹展开
                tree_ops::ensure_expanded(&mut self.tree, &parent_id);
                self.pending_create = Some(PendingCreate {
                    parent_id: Some(parent_id),
                    is_folder: true,
                    input: String::new(),
                });
                iced::widget::operation::focus(pending_input_id())
            }

            Message::StartCreateNoteInFolder(folder_id) => {
                self.context_menu = None;
                tree_ops::ensure_expanded(&mut self.tree, &folder_id);
                self.pending_create = Some(PendingCreate {
                    parent_id: Some(folder_id),
                    is_folder: false,
                    input: String::new(),
                });
                iced::widget::operation::focus(pending_input_id())
            }

            Message::PendingCreateInputChanged(val) => {
                if let Some(state) = &mut self.pending_create {
                    state.input = val;
                }
                Task::none()
            }

            Message::ConfirmCreate => {
                if let Some(state) = self.pending_create.take() {
                    let name = if state.input.trim().is_empty() {
                        if state.is_folder {
                            "新建文件夹"
                        } else {
                            "新建笔记"
                        }
                    } else {
                        state.input.trim()
                    }
                    .to_string();

                    if state.is_folder {
                        let db = self.db.clone();
                        let parent_id = state.parent_id.clone();
                        Task::perform(
                            async move {
                                db.execute(move |conn| {
                                    crate::db::folder::create_folder(
                                        conn,
                                        parent_id.as_deref(),
                                        &name,
                                    )
                                })
                                .await
                            },
                            move |result| match result {
                                Ok(folder) => {
                                    let node = TreeNode::Folder {
                                        folder,
                                        expanded: false,
                                        children: Vec::new(),
                                        loaded: true,
                                    };
                                    if let Some(pid) = state.parent_id {
                                        Message::SubFolderCreated(pid, node)
                                    } else {
                                        Message::FolderCreated(node)
                                    }
                                }
                                Err(e) => Message::DbError(e.to_string()),
                            },
                        )
                    } else {
                        let folder_id = state.parent_id.unwrap_or_default();
                        self.create_note_in_folder_with_name(folder_id, name)
                    }
                } else {
                    Task::none()
                }
            }

            Message::CancelCreate => {
                self.pending_create = None;
                Task::none()
            }

            Message::FolderCreated(node) => {
                self.tree.push(node);
                Task::none()
            }

            Message::SubFolderCreated(parent_id, node) => {
                tree_ops::add_node(&mut self.tree, &parent_id, node);
                Task::none()
            }

            Message::NoteCreated(node) => {
                let folder_id = match &node {
                    TreeNode::Note { meta } => meta.folder_id.clone(),
                    _ => return Task::none(),
                };
                tree_ops::add_node(&mut self.tree, &folder_id, node);
                Task::none()
            }

            Message::MarkdownLinkClicked(_url) => Task::none(),

            Message::InsertImage => crate::ui::image_picker::pick_image(),

            Message::ImagePicked(Some((filename, data))) => {
                if let Some(active) = &self.active_note {
                    // 自动进入编辑模式（如果当前在查看模式）
                    let need_enter_edit = !active.editing;
                    let db = self.db.clone();
                    let note_id = active.note.id.clone();
                    let data_for_handle = data.clone();
                    let store_task = Task::perform(
                        async move {
                            db.execute(move |conn| {
                                crate::db::image::store_image(conn, &note_id, &filename, &data)
                            })
                            .await
                        },
                        move |result| match result {
                            Ok(img) => Message::ImageStored(img.id, data_for_handle),
                            Err(e) => Message::DbError(e.to_string()),
                        },
                    );
                    if need_enter_edit && let Some(a) = &mut self.active_note {
                        a.editing = true;
                    }
                    store_task
                } else {
                    Task::none()
                }
            }

            Message::ImagePicked(None) => Task::none(),

            Message::ImageStored(image_id, data) => {
                if let Some(active) = &mut self.active_note {
                    // 插入图片 Handle
                    let handle = iced::widget::image::Handle::from_bytes(data);
                    active.images.insert(image_id.clone(), handle);
                    // 在光标位置插入 markdown 图片标签
                    let marker = format!("\n![image](attachment://{image_id})\n");
                    // 推入 undo 快照
                    active.undo_stack.push(active.note.content.clone());
                    if active.undo_stack.len() > 100 {
                        active.undo_stack.remove(0);
                    }
                    active.redo_stack.clear();
                    active.last_undo_push = Instant::now();
                    active
                        .content
                        .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                            Arc::new(marker),
                        )));
                    active.note.content = active.content.text();
                    rebuild_content_derived(active);
                    active.dirty = true;
                    active.last_edit = Instant::now();
                    sync_preview_to_cursor(active);
                }
                // focus 编辑器以便后续直接键入
                let sync_task = self
                    .active_note
                    .as_ref()
                    .map(sync_preview_task)
                    .unwrap_or_else(Task::none);
                Task::batch([
                    sync_task,
                    iced::widget::operation::focus(crate::ui::editor::editor_id()),
                ])
            }

            // 上下文菜单
            Message::ShowContextMenu(target) => {
                // 打开时快照当前鼠标位置，避免后续鼠标移动让菜单漂移
                self.context_menu_position = Some(self.cursor_position);
                self.context_menu = Some(target);
                Task::none()
            }

            Message::HideContextMenu => {
                self.context_menu = None;
                self.context_menu_position = None;
                Task::none()
            }

            Message::CursorMoved(point) => {
                self.cursor_position = point;
                Task::none()
            }

            // 重命名
            Message::StartRename(id, is_folder, current_name) => {
                self.context_menu = None;
                self.rename_state = Some(super::RenameState {
                    node_id: id,
                    is_folder,
                    input: current_name,
                });
                Task::none()
            }

            Message::RenameInputChanged(val) => {
                if let Some(state) = &mut self.rename_state {
                    state.input = val;
                }
                Task::none()
            }

            Message::ConfirmRename => {
                if let Some(state) = self.rename_state.take() {
                    let db = self.db.clone();
                    let id = state.node_id.clone();
                    let name = state.input.clone();
                    let is_folder = state.is_folder;
                    Task::perform(
                        async move {
                            db.execute(move |conn| {
                                if is_folder {
                                    crate::db::folder::rename_folder(conn, &id, &name)
                                } else {
                                    crate::db::note::rename_note(conn, &id, &name)
                                }
                            })
                            .await
                        },
                        move |result| match result {
                            Ok(()) => Message::RenameCompleted(state.node_id, state.input),
                            Err(e) => Message::DbError(e.to_string()),
                        },
                    )
                } else {
                    Task::none()
                }
            }

            Message::CancelRename => {
                self.rename_state = None;
                Task::none()
            }

            Message::RenameCompleted(id, new_name) => {
                tree_ops::rename_in_tree(&mut self.tree, &id, &new_name);
                Task::none()
            }

            // 删除
            Message::DeleteNode(id, is_folder) => {
                self.context_menu = None;
                let db = self.db.clone();
                let id_clone = id.clone();
                Task::perform(
                    async move {
                        db.execute(move |conn| {
                            if is_folder {
                                crate::db::folder::delete_folder(conn, &id_clone)
                            } else {
                                crate::db::note::delete_note(conn, &id_clone)
                            }
                        })
                        .await
                    },
                    move |result| match result {
                        Ok(()) => Message::NodeDeleted(id, is_folder),
                        Err(e) => Message::DbError(e.to_string()),
                    },
                )
            }

            Message::NodeDeleted(id, _is_folder) => {
                tree_ops::remove_from_tree(&mut self.tree, &id);
                if self.selected_id.as_deref() == Some(&id) {
                    self.selected_id = None;
                    self.active_note = None;
                }
                Task::none()
            }

            // 搜索
            Message::SearchQueryChanged(query) => {
                self.search_query = query.clone();
                if query.trim().is_empty() {
                    self.search_active = false;
                    self.search_results.clear();
                    return Task::none();
                }
                self.search_active = true;
                let db = self.db.clone();
                let submitted_query = query.clone();
                let task_query = submitted_query.clone();
                Task::perform(
                    async move {
                        db.execute(move |conn| crate::db::note::search_notes(conn, &task_query))
                            .await
                    },
                    move |result| match result {
                        Ok(results) => Message::SearchPerformed(submitted_query, results),
                        Err(e) => Message::DbError(e.to_string()),
                    },
                )
            }

            Message::SearchPerformed(query, results) => {
                if self.search_query == query {
                    self.search_results = results;
                }
                Task::none()
            }

            #[allow(dead_code)]
            Message::ClearSearch => {
                self.search_query.clear();
                self.search_results.clear();
                self.search_active = false;
                self.pending_note_jump = None;
                Task::none()
            }

            // 导出
            Message::ExportNote => {
                if let Some(active) = &self.active_note {
                    let title = active.note.title.clone();
                    let content = active.note.content.clone();
                    Task::perform(
                        async move {
                            let handle = rfd::AsyncFileDialog::new()
                                .set_file_name(format!("{title}.md"))
                                .add_filter("Markdown", &["md"])
                                .set_title("导出笔记")
                                .save_file()
                                .await;
                            match handle {
                                Some(file) => file
                                    .write(content.as_bytes())
                                    .await
                                    .map_err(|e| e.to_string()),
                                None => Ok(()),
                            }
                        },
                        Message::ExportCompleted,
                    )
                } else {
                    Task::none()
                }
            }

            Message::ExportCompleted(result) => {
                if let Err(e) = result {
                    self.error = Some(format!("导出失败: {e}"));
                }
                Task::none()
            }

            Message::DbError(err) => {
                self.error = Some(err);
                Task::none()
            }

            Message::DismissError => {
                self.error = None;
                Task::none()
            }

            Message::ToggleTheme => {
                self.dark_theme = !self.dark_theme;
                Task::none()
            }

            Message::ChangeFontSize(delta) => {
                let new_size = (self.editor_font_size as i32 + delta as i32).clamp(10, 28);
                self.editor_font_size = new_size as u16;
                Task::none()
            }

            Message::InsertMdShortcut(kind) => {
                if let Some(active) = &mut self.active_note {
                    if !active.editing {
                        active.editing = true;
                    }
                    let snippet = crate::ui::md_shortcut::snippet(kind).to_string();
                    // 推入 undo 快照
                    active.undo_stack.push(active.note.content.clone());
                    if active.undo_stack.len() > 100 {
                        active.undo_stack.remove(0);
                    }
                    active.redo_stack.clear();
                    active.last_undo_push = Instant::now();
                    active
                        .content
                        .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                            Arc::new(snippet),
                        )));
                    active.note.content = active.content.text();
                    rebuild_content_derived(active);
                    active.dirty = true;
                    active.last_edit = Instant::now();
                    sync_preview_to_cursor(active);
                }
                let sync_task = self
                    .active_note
                    .as_ref()
                    .map(sync_preview_task)
                    .unwrap_or_else(Task::none);
                Task::batch([
                    sync_task,
                    iced::widget::operation::focus(crate::ui::editor::editor_id()),
                ])
            }

            Message::KeyPressed(key, modifiers) => {
                // Esc：关闭弹出层（菜单 / 重命名 / 待创建 / 错误）
                if matches!(
                    key,
                    iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)
                ) {
                    if self.context_menu.is_some() {
                        self.context_menu = None;
                        return Task::none();
                    }
                    if self.pending_create.is_some() {
                        self.pending_create = None;
                        return Task::none();
                    }
                    if self.rename_state.is_some() {
                        self.rename_state = None;
                        return Task::none();
                    }
                    if self.error.is_some() {
                        self.error = None;
                        return Task::none();
                    }
                    // 编辑模式下退出编辑
                    if let Some(active) = &mut self.active_note
                        && active.editing
                    {
                        active.editing = false;
                        active.preview_target = None;
                        return Task::none();
                    }
                }
                if modifiers.command() {
                    match key.as_ref() {
                        iced::keyboard::Key::Character("s") => self.save_current_note(),
                        iced::keyboard::Key::Character("n") => {
                            let folder_id = self.find_selected_folder_id();
                            if let Some(fid) = folder_id {
                                self.pending_create = Some(PendingCreate {
                                    parent_id: Some(fid),
                                    is_folder: false,
                                    input: String::new(),
                                });
                                iced::widget::operation::focus(pending_input_id())
                            } else {
                                Task::none()
                            }
                        }
                        iced::keyboard::Key::Character("f") => {
                            iced::widget::operation::focus(iced::widget::Id::new("search-input"))
                        }
                        iced::keyboard::Key::Character("e") => {
                            if modifiers.shift() {
                                self.update(Message::ExportNote)
                            } else {
                                self.update(Message::ToggleEditMode)
                            }
                        }
                        iced::keyboard::Key::Character("z") => {
                            if modifiers.shift() {
                                self.update(Message::Redo)
                            } else {
                                self.update(Message::Undo)
                            }
                        }
                        iced::keyboard::Key::Character("y") => self.update(Message::Redo),
                        iced::keyboard::Key::Character("=")
                        | iced::keyboard::Key::Character("+") => {
                            self.update(Message::ChangeFontSize(1))
                        }
                        iced::keyboard::Key::Character("-") => {
                            self.update(Message::ChangeFontSize(-1))
                        }
                        _ => Task::none(),
                    }
                } else {
                    Task::none()
                }
            }
        }
    }
}
