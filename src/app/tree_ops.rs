use crate::db::DbPool;
use crate::message::Message;
use crate::model::folder::TreeNode;

/// 递归切换文件夹展开状态
pub fn toggle_in_tree(tree: &mut [TreeNode], id: &str, db: &DbPool) -> Option<iced::Task<Message>> {
    for node in tree.iter_mut() {
        if let TreeNode::Folder {
            folder,
            expanded,
            children,
            loaded,
        } = node
        {
            if folder.id == id {
                *expanded = !*expanded;
                if *expanded && !*loaded {
                    *loaded = true;
                    let db = db.clone();
                    let folder_id = folder.id.clone();
                    let id_owned = id.to_string();
                    return Some(iced::Task::perform(
                        async move {
                            db.execute(move |conn| {
                                crate::db::folder::load_children(conn, &folder_id)
                            })
                            .await
                        },
                        move |result| match result {
                            Ok(ch) => Message::FolderChildrenLoaded(id_owned, ch),
                            Err(e) => Message::DbError(e.to_string()),
                        },
                    ));
                }
                return Some(iced::Task::none());
            }
            if let Some(task) = toggle_in_tree(children, id, db) {
                return Some(task);
            }
        }
    }
    None
}

/// 设置文件夹子节点
pub fn set_children(tree: &mut [TreeNode], folder_id: &str, new_children: Vec<TreeNode>) {
    for node in tree.iter_mut() {
        if let TreeNode::Folder {
            folder, children, ..
        } = node
        {
            if folder.id == folder_id {
                *children = new_children;
                return;
            }
            set_children(children, folder_id, new_children.clone());
        }
    }
}

/// 查找给定 id 所属的文件夹 id：
/// - 若 id 本身是文件夹，返回该文件夹自身的 id
/// - 若 id 是某笔记节点，返回包含该笔记的父文件夹 id
/// - 否则递归进入子文件夹继续查找
pub fn find_folder_in_tree(tree: &[TreeNode], id: &str) -> Option<String> {
    for node in tree {
        if let TreeNode::Folder {
            folder, children, ..
        } = node
        {
            if folder.id == id {
                return Some(folder.id.clone());
            }
            // 递归进入子文件夹（优先匹配嵌套文件夹自身）
            if let Some(found) = find_folder_in_tree(children, id) {
                return Some(found);
            }
            // 再检查是否为当前文件夹直接包含的笔记
            for child in children {
                if let TreeNode::Note { meta } = child
                    && meta.id == id
                {
                    return Some(folder.id.clone());
                }
            }
        }
    }
    None
}

/// 查找节点名称
pub fn find_name_in_tree(tree: &[TreeNode], id: &str) -> Option<String> {
    for node in tree {
        if node.id() == id {
            return Some(node.name().to_string());
        }
        if let TreeNode::Folder { children, .. } = node
            && let Some(name) = find_name_in_tree(children, id)
        {
            return Some(name);
        }
    }
    None
}

/// 重命名节点
pub fn rename_in_tree(tree: &mut [TreeNode], id: &str, new_name: &str) {
    for node in tree.iter_mut() {
        match node {
            TreeNode::Folder {
                folder, children, ..
            } => {
                if folder.id == id {
                    folder.name = new_name.to_string();
                    return;
                }
                rename_in_tree(children, id, new_name);
            }
            TreeNode::Note { meta } => {
                if meta.id == id {
                    meta.title = new_name.to_string();
                    return;
                }
            }
        }
    }
}

/// 删除节点
pub fn remove_from_tree(tree: &mut Vec<TreeNode>, id: &str) {
    tree.retain(|node| node.id() != id);
    for node in tree.iter_mut() {
        if let TreeNode::Folder { children, .. } = node {
            remove_from_tree(children, id);
        }
    }
}

/// 添加节点到指定文件夹
pub fn add_node(tree: &mut [TreeNode], folder_id: &str, node: TreeNode) {
    for item in tree.iter_mut() {
        if let TreeNode::Folder {
            folder, children, ..
        } = item
        {
            if folder.id == folder_id {
                children.push(node);
                return;
            }
            add_node(children, folder_id, node.clone());
        }
    }
}

/// 确保指定文件夹处于展开状态（不触发加载）
pub fn ensure_expanded(tree: &mut [TreeNode], folder_id: &str) {
    for node in tree.iter_mut() {
        if let TreeNode::Folder {
            folder,
            expanded,
            children,
            ..
        } = node
        {
            if folder.id == folder_id {
                *expanded = true;
                return;
            }
            ensure_expanded(children, folder_id);
        }
    }
}
