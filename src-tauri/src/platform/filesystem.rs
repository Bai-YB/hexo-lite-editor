use crate::domain::{AppError, AppResult};
use atomic_write_file::AtomicWriteFile;
use std::{fs, io::Write, path::Path};

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
}
