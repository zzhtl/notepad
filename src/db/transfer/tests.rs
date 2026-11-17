use super::*;

struct TestDirectory(PathBuf);
impl TestDirectory {
    fn new() -> Self {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/transfer-tests")
            .join(crate::util::id::new_id());
        fs::create_dir_all(&path).expect("create test directory");
        Self(path)
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn database() -> Connection {
    let mut conn = Connection::open_in_memory().expect("open db");
    conn.execute_batch("PRAGMA foreign_keys = ON")
        .expect("foreign keys");
    super::super::migration::run(&mut conn).expect("migrate");
    conn
}
fn add_note(
    conn: &Connection,
    folder: &str,
    title: &str,
    content: &str,
) -> crate::model::note::Note {
    let mut note = super::super::note::create_note(conn, folder, title).expect("create note");
    note.content = content.into();
    super::super::note::save_note(conn, &note).expect("save note");
    note
}
fn count(conn: &Connection, table: &str) -> usize {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .expect("count")
}

#[test]
fn directory_roundtrip_deduplicates_and_updates_in_place() {
    let conn = database();
    let folder = super::super::folder::create_folder(&conn, None, "工作").unwrap();
    let child = super::super::folder::create_folder(&conn, Some(&folder.id), "资料").unwrap();
    super::super::folder::create_folder(&conn, Some(&child.id), "空目录").unwrap();
    let note = add_note(&conn, &child.id, "计划", "# 第一版\n中文和 emoji 📝\n");
    let directory = TestDirectory::new();
    let report = export_all(&conn, &directory.0).unwrap();
    assert_eq!(report.folders, 3);
    assert_eq!(
        fs::read_to_string(report.path.join("工作/资料/计划.md")).unwrap(),
        note.content
    );
    assert!(report.path.join("工作/资料/空目录").is_dir());
    let skipped = import_all(&conn, &report.path).unwrap();
    assert_eq!(
        (skipped.created, skipped.updated, skipped.skipped),
        (0, 0, 1)
    );
    fs::write(report.path.join("工作/资料/计划.md"), "# 修改后的计划").unwrap();
    let updated = import_all(&conn, &report.path).unwrap();
    assert_eq!((updated.created, updated.updated), (0, 1));
    assert_eq!(
        super::super::note::load_note(&conn, &note.id)
            .unwrap()
            .content,
        "# 修改后的计划"
    );
    assert_eq!(count(&conn, "notes"), 1);
    let fresh = database();
    assert_eq!(import_all(&fresh, &report.path).unwrap().created, 1);
    assert_eq!(import_all(&fresh, &report.path).unwrap().skipped, 1);
    assert_eq!(count(&fresh, "folders"), 3);
}

#[test]
fn duplicate_and_unsafe_names_are_lossless_and_portable() {
    let conn = database();
    for name in ["同名", "同名", "../CON:", ASSETS] {
        let folder = super::super::folder::create_folder(&conn, None, name).unwrap();
        for title in ["a/b", "a:b", "重复", "重复", "CON", "Foo", "foo", ".."] {
            add_note(&conn, &folder.id, title, title);
        }
    }
    let directory = TestDirectory::new();
    let exported = export_all(&conn, &directory.0).unwrap();
    let fresh = database();
    assert_eq!(import_all(&fresh, &exported.path).unwrap().created, 32);
    assert_eq!(import_all(&fresh, &exported.path).unwrap().skipped, 32);
    assert_eq!(count(&fresh, "folders"), 4);
    assert_eq!(count(&fresh, "notes"), 32);
    assert_eq!(import_all(&conn, &exported.path).unwrap().skipped, 32);
    assert_eq!(safe_name("CON"), "_CON");
    assert_eq!(safe_name("a/b:c"), "a_b_c");
    let names: Vec<String> = fresh
        .prepare("SELECT title FROM notes ORDER BY title")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(names.iter().filter(|title| *title == "a/b").count(), 4);
}

#[test]
fn image_roundtrip_preserves_data_and_repeated_import_is_idempotent() {
    let conn = database();
    let folder = super::super::folder::create_folder(&conn, None, "图片").unwrap();
    let mut note = add_note(&conn, &folder.id, "图文", "");
    let image =
        super::super::image::store_image(&conn, &note.id, "示例 (1).png", b"image-v1").unwrap();
    note.content = format!(
        "# 图文\n![说明](notepad://image/{})\n\n```\n![code](missing.png)\n```",
        image.id
    );
    super::super::note::save_note(&conn, &note).unwrap();
    let directory = TestDirectory::new();
    let exported = export_all(&conn, &directory.0).unwrap();
    let disk_content = fs::read_to_string(exported.path.join("图片/图文.md")).unwrap();
    assert!(disk_content.contains(".notepad-assets/"));
    assert!(disk_content.contains("![code](missing.png)"));
    assert_eq!(import_all(&conn, &exported.path).unwrap().skipped, 1);
    let fresh = database();
    assert_eq!(import_all(&fresh, &exported.path).unwrap().created, 1);
    assert_eq!(import_all(&fresh, &exported.path).unwrap().skipped, 1);
    assert_eq!(count(&fresh, "images"), 1);
    let data: Vec<u8> = fresh
        .query_row("SELECT data FROM images", [], |r| r.get(0))
        .unwrap();
    assert_eq!(data, b"image-v1");
    let asset = fs::read_dir(exported.path.join("图片/.notepad-assets"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::write(asset, b"image-v2").unwrap();
    assert_eq!(import_all(&fresh, &exported.path).unwrap().updated, 1);
    assert_eq!(count(&fresh, "images"), 1);
    assert_eq!(import_all(&fresh, &exported.path).unwrap().skipped, 1);
}

#[test]
fn plain_markdown_root_files_and_distinct_paths() {
    let directory = TestDirectory::new();
    fs::create_dir_all(directory.0.join("子目录")).unwrap();
    fs::write(directory.0.join("根笔记.MD"), "\u{feff}# root").unwrap();
    fs::write(directory.0.join("子目录/根笔记.markdown"), "# child").unwrap();
    fs::write(directory.0.join("ignore.txt"), "ignored").unwrap();
    let conn = database();
    let report = import_all(&conn, &directory.0).unwrap();
    assert_eq!((report.created, report.ignored), (2, 1));
    assert_eq!(count(&conn, "folders"), 2);
    assert_eq!(import_all(&conn, &directory.0).unwrap().skipped, 2);
    fs::write(directory.0.join("根笔记.MD"), "updated").unwrap();
    let report = import_all(&conn, &directory.0).unwrap();
    assert_eq!((report.updated, report.skipped), (1, 1));
}

#[test]
fn invalid_utf8_rolls_back_updates_and_creations() {
    let directory = TestDirectory::new();
    fs::create_dir(directory.0.join("工作")).unwrap();
    fs::write(directory.0.join("工作/a.md"), "updated").unwrap();
    fs::write(directory.0.join("工作/b.md"), "new").unwrap();
    fs::write(directory.0.join("工作/z.md"), [0xff, 0xfe]).unwrap();
    let conn = database();
    let folder = super::super::folder::create_folder(&conn, None, "工作").unwrap();
    let note = add_note(&conn, &folder.id, "a", "original");
    assert!(
        import_all(&conn, &directory.0)
            .unwrap_err()
            .contains("未写入任何更改")
    );
    assert_eq!(
        super::super::note::load_note(&conn, &note.id)
            .unwrap()
            .content,
        "original"
    );
    assert_eq!(count(&conn, "notes"), 1);
}

#[test]
fn local_reference_images_and_code_are_handled_by_markdown_parser() {
    let directory = TestDirectory::new();
    fs::write(directory.0.join("image.png"), b"png").unwrap();
    let content =
        "![one][ref]\n\n![two][ref]\n\n[ref]: image.png \"标题\"\n\n`![code](missing.png)`";
    fs::write(directory.0.join("note.md"), content).unwrap();
    let conn = database();
    assert_eq!(import_all(&conn, &directory.0).unwrap().created, 1);
    assert_eq!(count(&conn, "images"), 1);
    assert_eq!(import_all(&conn, &directory.0).unwrap().skipped, 1);
    let content: String = conn
        .query_row("SELECT content FROM notes", [], |r| r.get(0))
        .unwrap();
    assert_eq!(content.matches("notepad://image/").count(), 2);
    assert!(content.contains("`![code](missing.png)`"));
}

#[test]
fn missing_images_and_outside_paths_roll_back() {
    let directory = TestDirectory::new();
    let source = directory.0.join("source");
    fs::create_dir(&source).unwrap();
    fs::write(directory.0.join("outside.png"), b"private").unwrap();
    let conn = database();
    for url in ["missing.png", "../outside.png"] {
        fs::write(source.join("note.md"), format!("![image]({url})")).unwrap();
        assert!(import_all(&conn, &source).is_err());
        assert_eq!(count(&conn, "notes"), 0);
        assert_eq!(count(&conn, "folders"), 0);
    }
}

#[cfg(unix)]
#[test]
fn symbolic_directories_are_not_followed() {
    let directory = TestDirectory::new();
    fs::write(directory.0.join("note.md"), "# safe").unwrap();
    std::os::unix::fs::symlink(&directory.0, directory.0.join("loop")).unwrap();
    let conn = database();
    let report = import_all(&conn, &directory.0).unwrap();
    assert_eq!((report.created, report.ignored), (1, 1));
}

#[test]
fn each_export_uses_a_new_directory_and_keeps_previous_files() {
    let directory = TestDirectory::new();
    let conn = database();
    let first = export_all(&conn, &directory.0).unwrap();
    fs::write(first.path.join("keep.md"), "keep me").unwrap();
    let second = export_all(&conn, &directory.0).unwrap();
    assert_ne!(first.path, second.path);
    assert_eq!(
        fs::read_to_string(first.path.join("keep.md")).unwrap(),
        "keep me"
    );
    assert_eq!(import_all(&conn, &second.path).unwrap().created, 0);
}
