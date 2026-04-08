use rusqlite::{Connection, params};

use crate::model::image::ImageAttachment;
use crate::util::id::new_id;

/// 存储图片到数据库
pub fn store_image(
    conn: &Connection,
    note_id: &str,
    filename: &str,
    data: &[u8],
) -> Result<ImageAttachment, rusqlite::Error> {
    let id = new_id();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO images (id, note_id, filename, data, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, note_id, filename, data, now],
    )?;

    Ok(ImageAttachment {
        id,
        note_id: note_id.to_string(),
        filename: filename.to_string(),
        data: data.to_vec(),
        created_at: now,
    })
}

/// 加载笔记的所有图片
#[allow(dead_code)]
pub fn load_images(
    conn: &Connection,
    note_id: &str,
) -> Result<Vec<ImageAttachment>, rusqlite::Error> {
    let mut stmt = conn.prepare_cached(
        "SELECT id, note_id, filename, data, created_at
         FROM images WHERE note_id = ?1",
    )?;

    let images = stmt
        .query_map(params![note_id], |row| {
            Ok(ImageAttachment {
                id: row.get(0)?,
                note_id: row.get(1)?,
                filename: row.get(2)?,
                data: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?
        .collect::<Result<_, _>>()?;

    Ok(images)
}

/// 删除图片
#[allow(dead_code)]
pub fn delete_image(conn: &Connection, id: &str) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM images WHERE id = ?1", params![id])?;
    Ok(())
}
