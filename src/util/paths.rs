use std::io;
use std::path::{Path, PathBuf};

/// 安装包负责桌面入口；便携版仍使用原有的同目录数据库和快捷方式逻辑。
pub struct AppPaths {
    pub installed: bool,
    pub database: PathBuf,
}

pub fn resolve() -> io::Result<AppPaths> {
    resolve_at(&std::env::current_exe()?, dirs::data_local_dir().as_deref())
}

fn resolve_at(executable: &Path, data_directory: Option<&Path>) -> io::Result<AppPaths> {
    let directory = executable
        .parent()
        .ok_or_else(|| io::Error::other("无法获取程序目录"))?;
    // macOS 的 MacOS 目录只放代码，安装标识作为资源参与应用签名。
    let marker_directory = if directory.ends_with(Path::new("Contents").join("MacOS")) {
        directory.with_file_name("Resources")
    } else {
        directory.to_path_buf()
    };
    let installed = marker_directory.join("notepad.install").try_exists()?;
    let database = if installed {
        let directory = data_directory
            .ok_or_else(|| io::Error::other("无法获取用户数据目录"))?
            .join("notepad");
        std::fs::create_dir_all(&directory)?;
        directory.join("notepad.db")
    } else {
        directory.join("notepad.db")
    };
    Ok(AppPaths {
        installed,
        database,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDirectory(PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target/paths-tests")
                .join(crate::util::id::new_id());
            std::fs::create_dir_all(&directory).expect("create test directory");
            Self(directory)
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn portable_build_preserves_existing_database_location() {
        let root = TestDirectory::new();
        let database = root.0.join("notepad.db");
        std::fs::write(&database, b"existing data").unwrap();
        let paths = resolve_at(&root.0.join("notepad"), None).unwrap();
        assert!(!paths.installed);
        assert_eq!(paths.database, database);
        assert_eq!(std::fs::read(database).unwrap(), b"existing data");
    }

    #[test]
    fn installed_build_stores_data_outside_installation_and_keeps_it_on_upgrade() {
        let root = TestDirectory::new();
        let install = root.0.join("app");
        std::fs::create_dir(&install).unwrap();
        std::fs::write(install.join("notepad.install"), b"").unwrap();
        let data = root.0.join("user-data");
        let paths = resolve_at(&install.join("notepad"), Some(&data)).unwrap();
        assert!(paths.installed);
        assert_eq!(paths.database, data.join("notepad/notepad.db"));
        let db = crate::db::DbPool::open(&paths.database).expect("initialize installed database");
        drop(db);
        assert!(paths.database.is_file());
        assert!(!install.join("notepad.db").exists());
        let again = resolve_at(&install.join("notepad"), Some(&data)).unwrap();
        assert_eq!(again.database, paths.database);
    }

    #[test]
    fn macos_bundle_reads_installed_marker_from_resources() {
        let root = TestDirectory::new();
        let contents = root.0.join("Notepad.app/Contents");
        std::fs::create_dir_all(contents.join("MacOS")).unwrap();
        std::fs::create_dir_all(contents.join("Resources")).unwrap();
        std::fs::write(contents.join("Resources/notepad.install"), b"").unwrap();
        let data = root.0.join("user-data");
        let paths = resolve_at(&contents.join("MacOS/notepad"), Some(&data)).unwrap();
        assert!(paths.installed);
        assert_eq!(paths.database, data.join("notepad/notepad.db"));
        assert!(!contents.join("MacOS/notepad.install").exists());
    }

    #[test]
    fn installed_build_does_not_fall_back_to_protected_program_directory() {
        let root = TestDirectory::new();
        std::fs::write(root.0.join("notepad.install"), b"").unwrap();
        assert!(resolve_at(&root.0.join("notepad"), None).is_err());
        assert!(!root.0.join("notepad.db").exists());
    }
}
