/// 文件夹
#[derive(Debug, Clone)]
pub struct Folder {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    #[allow(dead_code)]
    pub sort_order: i32,
    #[allow(dead_code)]
    pub created_at: String,
    #[allow(dead_code)]
    pub updated_at: String,
}

/// 树形节点
#[derive(Debug, Clone)]
pub enum TreeNode {
    Folder {
        folder: Folder,
        expanded: bool,
        children: Vec<TreeNode>,
        loaded: bool,
    },
    Note {
        meta: NoteMeta,
    },
}

/// 笔记元数据（不含 content，用于树形展示）
#[derive(Debug, Clone)]
pub struct NoteMeta {
    pub id: String,
    pub folder_id: String,
    pub title: String,
    #[allow(dead_code)]
    pub sort_order: i32,
}

impl TreeNode {
    pub fn id(&self) -> &str {
        match self {
            TreeNode::Folder { folder, .. } => &folder.id,
            TreeNode::Note { meta } => &meta.id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            TreeNode::Folder { folder, .. } => &folder.name,
            TreeNode::Note { meta } => &meta.title,
        }
    }
}
