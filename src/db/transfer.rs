//! Markdown 目录传输。磁盘操作在数据库后台线程执行，每次导入使用一个事务。
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::model::{folder::TreeNode, image::ImageAttachment};

const MANIFEST: &str = ".notepad.json";
const ASSETS: &str = ".notepad-assets";
type TransferResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone, Default)]
pub struct TransferReport {
    pub path: PathBuf,
    pub folders: usize,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub ignored: usize,
}

#[derive(Default, Serialize, Deserialize)]
struct Manifest {
    version: u32,
    folders: BTreeMap<String, Identity>,
    notes: BTreeMap<String, Identity>,
}

#[derive(Serialize, Deserialize)]
struct Identity {
    id: String,
    name: String,
}

/// 文件名跨平台安全；原始名称由清单保留，重名由调用方分配后缀。
fn safe_name(name: &str) -> String {
    let mut value: String = name
        .chars()
        .map(|ch| {
            if ch.is_control() || "<>:\"/\\|?*".contains(ch) {
                '_'
            } else {
                ch
            }
        })
        .collect();
    value = value.trim().trim_end_matches('.').to_string();
    value.truncate(value.floor_char_boundary(160.min(value.len())));
    let stem = value.split('.').next().unwrap_or("").to_ascii_uppercase();
    if value.is_empty() || value == "." || value == ".." {
        value = "未命名".into();
    } else if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
    {
        value.insert(0, '_');
    }
    value
}

fn allocate_name(name: &str, extension: &str, used: &mut HashSet<String>) -> String {
    let base = safe_name(name);
    let mut candidate = format!("{base}{extension}");
    let mut index = 2;
    while !used.insert(candidate.to_lowercase()) {
        candidate = format!("{base} ({index}){extension}");
        index += 1;
    }
    candidate
}

fn relative_key(root: &Path, path: &Path) -> TransferResult<String> {
    Ok(path
        .strip_prefix(root)?
        .components()
        .map(|p| p.as_os_str().to_str().ok_or("路径不是有效 UTF-8"))
        .collect::<Result<Vec<_>, _>>()?
        .join("/"))
}

/// 在用户选择的目录下创建独立备份目录，不覆盖已有文件。
pub fn export_all(conn: &Connection, destination: &Path) -> Result<TransferReport, String> {
    let name = format!(
        "Notepad-{}-{}",
        chrono::Local::now().format("%Y%m%d-%H%M%S"),
        &crate::util::id::new_id()[..8]
    );
    let root = destination.join(name);
    fs::create_dir(&root).map_err(|e| format!("{}: {e}", root.display()))?;
    let result = (|| -> TransferResult<TransferReport> {
        let tx = conn.unchecked_transaction()?;
        let mut manifest = Manifest {
            version: 1,
            ..Manifest::default()
        };
        let mut report = TransferReport {
            path: root.clone(),
            ..TransferReport::default()
        };
        let roots = super::folder::load_root_tree(&tx)?;
        export_nodes(&tx, &roots, &root, &root, &mut manifest, &mut report, 0)?;
        fs::write(root.join(MANIFEST), serde_json::to_vec_pretty(&manifest)?)?;
        tx.commit()?;
        Ok(report)
    })();
    // 失败目录保留，明确提示不完整，避免静默丢失用户文件。
    result.map_err(|e| format!("{e}；未完成的导出位于 {}，请勿用作备份", root.display()))
}

fn export_nodes(
    conn: &Connection,
    nodes: &[TreeNode],
    root: &Path,
    directory: &Path,
    manifest: &mut Manifest,
    report: &mut TransferReport,
    depth: usize,
) -> TransferResult<()> {
    if depth > 128 {
        return Err("文件夹层级超过 128 层".into());
    }
    let mut used = HashSet::from([MANIFEST.to_string(), ASSETS.to_string()]);
    for node in nodes {
        match node {
            TreeNode::Folder { folder, .. } => {
                let path = directory.join(allocate_name(&folder.name, "", &mut used));
                fs::create_dir(&path)?;
                manifest.folders.insert(
                    relative_key(root, &path)?,
                    Identity {
                        id: folder.id.clone(),
                        name: folder.name.clone(),
                    },
                );
                report.folders += 1;
                let children = super::folder::load_children(conn, &folder.id)?;
                export_nodes(conn, &children, root, &path, manifest, report, depth + 1)?;
            }
            TreeNode::Note { meta } => {
                let note = super::note::load_note(conn, &meta.id)?;
                let path = directory.join(allocate_name(&note.title, ".md", &mut used));
                let images = super::image::load_images(conn, &note.id)?;
                let mut exported: HashMap<String, String> = HashMap::new();
                let content = rewrite_images(&note.content, |url| {
                    let Some(img) = images.iter().find(|img| image_id(url) == img.id) else {
                        return Ok(None);
                    };
                    if let Some(url) = exported.get(&img.id) {
                        return Ok(Some(url.clone()));
                    }
                    let assets = directory.join(ASSETS);
                    fs::create_dir_all(&assets)?;
                    let filename = format!("{}-{}", safe_name(&img.id), safe_name(&img.filename));
                    fs::write(assets.join(&filename), &img.data)?;
                    let url = format!(
                        "{ASSETS}/{}",
                        percent_encoding::utf8_percent_encode(
                            &filename,
                            percent_encoding::NON_ALPHANUMERIC
                        )
                    );
                    exported.insert(img.id.clone(), url.clone());
                    Ok(Some(url))
                })?;
                fs::write(&path, content)?;
                manifest.notes.insert(
                    relative_key(root, &path)?,
                    Identity {
                        id: note.id,
                        name: note.title,
                    },
                );
                report.created += 1;
            }
        }
    }
    Ok(())
}

fn image_id(url: &str) -> &str {
    for prefix in [
        "notepad://image/",
        "notepad:image:",
        "attachment://",
        "attachment:",
        "img-",
        "img:",
    ] {
        if let Some(id) = url.strip_prefix(prefix) {
            return id.trim_matches('/');
        }
    }
    url
}

/// 只改写 Markdown 图片节点，代码块和普通文字保持原样；兼容引用式图片。
fn rewrite_images(
    content: &str,
    mut resolve: impl FnMut(&str) -> TransferResult<Option<String>>,
) -> TransferResult<String> {
    let mut replacements = Vec::new();
    let mut pending = None;
    for (event, range) in Parser::new(content).into_offset_iter() {
        match event {
            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => {
                pending = Some((
                    range.start,
                    dest_url.into_string(),
                    title.into_string(),
                    String::new(),
                ));
            }
            Event::Text(value) | Event::Code(value) => {
                if let Some((_, _, _, alt)) = &mut pending {
                    alt.push_str(&value);
                }
            }
            Event::End(TagEnd::Image) => {
                if let Some((start, url, title, alt)) = pending.take()
                    && let Some(replacement) = resolve(&url)?
                    && replacement != url
                {
                    let alt = alt
                        .replace('\\', "\\\\")
                        .replace('[', "\\[")
                        .replace(']', "\\]");
                    let title = title.replace('\\', "\\\\").replace('"', "\\\"");
                    let suffix = if title.is_empty() {
                        String::new()
                    } else {
                        format!(" \"{title}\"")
                    };
                    let source = &content[start..range.end];
                    let inline = source.rfind("](").and_then(|offset| {
                        source[offset + 2..]
                            .find(&url)
                            .map(|index| start + offset + 2 + index)
                    });
                    if let Some(offset) = inline {
                        replacements.push((offset..offset + url.len(), replacement));
                    } else {
                        replacements.push((
                            start..range.end,
                            format!("![{alt}](<{replacement}>{suffix})"),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    let mut output = content.to_string();
    for (range, replacement) in replacements.into_iter().rev() {
        output.replace_range(range, &replacement);
    }
    Ok(output)
}

struct Importer<'a> {
    conn: &'a Connection,
    root: PathBuf,
    manifest: Manifest,
    report: TransferReport,
    used_folders: HashSet<String>,
    used_notes: HashSet<String>,
}

/// 按目录和文件名匹配：相同内容跳过，变化内容原位更新，失败回滚整次导入。
pub fn import_all(conn: &Connection, source: &Path) -> Result<TransferReport, String> {
    (|| -> TransferResult<TransferReport> {
        let root = source.canonicalize()?;
        let manifest_path = root.join(MANIFEST);
        let manifest = if manifest_path.try_exists()? {
            if fs::symlink_metadata(&manifest_path)?
                .file_type()
                .is_symlink()
            {
                return Err("清单不能是符号链接".into());
            }
            let manifest: Manifest = serde_json::from_slice(&fs::read(&manifest_path)?)?;
            if manifest.version != 1 {
                return Err("不支持的 Notepad 导出版本".into());
            }
            manifest
        } else {
            Manifest::default()
        };
        let tx = conn.unchecked_transaction()?;
        let mut importer = Importer {
            conn: &tx,
            root: root.clone(),
            manifest,
            report: TransferReport {
                path: root.clone(),
                ..TransferReport::default()
            },
            used_folders: HashSet::new(),
            used_notes: HashSet::new(),
        };
        importer.visit(&root, None, 0)?;
        let report = importer.report;
        tx.commit()?;
        Ok(report)
    })()
    .map_err(|e| format!("{e}；本次导入未写入任何更改"))
}

impl Importer<'_> {
    fn folder(
        &mut self,
        parent: Option<&str>,
        name: &str,
        identity: Option<&Identity>,
    ) -> TransferResult<String> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id FROM folders WHERE parent_id IS ?1 AND name = ?2 ORDER BY sort_order, id",
        )?;
        let candidates = stmt
            .query_map(params![parent, name], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let matched = candidates
            .iter()
            .find(|id| identity.is_some_and(|meta| &meta.id == *id))
            .or_else(|| {
                candidates
                    .iter()
                    .find(|id| !self.used_folders.contains(*id))
            });
        let id = match matched {
            Some(id) => id.clone(),
            None => {
                self.report.folders += 1;
                super::folder::create_folder(self.conn, parent, name)?.id
            }
        };
        self.used_folders.insert(id.clone());
        Ok(id)
    }

    fn visit(
        &mut self,
        directory: &Path,
        parent: Option<&str>,
        depth: usize,
    ) -> TransferResult<()> {
        if depth > 128 {
            return Err("文件夹层级超过 128 层".into());
        }
        let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        let mut root_notes_folder = None;
        for entry in entries {
            let path = entry.path();
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "文件名不是有效 UTF-8")?;
            if name == MANIFEST || name == ASSETS {
                continue;
            }
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                self.report.ignored += 1;
                continue;
            }
            let key = relative_key(&self.root, &path)?;
            if kind.is_dir() {
                // 取出元数据以免在递归修改状态时借用整个清单。
                let identity = self.manifest.folders.remove(&key);
                let title = identity.as_ref().map_or(name.as_str(), |meta| &meta.name);
                let id = self.folder(parent, title, identity.as_ref())?;
                self.visit(&path, Some(&id), depth + 1)?;
            } else if kind.is_file()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| {
                        ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown")
                    })
            {
                let folder_id = if let Some(parent) = parent {
                    parent.to_string()
                } else {
                    if root_notes_folder.is_none() {
                        // 数据模型要求笔记属于文件夹；根目录 Markdown 使用固定收纳目录。
                        root_notes_folder = Some(self.folder(None, "导入笔记", None)?);
                    }
                    root_notes_folder.clone().ok_or("无法创建导入目录")?
                };
                self.note(&path, &key, &folder_id)?;
            } else {
                self.report.ignored += 1;
            }
        }
        Ok(())
    }

    fn note(&mut self, path: &Path, key: &str, folder_id: &str) -> TransferResult<()> {
        let identity = self.manifest.notes.get(key);
        let title = identity.map(|meta| meta.name.as_str()).unwrap_or(
            path.file_stem()
                .and_then(|name| name.to_str())
                .ok_or("笔记名称不是有效 UTF-8")?,
        );
        let content = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
        let mut stmt = self.conn.prepare_cached(
            "SELECT id FROM notes WHERE folder_id = ?1 AND title = ?2 ORDER BY sort_order, id",
        )?;
        let candidates = stmt
            .query_map(params![folder_id, title], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let matched = candidates
            .iter()
            .find(|id| identity.is_some_and(|meta| &meta.id == *id))
            .or_else(|| {
                candidates
                    .iter()
                    .find(|id| identity.is_none() || !self.used_notes.contains(*id))
            });
        let is_new = matched.is_none();
        let mut note = if let Some(id) = matched {
            super::note::load_note(self.conn, id)?
        } else {
            super::note::create_note(self.conn, folder_id, title)?
        };
        self.used_notes.insert(note.id.clone());
        let old_images = super::image::load_images(self.conn, &note.id)?;
        let mut images: HashMap<String, ImageAttachment> = HashMap::new();
        let mut resolved: HashMap<String, String> = HashMap::new();
        let imported = rewrite_images(content, |url| {
            if let Some(value) = resolved.get(url) {
                return Ok(Some(value.clone()));
            }
            if let Some(img) = old_images.iter().find(|img| image_id(url) == img.id) {
                images.insert(img.id.clone(), img.clone());
                return Ok(None);
            }
            if url.contains(':') || url.starts_with("//") || url.starts_with('#') {
                return Ok(None);
            }
            let decoded = percent_encoding::percent_decode_str(url).decode_utf8()?;
            let local = path
                .parent()
                .ok_or("笔记没有父目录")?
                .join(decoded.as_ref())
                .canonicalize()
                .map_err(|e| format!("{} 的图片 {url}: {e}", path.display()))?;
            if !local.starts_with(&self.root) {
                return Err(format!("图片超出导入目录: {url}").into());
            }
            let filename = local
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or("图片文件名无效")?
                .to_string();
            let data = fs::read(local)?;
            let img = old_images
                .iter()
                .find(|img| img.data == data)
                .cloned()
                .unwrap_or_else(|| ImageAttachment {
                    id: crate::util::id::new_id(),
                    note_id: note.id.clone(),
                    filename,
                    data,
                    created_at: chrono::Utc::now().to_rfc3339(),
                });
            let replacement = format!("notepad://image/{}", img.id);
            images.insert(img.id.clone(), img);
            resolved.insert(url.into(), replacement.clone());
            Ok(Some(replacement))
        })?;
        if !is_new && imported == note.content {
            self.report.skipped += 1;
            return Ok(());
        }
        note.content = imported;
        super::note::save_note(self.conn, &note)?;
        for img in images.values() {
            self.conn.execute(
                "INSERT OR IGNORE INTO images (id, note_id, filename, data, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![img.id, note.id, img.filename, img.data, img.created_at])?;
        }
        for img in old_images {
            if !images.contains_key(&img.id) {
                super::image::delete_image(self.conn, &img.id)?;
            }
        }
        if is_new {
            self.report.created += 1;
        } else {
            self.report.updated += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
