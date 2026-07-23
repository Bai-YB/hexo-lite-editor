use crate::{
    domain::{AppConfigV3, AppError, AppResult, TaskType},
    platform::command_path,
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStep {
    pub name: &'static str,
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HexoExecutor {
    Local(PathBuf),
    Pnpm,
    Npm,
}

pub fn build_task_steps(
    kind: TaskType,
    config: &AppConfigV3,
    root: &Path,
) -> AppResult<Vec<TaskStep>> {
    if kind == TaskType::ServerStop {
        return Ok(Vec::new());
    }
    if kind == TaskType::GitStatus {
        return Ok(vec![TaskStep {
            name: "Git 状态",
            program: platform_program("git"),
            args: vec!["status".into(), "--short".into(), "--branch".into()],
        }]);
    }
    let executor = select_hexo_executor(root, command_available)?;
    let clean = hexo_step("清理缓存", &executor, "clean");
    let generate = hexo_step("生成站点", &executor, "generate");
    let deploy = hexo_step("部署站点", &executor, "deploy");
    Ok(match kind {
        TaskType::Clean => vec![clean],
        TaskType::Generate => vec![generate],
        TaskType::Deploy => vec![deploy],
        TaskType::Publish => {
            let mut steps = Vec::new();
            if config.publish.clean_before_generate {
                steps.push(clean);
            }
            if config.publish.generate_before_deploy {
                steps.push(generate);
            }
            steps.push(deploy);
            if config.publish.git_push_after_deploy {
                steps.push(TaskStep {
                    name: "推送 Git",
                    program: platform_program("git"),
                    args: vec!["push".into()],
                });
            }
            steps
        }
        TaskType::ServerStart => vec![hexo_step("启动预览", &executor, "server")],
        TaskType::GitStatus | TaskType::ServerStop => unreachable!("handled above"),
    })
}

fn hexo_step(name: &'static str, executor: &HexoExecutor, command: &str) -> TaskStep {
    match executor {
        HexoExecutor::Local(path) => TaskStep {
            name,
            program: path.display().to_string(),
            args: vec![command.to_string()],
        },
        HexoExecutor::Pnpm => TaskStep {
            name,
            program: platform_program("pnpm"),
            args: vec!["exec".into(), "hexo".into(), command.into()],
        },
        HexoExecutor::Npm => TaskStep {
            name,
            program: platform_program("npm"),
            args: vec!["exec".into(), "--".into(), "hexo".into(), command.into()],
        },
    }
}

fn select_hexo_executor(root: &Path, available: impl Fn(&str) -> bool) -> AppResult<HexoExecutor> {
    for local in local_hexo_candidates(root) {
        if local.is_file() {
            return Ok(HexoExecutor::Local(local));
        }
    }
    let package_manager = fs::read_to_string(root.join("package.json"))
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .and_then(|value| value.get("packageManager")?.as_str().map(str::to_string));
    let prefers_pnpm = package_manager
        .as_deref()
        .is_some_and(|value| value.starts_with("pnpm"))
        || root.join("pnpm-lock.yaml").is_file();
    if prefers_pnpm && available("pnpm") {
        return Ok(HexoExecutor::Pnpm);
    }
    if available("npm") {
        return Ok(HexoExecutor::Npm);
    }
    Err(AppError::new(
        "hexo_dependency_missing",
        "未找到项目本地 Hexo，也无法使用 pnpm/npm。请先在博客项目中安装依赖。",
        true,
    ))
}

fn local_hexo_candidates(root: &Path) -> Vec<PathBuf> {
    let bin = root.join("node_modules").join(".bin");
    if cfg!(windows) {
        vec![bin.join("hexo.cmd"), bin.join("hexo.exe"), bin.join("hexo")]
    } else {
        vec![bin.join("hexo")]
    }
}

fn command_available(program: &str) -> bool {
    if cfg!(windows) {
        Command::new("where.exe")
            .arg(platform_program(program))
            .env("PATH", command_path())
            .output()
            .is_ok_and(|output| output.status.success())
    } else {
        Command::new("sh")
            .args(["-c", &format!("command -v {program}")])
            .env("PATH", command_path())
            .output()
            .is_ok_and(|output| output.status.success())
    }
}

fn platform_program(program: &str) -> String {
    if cfg!(windows) && matches!(program, "npm" | "pnpm") {
        format!("{program}.cmd")
    } else {
        program.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn publish_steps_follow_settings() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("node_modules/.bin")).unwrap();
        fs::write(temp.path().join("node_modules/.bin/hexo.cmd"), "").unwrap();
        let mut config = AppConfigV3::default();
        config.publish.clean_before_generate = true;
        config.publish.generate_before_deploy = false;
        config.publish.git_push_after_deploy = true;
        let names: Vec<_> = build_task_steps(TaskType::Publish, &config, temp.path())
            .unwrap()
            .into_iter()
            .map(|step| step.name)
            .collect();
        assert_eq!(names, vec!["清理缓存", "部署站点", "推送 Git"]);
    }

    #[test]
    fn selects_local_then_pnpm_then_npm() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("node_modules/.bin")).unwrap();
        let local = temp.path().join(if cfg!(windows) {
            "node_modules/.bin/hexo.cmd"
        } else {
            "node_modules/.bin/hexo"
        });
        fs::write(&local, "").unwrap();
        assert!(matches!(
            select_hexo_executor(temp.path(), |_| true).unwrap(),
            HexoExecutor::Local(_)
        ));
        fs::remove_file(local).unwrap();
        fs::write(temp.path().join("pnpm-lock.yaml"), "lockfileVersion: '9.0'").unwrap();
        assert_eq!(
            select_hexo_executor(temp.path(), |name| name == "pnpm").unwrap(),
            HexoExecutor::Pnpm
        );
        fs::remove_file(temp.path().join("pnpm-lock.yaml")).unwrap();
        assert_eq!(
            select_hexo_executor(temp.path(), |name| name == "npm").unwrap(),
            HexoExecutor::Npm
        );
        assert_eq!(
            select_hexo_executor(temp.path(), |_| false)
                .unwrap_err()
                .code,
            "hexo_dependency_missing"
        );
    }
}
