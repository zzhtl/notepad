mod tree_ops;
pub mod update;

use std::collections::HashMap;
use std::time::Instant;

use iced::widget::{
    Space, button, column, container, markdown, mouse_area, row, rule, scrollable, stack, text,
    text_editor,
};
use iced::{Border, Color, Element, Fill, Length, Padding, Point, Subscription, Task, Theme};

use crate::db::DbPool;
use crate::message::{ContextMenuTarget, Message};
use crate::model::folder::{Folder, TreeNode};
use crate::model::note::Note;
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewSyncMode {
    Once,
    FollowCursor,
}

#[derive(Debug, Clone, Copy)]
pub struct PreviewSyncTarget {
    pub line: usize,
    pub mode: PreviewSyncMode,
}

#[derive(Debug, Clone)]
pub struct PendingNoteJump {
    pub note_id: String,
    pub line: usize,
    pub column: usize,
    pub match_len: usize,
}

/// 当前活跃笔记状态
pub struct ActiveNote {
    pub note: Note,
    pub content: text_editor::Content,
    pub markdown_items: Vec<markdown::Item>,
    pub images: HashMap<String, iced::widget::image::Handle>,
    pub dirty: bool,
    pub last_edit: Instant,
    pub preview_content_height: f32,
    pub preview_viewport_height: f32,
    pub preview_target: Option<PreviewSyncTarget>,
    pub editing: bool,
    pub undo_stack: Vec<String>,
    pub redo_stack: Vec<String>,
    pub last_undo_push: Instant,
    /// 搜索高亮查询词（查看模式下高亮文档中的匹配内容）
    pub highlight_query: Option<String>,
    /// 搜索导航目标行（查看模式滚动定位）
    pub highlight_line: Option<usize>,
}

/// 重命名状态
pub(crate) struct RenameState {
    pub node_id: String,
    pub is_folder: bool,
    pub input: String,
}

/// 待创建状态（内联命名）
pub struct PendingCreate {
    pub parent_id: Option<String>,
    pub is_folder: bool,
    pub input: String,
}

#[derive(Debug, Clone)]
pub(crate) struct MoveFolderOption {
    pub id: String,
    pub name: String,
    pub depth: usize,
}

pub(crate) struct MoveNoteState {
    pub note_id: String,
    pub current_folder_id: String,
    pub folders: Vec<MoveFolderOption>,
    pub loading: bool,
}

/// 应用主结构体
pub struct App {
    pub(crate) db: DbPool,
    pub(crate) tree: Vec<TreeNode>,
    pub(crate) selected_id: Option<String>,
    pub(crate) active_note: Option<ActiveNote>,
    pub(crate) context_menu: Option<ContextMenuTarget>,
    pub(crate) rename_state: Option<RenameState>,
    pub(crate) error: Option<String>,
    pub(crate) search_query: String,
    pub(crate) search_results: Vec<crate::model::note::SearchResult>,
    pub(crate) search_active: bool,
    pub(crate) pending_note_jump: Option<PendingNoteJump>,
    pub(crate) pending_create: Option<PendingCreate>,
    pub(crate) last_note_click: Option<(String, Instant)>,
    pub(crate) dark_theme: bool,
    pub(crate) editor_font_size: u16,
    pub(crate) dragging_note_id: Option<String>,
    pub(crate) drag_hover_folder_id: Option<String>,
    pub(crate) move_note_state: Option<MoveNoteState>,
    /// 实时跟踪的鼠标位置（窗口坐标），用于右键菜单定位
    pub(crate) cursor_position: Point,
    /// 右键菜单打开时锁定的位置快照（None 表示未打开）
    pub(crate) context_menu_position: Option<Point>,
}

fn build_move_folder_options(folders: Vec<Folder>) -> Vec<MoveFolderOption> {
    fn visit(
        by_parent: &HashMap<Option<String>, Vec<Folder>>,
        parent_id: Option<String>,
        depth: usize,
        output: &mut Vec<MoveFolderOption>,
    ) {
        if let Some(children) = by_parent.get(&parent_id) {
            for folder in children {
                output.push(MoveFolderOption {
                    id: folder.id.clone(),
                    name: folder.name.clone(),
                    depth,
                });

                visit(by_parent, Some(folder.id.clone()), depth + 1, output);
            }
        }
    }

    let mut by_parent: HashMap<Option<String>, Vec<Folder>> = HashMap::new();

    for folder in folders {
        by_parent
            .entry(folder.parent_id.clone())
            .or_default()
            .push(folder);
    }

    for siblings in by_parent.values_mut() {
        siblings.sort_by(|left, right| {
            left.sort_order
                .cmp(&right.sort_order)
                .then_with(|| left.name.cmp(&right.name))
        });
    }

    let mut output = Vec::new();
    visit(&by_parent, None, 0, &mut output);
    output
}

impl App {
    pub fn new(db: DbPool) -> (Self, Task<Message>) {
        let app = Self {
            db: db.clone(),
            tree: Vec::new(),
            selected_id: None,
            active_note: None,
            context_menu: None,
            rename_state: None,
            error: None,
            search_query: String::new(),
            search_results: Vec::new(),
            search_active: false,
            pending_note_jump: None,
            pending_create: None,
            last_note_click: None,
            dark_theme: true,
            editor_font_size: 14,
            dragging_note_id: None,
            drag_hover_folder_id: None,
            move_note_state: None,
            cursor_position: Point::ORIGIN,
            context_menu_position: None,
        };

        let task = Task::perform(
            async move { db.execute(crate::db::folder::load_root_tree).await },
            |result| match result {
                Ok(tree) => Message::TreeLoaded(tree),
                Err(e) => Message::DbError(e.to_string()),
            },
        );

        (app, task)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let sidebar = ui::sidebar::view(
            &self.tree,
            self.selected_id.as_deref(),
            self.rename_state
                .as_ref()
                .map(|s| (s.node_id.as_str(), s.input.as_str())),
            &self.search_query,
            self.search_active,
            &self.search_results,
            self.pending_create.as_ref(),
            self.dragging_note_id.as_deref(),
            self.drag_hover_folder_id.as_deref(),
        );

        let editor_area: Element<'_, Message> = if let Some(active) = &self.active_note {
            let toolbar = ui::toolbar::view(
                &active.note.title,
                active.dirty,
                active.editing,
                self.dark_theme,
                self.editor_font_size,
            );
            let editor = ui::editor::view(active, &self.theme(), self.editor_font_size);

            {
                let status_bar = ui::status_bar::view(active);
                column![
                    toolbar,
                    rule::horizontal(1),
                    editor,
                    rule::horizontal(1),
                    status_bar,
                ]
                .height(Fill)
                .into()
            }
        } else {
            ui::welcome::view(self.dark_theme).into()
        };

        let mut content = column![].width(Fill).height(Fill);

        if let Some(err) = &self.error {
            content = content.push(ui::error_banner::view(err));
        }

        content = content.push(row![sidebar, rule::vertical(1), editor_area].height(Fill));

        // 用 mouse_area 包裹主视图以持续跟踪鼠标位置，供右键菜单弹出定位使用。
        // on_move 只观察事件而不捕获，不会影响子组件的交互。
        let main_view: Element<'_, Message> =
            mouse_area(container(content).width(Fill).height(Fill))
                .on_move(Message::CursorMoved)
                .on_release(Message::FinishNoteDrag)
                .into();

        // 上下文菜单覆盖层
        if let Some(ctx) = &self.context_menu {
            let node_name = self.find_node_name(&ctx.node_id).unwrap_or_default();
            let menu_style = |theme: &Theme| -> container::Style {
                let palette = theme.extended_palette();
                container::Style {
                    background: Some(palette.background.weak.color.into()),
                    border: Border {
                        radius: 8.0.into(),
                        width: 1.0,
                        color: Color {
                            a: 0.35,
                            ..palette.primary.base.color
                        },
                    },
                    ..container::Style::default()
                }
            };
            let menu: Element<'_, Message> = if let Some(move_state) = &self.move_note_state {
                let mut folder_items: Vec<Element<'_, Message>> = vec![
                    container(text("移动到文件夹").size(13))
                        .padding([4, 12])
                        .into(),
                    button(text("返回").size(13))
                        .on_press(Message::BackToContextMenu)
                        .padding([4, 12])
                        .width(Length::Fill)
                        .style(button::text)
                        .into(),
                ];

                if move_state.loading {
                    folder_items.push(
                        container(text("正在加载文件夹...").size(13))
                            .padding([6, 12])
                            .width(Length::Fill)
                            .into(),
                    );
                } else if move_state.folders.is_empty() {
                    folder_items.push(
                        container(text("没有可用文件夹").size(13))
                            .padding([6, 12])
                            .width(Length::Fill)
                            .into(),
                    );
                } else {
                    for folder in &move_state.folders {
                        let indent = 12.0 + folder.depth as f32 * 16.0;
                        let label = row![
                            Space::new().width(Length::Fixed(indent)),
                            text(&folder.name).size(13),
                        ];

                        if folder.id == move_state.current_folder_id {
                            folder_items.push(
                                container(label.push(text("（当前）").size(12).style(
                                    |theme: &Theme| {
                                        let palette = theme.extended_palette();
                                        text::Style {
                                            color: Some(palette.background.weak.text),
                                        }
                                    },
                                )))
                                .padding([4, 12])
                                .width(Length::Fill)
                                .into(),
                            );
                        } else {
                            folder_items.push(
                                button(label)
                                    .on_press(Message::MoveNoteToFolder(
                                        move_state.note_id.clone(),
                                        folder.id.clone(),
                                    ))
                                    .padding([4, 12])
                                    .width(Length::Fill)
                                    .style(button::text)
                                    .into(),
                            );
                        }
                    }
                }

                let content: Element<'_, Message> = if folder_items.len() > 10 {
                    scrollable(column(folder_items).spacing(1))
                        .height(Length::Fixed(240.0))
                        .into()
                } else {
                    column(folder_items).spacing(1).into()
                };

                container(content)
                    .padding(6)
                    .width(Length::Fixed(260.0))
                    .style(menu_style)
                    .into()
            } else {
                let mut menu_items: Vec<Element<'_, Message>> = Vec::new();

                if ctx.is_folder {
                    menu_items.push(
                        button(text("新建子文件夹").size(13))
                            .on_press(Message::StartCreateSubFolder(ctx.node_id.clone()))
                            .padding([4, 12])
                            .width(Length::Fill)
                            .style(button::text)
                            .into(),
                    );
                    menu_items.push(
                        button(text("新建笔记").size(13))
                            .on_press(Message::StartCreateNoteInFolder(ctx.node_id.clone()))
                            .padding([4, 12])
                            .width(Length::Fill)
                            .style(button::text)
                            .into(),
                    );
                } else {
                    menu_items.push(
                        button(text("编辑").size(13))
                            .on_press(Message::EditNote(ctx.node_id.clone()))
                            .padding([4, 12])
                            .width(Length::Fill)
                            .style(button::text)
                            .into(),
                    );

                    if let Some(parent_folder_id) = &ctx.parent_folder_id {
                        menu_items.push(
                            button(text("移动到文件夹").size(13))
                                .on_press(Message::OpenMoveNoteMenu(
                                    ctx.node_id.clone(),
                                    parent_folder_id.clone(),
                                ))
                                .padding([4, 12])
                                .width(Length::Fill)
                                .style(button::text)
                                .into(),
                        );
                    }
                }

                menu_items.push(
                    button(text("重命名").size(13))
                        .on_press(Message::StartRename(
                            ctx.node_id.clone(),
                            ctx.is_folder,
                            node_name,
                        ))
                        .padding([4, 12])
                        .width(Length::Fill)
                        .style(button::text)
                        .into(),
                );
                menu_items.push(
                    button(text("删除").size(13))
                        .on_press(Message::DeleteNode(ctx.node_id.clone(), ctx.is_folder))
                        .padding([4, 12])
                        .width(Length::Fill)
                        .style(button::danger)
                        .into(),
                );

                container(column(menu_items).spacing(1).width(Length::Fixed(180.0)))
                    .padding(6)
                    .style(menu_style)
                    .into()
            };

            let dismiss = mouse_area(Space::new().width(Fill).height(Fill))
                .on_press(Message::HideContextMenu);

            // 菜单位置：使用打开时的鼠标位置快照，避免用户移动鼠标时菜单跟随漂移。
            // 通过在外层 container 上设置 top/left padding 来把菜单定位到鼠标点击处。
            let pos = self.context_menu_position.unwrap_or(self.cursor_position);
            let menu_pad = Padding {
                top: pos.y.max(0.0),
                left: pos.x.max(0.0),
                right: 0.0,
                bottom: 0.0,
            };

            stack![main_view, dismiss, container(menu).padding(menu_pad)]
                .width(Fill)
                .height(Fill)
                .into()
        } else {
            main_view
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let auto_save = if self.active_note.as_ref().is_some_and(|n| n.dirty) {
            iced::time::every(std::time::Duration::from_millis(500)).map(|_| Message::SaveTick)
        } else {
            Subscription::none()
        };

        let keyboard = iced::keyboard::listen().map(|event| match event {
            iced::keyboard::Event::KeyPressed { key, modifiers, .. } => {
                Message::KeyPressed(key, modifiers)
            }
            _ => Message::KeyPressed(
                iced::keyboard::Key::Unidentified,
                iced::keyboard::Modifiers::default(),
            ),
        });

        Subscription::batch([auto_save, keyboard])
    }

    pub fn title(&self) -> String {
        if let Some(active) = &self.active_note {
            let dirty_mark = if active.dirty { " *" } else { "" };
            format!("{}{} - Notepad", active.note.title, dirty_mark)
        } else {
            "Notepad".to_string()
        }
    }

    pub fn theme(&self) -> Theme {
        if self.dark_theme {
            Theme::TokyoNight
        } else {
            Theme::Light
        }
    }

    // --- 内部辅助 ---

    pub(crate) fn create_note_in_folder_with_name(
        &self,
        folder_id: String,
        name: String,
    ) -> Task<Message> {
        let db = self.db.clone();
        Task::perform(
            async move {
                db.execute(move |conn| crate::db::note::create_note(conn, &folder_id, &name))
                    .await
            },
            |result| match result {
                Ok(note) => Message::NoteCreated(TreeNode::Note {
                    meta: crate::model::folder::NoteMeta {
                        id: note.id,
                        folder_id: note.folder_id,
                        title: note.title,
                        sort_order: note.sort_order,
                    },
                }),
                Err(e) => Message::DbError(e.to_string()),
            },
        )
    }

    pub(crate) fn move_note_to_folder(&self, note_id: String, folder_id: String) -> Task<Message> {
        let db = self.db.clone();
        Task::perform(
            async move {
                db.execute(move |conn| crate::db::note::move_note(conn, &note_id, &folder_id))
                    .await
            },
            |result| match result {
                Ok(meta) => Message::NoteMoved(meta),
                Err(e) => Message::DbError(e.to_string()),
            },
        )
    }

    pub(crate) fn find_selected_folder_id(&self) -> Option<String> {
        if let Some(id) = &self.selected_id {
            tree_ops::find_folder_in_tree(&self.tree, id)
        } else {
            self.tree.first().map(|n| n.id().to_string())
        }
    }

    pub(crate) fn find_node_name(&self, id: &str) -> Option<String> {
        tree_ops::find_name_in_tree(&self.tree, id)
    }

    pub(crate) fn save_if_dirty(&self) -> Task<Message> {
        if self.active_note.as_ref().is_some_and(|n| n.dirty) {
            self.save_current_note()
        } else {
            Task::none()
        }
    }

    pub(crate) fn save_current_note(&self) -> Task<Message> {
        if let Some(active) = &self.active_note {
            let db = self.db.clone();
            let note = active.note.clone();
            Task::perform(
                async move {
                    db.execute(move |conn| crate::db::note::save_note(conn, &note))
                        .await
                },
                |result| match result {
                    Ok(()) => Message::NoteSaved,
                    Err(e) => Message::DbError(e.to_string()),
                },
            )
        } else {
            Task::none()
        }
    }
}
