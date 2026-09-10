use crate::domain::{AppError, AppResult, ArticleKind};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn folder(kind: ArticleKind) -> &'static str {
    match kind {
        ArticleKind::Post => "_posts",
        ArticleKind::Draft => "_drafts",
    }
}

/// Move the Markdown file and Hexo's same-name asset folder together, preserving
/// nested source directories. Preflight both destinations before changing files.
pub fn move_article_files(
    root: &Path,
    source: &Path,
    current: ArticleKind,
    next: ArticleKind,
) -> AppResult<PathBuf> {
    let source_root = root
        .join("source")
        .join(folder(current))
        .canonicalize()
        .map_err(|error| AppError::io("无法验证文章目录", error))?;
    let relative = source
        .strip_prefix(&source_root)
        .map_err(|_| AppError::invalid("文章不属于当前文章目录。"))?;
    let destination_root = root.join("source").join(folder(next));
    fs::create_dir_all(&destination_root)
        .map_err(|error| AppError::io("无法创建文章目录", error))?;
    let destination_root = destination_root
        .canonicalize()
        .map_err(|error| AppError::io("无法验证目标目录", error))?;
    if !destination_root.starts_with(root) {
        return Err(AppError::invalid("目标文章目录不属于当前项目。"));
    }
    let target = destination_root.join(relative);
    let parent = target
        .parent()
        .ok_or_else(|| AppError::invalid("目标文章路径无效。"))?;
    // Check each existing ancestor before creating nested directories.
    let mut ancestor = parent;
    while !ancestor.exists() {
        ancestor = ancestor
            .parent()
            .ok_or_else(|| AppError::invalid("目标文章路径无效。"))?;
    }
    if !ancestor
        .canonicalize()
        .map_err(|error| AppError::io("无法验证目标目录", error))?
        .starts_with(&destination_root)
    {
        return Err(AppError::invalid("目标文章目录不属于当前项目。"));
    }
    let assets = source.with_extension("");
    let target_assets = target.with_extension("");
    if target.exists() || target_assets.exists() {
        return Err(AppError::new(
            "article_exists",
            "目标位置已有同名文章或资源目录，请先处理冲突。",
            true,
        ));
    }
    let has_assets = assets.is_dir();
    if has_assets
        && !assets
            .canonicalize()
            .map_err(|error| AppError::io("无法验证文章资源目录", error))?
            .starts_with(&source_root)
    {
        return Err(AppError::invalid("文章资源目录不属于当前项目。"));
    }
    fs::create_dir_all(parent).map_err(|error| AppError::io("无法创建文章目录", error))?;
    if has_assets {
        fs::rename(&assets, &target_assets)
            .map_err(|error| AppError::io("移动文章资源目录失败", error))?;
    }
    if let Err(error) = fs::rename(source, &target) {
        if has_assets {
            fs::rename(&target_assets, &assets).map_err(|rollback| {
                AppError::new(
                    "article_move_rollback_failed",
                    format!(
                        "文章移动失败，资源目录回退失败，请检查原文与目标目录：{error}；{rollback}"
                    ),
                    true,
                )
            })?;
        }
        return Err(AppError::io("移动文章失败，资源目录已恢复", error));
    }
    target
        .canonicalize()
        .map_err(|error| AppError::io("验证移动后的文章失败", error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn moves_nested_article_and_assets_in_both_directions() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let draft = root.join("source/_drafts/notes/foo.md");
        fs::create_dir_all(draft.with_extension("")).unwrap();
        fs::write(&draft, "![photo](photo.png)").unwrap();
        fs::write(draft.with_extension("").join("photo.png"), b"photo").unwrap();
        let post =
            move_article_files(&root, &draft, ArticleKind::Draft, ArticleKind::Post).unwrap();
        assert!(post.ends_with("source/_posts/notes/foo.md"));
        assert_eq!(
            fs::read(post.with_extension("").join("photo.png")).unwrap(),
            b"photo"
        );
        assert!(!draft.exists());
        assert!(!draft.with_extension("").exists());
        assert_eq!(
            move_article_files(&root, &post, ArticleKind::Post, ArticleKind::Draft).unwrap(),
            draft
        );
    }

    #[test]
    fn asset_conflict_does_not_partially_move_article() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let source = root.join("source/_drafts/foo.md");
        fs::create_dir_all(source.with_extension("")).unwrap();
        fs::write(&source, "body").unwrap();
        fs::create_dir_all(root.join("source/_posts/foo")).unwrap();
        assert!(move_article_files(&root, &source, ArticleKind::Draft, ArticleKind::Post).is_err());
        assert_eq!(fs::read_to_string(&source).unwrap(), "body");
        assert!(source.with_extension("").is_dir());
        assert!(!root.join("source/_posts/foo.md").exists());
    }
}
