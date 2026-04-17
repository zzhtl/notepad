use crate::model::folder::{Folder, NoteMeta, TreeNode};
use crate::model::image::ImageAttachment;
use crate::model::note::{Note, SearchResult};

/// 上下文菜单目标
#[derive(Debug, Clone)]
pub struct ContextMenuTarget {
    pub node_id: String,
    pub is_folder: bool,
    pub parent_folder_id: Option<String>,
}

/// Markdown 快捷插入类型
#[derive(Debug, Clone, Copy)]
pub enum MdShortcut {
    H1,
    H2,
    H3,
    Bold,
    Italic,
    Strikethrough,
    InlineCode,
    CodeBlock,
    BulletList,
    NumberedList,
    Checkbox,
    Quote,
    HorizontalRule,
    Link,
    Table,
}

/// 全局消息枚举
#[derive(Debug, Clone)]
pub enum Message {
    // 树形视图
    TreeLoaded(Vec<TreeNode>),
    ToggleFolder(String),
    FolderChildrenLoaded(String, Vec<TreeNode>),
    SelectNote(String),
    OpenSearchResult(SearchResult),

    // 文件夹操作
    FolderCreated(TreeNode),
    SubFolderCreated(String, TreeNode),

    // 笔记操作
    NoteCreated(TreeNode),
    NoteLoaded(Note),
    NoteLoadedEditing(Note),
    NoteMoved(NoteMeta),
    SaveNote,
    NoteSaved,

    // 内联创建
    StartCreateFolder,
    StartCreateNote,
    StartCreateSubFolder(String),
    StartCreateNoteInFolder(String),
    PendingCreateInputChanged(String),
    ConfirmCreate,
    CancelCreate,

    // 编辑器
    EditorAction(iced::widget::text_editor::Action),
    #[allow(dead_code)]
    MarkdownParsed(Vec<iced::widget::markdown::Item>),
    SaveTick,

    // 查看/编辑模式
    ToggleEditMode,
    EditNote(String),

    // Markdown 快捷
    InsertMdShortcut(MdShortcut),

    // Undo/Redo
    Undo,
    Redo,

    // 编辑区滚动
    EditorScrolled(iced::widget::scrollable::Viewport),

    // 同步滚动
    PreviewScrolled(iced::widget::scrollable::Viewport),

    // 链接
    MarkdownLinkClicked(iced::widget::markdown::Uri),

    // 图片
    InsertImage,
    ImagePicked(Option<(String, Vec<u8>)>),
    ImageStored(String, Vec<u8>),
    ImagesLoaded(Vec<ImageAttachment>),

    // 键盘快捷键
    KeyPressed(iced::keyboard::Key, iced::keyboard::Modifiers),

    // 上下文菜单
    ShowContextMenu(ContextMenuTarget),
    HideContextMenu,
    BackToContextMenu,
    OpenMoveNoteMenu(String, String),
    MoveFolderOptionsLoaded(String, String, Vec<Folder>),
    MoveNoteToFolder(String, String),

    // 鼠标位置（用于右键菜单定位）
    CursorMoved(iced::Point),

    // 拖拽移动笔记
    StartNoteDrag(String),
    FinishNoteDrag,
    NoteDragEnteredFolder(String),
    NoteDragLeftFolder(String),
    DropDraggedNoteOnFolder(String),

    // 重命名
    StartRename(String, bool, String),
    RenameInputChanged(String),
    ConfirmRename,
    CancelRename,
    RenameCompleted(String, String),

    // 删除
    DeleteNode(String, bool),
    NodeDeleted(String, bool),

    // 搜索
    SearchQueryChanged(String),
    SearchPerformed(String, Vec<SearchResult>),
    #[allow(dead_code)]
    ClearSearch,

    // 导出
    ExportNote,
    ExportCompleted(Result<(), String>),

    // 主题
    ToggleTheme,

    // 字体大小
    ChangeFontSize(i8),

    // 错误
    DbError(String),
    DismissError,
}
