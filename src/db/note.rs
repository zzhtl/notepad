use rusqlite::{Connection, params};

use crate::model::folder::NoteMeta;
use crate::model::note::{Note, SearchResult};
use crate::util::id::new_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MatchLocation {
    byte_index: usize,
    line: usize,
    column: usize,
}

/// 加载笔记完整内容
pub fn load_note(conn: &Connection, id: &str) -> Result<Note, rusqlite::Error> {
    conn.query_row(
        "SELECT id, folder_id, title, content, sort_order, created_at, updated_at
         FROM notes WHERE id = ?1",
        params![id],
        |row| {
            Ok(Note {
                id: row.get(0)?,
                folder_id: row.get(1)?,
                title: row.get(2)?,
                content: row.get(3)?,
                sort_order: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        },
    )
}

/// 保存笔记内容
pub fn save_note(conn: &Connection, note: &Note) -> Result<(), rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE notes SET title = ?1, content = ?2, updated_at = ?3 WHERE id = ?4",
        params![note.title, note.content, now, note.id],
    )?;
    Ok(())
}

/// 创建笔记
pub fn create_note(
    conn: &Connection,
    folder_id: &str,
    title: &str,
) -> Result<Note, rusqlite::Error> {
    let id = new_id();
    let now = chrono::Utc::now().to_rfc3339();

    let max_order: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM notes WHERE folder_id = ?1",
            params![folder_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    conn.execute(
        "INSERT INTO notes (id, folder_id, title, content, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, '', ?4, ?5, ?6)",
        params![id, folder_id, title, max_order + 1, now, now],
    )?;

    Ok(Note {
        id,
        folder_id: folder_id.to_string(),
        title: title.to_string(),
        content: String::new(),
        sort_order: max_order + 1,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// 重命名笔记
pub fn rename_note(conn: &Connection, id: &str, title: &str) -> Result<(), rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE notes SET title = ?1, updated_at = ?2 WHERE id = ?3",
        params![title, now, id],
    )?;
    Ok(())
}

/// 删除笔记
pub fn delete_note(conn: &Connection, id: &str) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM notes WHERE id = ?1", params![id])?;
    Ok(())
}

/// 移动笔记到目标文件夹，并将排序放到目标文件夹末尾
pub fn move_note(
    conn: &Connection,
    note_id: &str,
    folder_id: &str,
) -> Result<NoteMeta, rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    let max_order: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM notes WHERE folder_id = ?1",
            params![folder_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    conn.execute(
        "UPDATE notes
         SET folder_id = ?1, sort_order = ?2, updated_at = ?3
         WHERE id = ?4",
        params![folder_id, max_order + 1, now, note_id],
    )?;

    conn.query_row(
        "SELECT id, folder_id, title, sort_order FROM notes WHERE id = ?1",
        params![note_id],
        |row| {
            Ok(NoteMeta {
                id: row.get(0)?,
                folder_id: row.get(1)?,
                title: row.get(2)?,
                sort_order: row.get(3)?,
            })
        },
    )
}

fn find_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }

    let needle = needle.to_lowercase();

    for (start, _) in haystack.char_indices() {
        let mut lowered = String::new();

        for ch in haystack[start..].chars() {
            lowered.extend(ch.to_lowercase());

            if lowered == needle {
                return Some(start);
            }

            if lowered.len() >= needle.len() || !needle.starts_with(&lowered) {
                break;
            }
        }
    }

    None
}

#[cfg(test)]
fn locate_match(content: &str, query: &str) -> Option<MatchLocation> {
    let byte_index = find_case_insensitive(content, query)?;
    let mut line = 0;
    let mut column = 0;

    for ch in content[..byte_index].chars() {
        if ch == '\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
    }

    Some(MatchLocation {
        byte_index,
        line,
        column,
    })
}

/// 查找内容中所有匹配位置（单个笔记内最多 max_per_note 个）
fn locate_all_matches(content: &str, query: &str, max_per_note: usize) -> Vec<MatchLocation> {
    let mut results = Vec::new();
    let mut search_from = 0;
    let mut line = 0;
    let mut column = 0;

    while results.len() < max_per_note {
        let Some(byte_index) = find_case_insensitive(&content[search_from..], query) else {
            break;
        };
        let abs_byte_index = search_from + byte_index;

        // 从上次位置继续计算行列
        for ch in content[search_from..abs_byte_index].chars() {
            if ch == '\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
        }

        results.push(MatchLocation {
            byte_index: abs_byte_index,
            line,
            column,
        });

        // 移动到匹配之后继续搜索
        let match_len = content[abs_byte_index..]
            .chars()
            .take(query.chars().count().max(1))
            .map(char::len_utf8)
            .sum::<usize>()
            .max(1);
        let next_start = abs_byte_index + match_len;
        // 更新行列到 match 结束位置
        for ch in content[abs_byte_index..next_start.min(content.len())].chars() {
            if ch == '\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
        }
        search_from = next_start;
        if search_from >= content.len() {
            break;
        }
    }

    results
}

fn fallback_snippet(content: &str) -> String {
    content
        .chars()
        .take(60)
        .collect::<String>()
        .replace('\n', " ")
}

/// 提取匹配位置所在的完整行文本
fn extract_full_line(content: &str, byte_index: usize) -> String {
    let line_start = content[..byte_index].rfind('\n').map_or(0, |i| i + 1);
    let line_end = content[byte_index..]
        .find('\n')
        .map_or(content.len(), |i| byte_index + i);
    content[line_start..line_end].to_string()
}

/// 提取搜索结果摘要（保留供测试和未来使用）
#[cfg(test)]
fn extract_snippet(content: &str, byte_index: usize, query: &str) -> String {
    let match_len = content[byte_index..]
        .chars()
        .take(query.chars().count().max(1))
        .map(char::len_utf8)
        .sum::<usize>()
        .max(1);

    let start = content.floor_char_boundary(byte_index.saturating_sub(30));
    let end = content.ceil_char_boundary((byte_index + match_len + 30).min(content.len()));
    let mut snippet = content[start..end].to_string();

    if start > 0 {
        snippet.insert_str(0, "...");
    }
    if end < content.len() {
        snippet.push_str("...");
    }

    snippet.replace('\n', " ")
}

const MAX_MATCHES_PER_NOTE: usize = 10;
const MAX_TOTAL_RESULTS: usize = 200;

/// 全文搜索笔记（标题 + 内容），返回每个笔记的所有匹配行
pub fn search_notes(conn: &Connection, query: &str) -> Result<Vec<SearchResult>, rusqlite::Error> {
    let pattern = format!("%{query}%");
    let query_char_len = query.chars().count();
    let mut stmt = conn.prepare_cached(
        "SELECT id, folder_id, title, content
         FROM notes
         WHERE title LIKE ?1 OR content LIKE ?1
         ORDER BY updated_at DESC
         LIMIT 100",
    )?;

    let mut all_results = Vec::new();

    let rows = stmt.query_map(params![pattern], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    for row in rows {
        if all_results.len() >= MAX_TOTAL_RESULTS {
            break;
        }
        let (note_id, folder_id, title, content) = row?;
        let title_match = find_case_insensitive(&title, query).is_some();
        let content_matches = locate_all_matches(&content, query, MAX_MATCHES_PER_NOTE);

        if content_matches.is_empty() {
            // 仅标题命中
            let snippet = if title_match {
                if content.trim().is_empty() {
                    "标题命中".to_string()
                } else {
                    fallback_snippet(&content)
                }
            } else {
                continue;
            };
            all_results.push(SearchResult {
                note_id,
                folder_id,
                title,
                snippet,
                match_line: None,
                match_column: None,
                match_len: query_char_len,
            });
        } else {
            // 内容匹配：每个匹配位置一条结果
            for matched in content_matches {
                if all_results.len() >= MAX_TOTAL_RESULTS {
                    break;
                }
                all_results.push(SearchResult {
                    note_id: note_id.clone(),
                    folder_id: folder_id.clone(),
                    title: title.clone(),
                    snippet: extract_full_line(&content, matched.byte_index),
                    match_line: Some(matched.line),
                    match_column: Some(matched.column),
                    match_len: query_char_len,
                });
            }
        }
    }

    Ok(all_results)
}

#[cfg(test)]
mod move_tests {
    use rusqlite::Connection;

    #[test]
    fn move_note_updates_folder_and_sort_order() {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        crate::db::migration::run(&mut conn).expect("run migrations");

        let source =
            crate::db::folder::create_folder(&conn, None, "source").expect("create source folder");
        let target =
            crate::db::folder::create_folder(&conn, None, "target").expect("create target folder");

        let first =
            crate::db::note::create_note(&conn, &target.id, "first").expect("create first note");
        let moved =
            crate::db::note::create_note(&conn, &source.id, "moved").expect("create moved note");

        let updated =
            super::move_note(&conn, &moved.id, &target.id).expect("move note to target folder");

        assert_eq!(updated.folder_id, target.id);
        assert_eq!(updated.id, moved.id);
        assert_eq!(updated.sort_order, first.sort_order + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_snippet, locate_all_matches, locate_match};

    #[test]
    fn locate_match_reports_line_and_column() {
        let content = "第一行\n第二行搜索词\n第三行";
        let matched = locate_match(content, "搜索词").expect("match should exist");

        assert_eq!(matched.line, 1);
        assert_eq!(matched.column, 3);
    }

    #[test]
    fn locate_match_is_case_insensitive() {
        let content = "Hello WORLD";
        let matched = locate_match(content, "world").expect("match should exist");

        assert_eq!(matched.line, 0);
        assert_eq!(matched.column, 6);
    }

    #[test]
    fn extract_snippet_adds_ellipsis_when_needed() {
        let content =
            "0123456789abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz0123456789";
        let snippet = extract_snippet(content, 45, "jkl");

        assert!(snippet.starts_with("..."));
        assert!(snippet.ends_with("..."));
    }

    #[test]
    fn locate_all_matches_finds_multiple() {
        let content = "hello world\nhello rust\nsay hello";
        let matches = locate_all_matches(content, "hello", 10);

        assert_eq!(matches.len(), 3);
        assert_eq!((matches[0].line, matches[0].column), (0, 0));
        assert_eq!((matches[1].line, matches[1].column), (1, 0));
        assert_eq!((matches[2].line, matches[2].column), (2, 4));
    }

    #[test]
    fn locate_all_matches_respects_limit() {
        let content = "aa aa aa aa aa";
        let matches = locate_all_matches(content, "aa", 3);

        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn locate_all_matches_case_insensitive() {
        let content = "Hello HELLO hello";
        let matches = locate_all_matches(content, "hello", 10);

        assert_eq!(matches.len(), 3);
    }
}
