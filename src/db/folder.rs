use rusqlite::{Connection, params};

use crate::model::folder::{Folder, NoteMeta, TreeNode};
use crate::util::id::new_id;

/// 加载根级树（所有根文件夹及其直接子笔记元数据）
pub fn load_root_tree(conn: &Connection) -> Result<Vec<TreeNode>, rusqlite::Error> {
    let mut stmt = conn.prepare_cached(
        "SELECT id, parent_id, name, sort_order, created_at, updated_at
         FROM folders WHERE parent_id IS NULL ORDER BY sort_order, name",
    )?;

    let folders: Vec<Folder> = stmt
        .query_map([], |row| {
            Ok(Folder {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                name: row.get(2)?,
                sort_order: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?
        .collect::<Result<_, _>>()?;

    let mut tree = Vec::with_capacity(folders.len());
    for folder in folders {
        let children = load_children(conn, &folder.id)?;
        tree.push(TreeNode::Folder {
            folder,
            expanded: false,
            children,
            loaded: true,
        });
    }
    Ok(tree)
}

/// 加载文件夹的子节点（子文件夹 + 笔记元数据）
pub fn load_children(conn: &Connection, folder_id: &str) -> Result<Vec<TreeNode>, rusqlite::Error> {
    let mut children = Vec::new();

    // 子文件夹
    let mut stmt = conn.prepare_cached(
        "SELECT id, parent_id, name, sort_order, created_at, updated_at
         FROM folders WHERE parent_id = ?1 ORDER BY sort_order, name",
    )?;
    let sub_folders: Vec<Folder> = stmt
        .query_map(params![folder_id], |row| {
            Ok(Folder {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                name: row.get(2)?,
                sort_order: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?
        .collect::<Result<_, _>>()?;

    for sub in sub_folders {
        children.push(TreeNode::Folder {
            folder: sub,
            expanded: false,
            children: Vec::new(),
            loaded: false,
        });
    }

    // 笔记元数据
    let mut stmt = conn.prepare_cached(
        "SELECT id, folder_id, title, sort_order
         FROM notes WHERE folder_id = ?1 ORDER BY sort_order, title",
    )?;
    let notes: Vec<NoteMeta> = stmt
        .query_map(params![folder_id], |row| {
            Ok(NoteMeta {
                id: row.get(0)?,
                folder_id: row.get(1)?,
                title: row.get(2)?,
                sort_order: row.get(3)?,
            })
        })?
        .collect::<Result<_, _>>()?;

    for note in notes {
        children.push(TreeNode::Note { meta: note });
    }

    Ok(children)
}

/// 创建文件夹
pub fn create_folder(
    conn: &Connection,
    parent_id: Option<&str>,
    name: &str,
) -> Result<Folder, rusqlite::Error> {
    let id = new_id();
    let now = chrono::Utc::now().to_rfc3339();

    // 获取当前最大 sort_order
    let max_order: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM folders WHERE parent_id IS ?1",
            params![parent_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    conn.execute(
        "INSERT INTO folders (id, parent_id, name, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, parent_id, name, max_order + 1, now, now],
    )?;

    Ok(Folder {
        id,
        parent_id: parent_id.map(String::from),
        name: name.to_string(),
        sort_order: max_order + 1,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// 重命名文件夹
pub fn rename_folder(conn: &Connection, id: &str, name: &str) -> Result<(), rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE folders SET name = ?1, updated_at = ?2 WHERE id = ?3",
        params![name, now, id],
    )?;
    Ok(())
}

/// 删除文件夹（级联删除子文件夹和笔记）
pub fn delete_folder(conn: &Connection, id: &str) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM folders WHERE id = ?1", params![id])?;
    Ok(())
}
