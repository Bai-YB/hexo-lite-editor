use crate::domain::{AppError, AppResult};
use atomic_write_file::AtomicWriteFile;
use std::{fs, io::Write, path::Path};

const LEGACY_APP_IDENTIFIER: &str = "com.user.hexo-lite-editor";

pub fn atomic_write(path: &Path, bytes: &[u8]) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| AppError::io("无法创建目标目录", error))?;
    }
    let mut file =
        AtomicWriteFile::open(path).map_err(|error| AppError::io("无法创建原子写入文件", error))?;
    file.write_all(bytes)
        .map_err(|error| AppError::io("写入临时文件失败", error))?;
    file.commit()
        .map_err(|error| AppError::io("提交原子写入失败", error))
}

/// Copies data from the provisional 1.0.x application identifier without
/// deleting the old directory, so a downgrade remains recoverable.
pub fn migrate_legacy_app_data(config_dir: &Path) -> AppResult<bool> {
    let Some(parent) = config_dir.parent() else {
        return Ok(false);
    };
    let legacy_dir = parent.join(LEGACY_APP_IDENTIFIER);
    if legacy_dir == config_dir || !legacy_dir.is_dir() {
        return Ok(false);
    }
    copy_missing_tree(&legacy_dir, config_dir)?;
    Ok(true)
}

fn copy_missing_tree(source: &Path, destination: &Path) -> AppResult<()> {
    fs::create_dir_all(destination)
        .map_err(|error| AppError::io("无法创建新的应用数据目录", error))?;
    for item in
        fs::read_dir(source).map_err(|error| AppError::io("无法读取旧版应用数据目录", error))?
    {
        let item = item.map_err(|error| AppError::io("无法读取旧版应用数据", error))?;
        let file_type = item
            .file_type()
            .map_err(|error| AppError::io("无法识别旧版应用数据类型", error))?;
        if file_type.is_symlink() {
            continue;
        }
        let target = destination.join(item.file_name());
        if file_type.is_dir() {
            copy_missing_tree(&item.path(), &target)?;
        } else if file_type.is_file() && !target.exists() {
            fs::copy(item.path(), target)
                .map_err(|error| AppError::io("迁移旧版应用数据失败", error))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn atomically_replaces_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("config.json");
        atomic_write(&path, b"one").unwrap();
        atomic_write(&path, b"two").unwrap();
        assert_eq!(fs::read_to_string(path).unwrap(), "two");
    }

    #[test]
    fn copies_legacy_identifier_data_without_overwriting_or_deleting() {
        let temp = TempDir::new().unwrap();
        let legacy = temp.path().join(LEGACY_APP_IDENTIFIER);
        let current = temp.path().join("io.github.bai-yb.hexo-lite-editor");
        fs::create_dir_all(legacy.join("task-logs")).unwrap();
        fs::create_dir_all(&current).unwrap();
        fs::write(legacy.join("config-v3.json"), "legacy").unwrap();
        fs::write(legacy.join("task-logs").join("one.log"), "log").unwrap();
        fs::write(current.join("config-v3.json"), "current").unwrap();

        assert!(migrate_legacy_app_data(&current).unwrap());
        assert_eq!(
            fs::read_to_string(current.join("config-v3.json")).unwrap(),
            "current"
        );
        assert_eq!(
            fs::read_to_string(current.join("task-logs").join("one.log")).unwrap(),
            "log"
        );
        assert!(legacy.join("config-v3.json").is_file());
    }
}
