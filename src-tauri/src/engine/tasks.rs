use crate::{
    domain::{AppConfigV3, AppError, AppResult, TaskType},
    platform::{command_path, silent_command},
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStep {
    pub name: &'static str,
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HexoExecutor {
    Direct(PathBuf),
    Node { script: PathBuf },
    Pnpm,
    Npm,
}

#[cfg(windows)]
const WINDOWS_HEXO_BOOTSTRAP: &str = r#"
const childProcess = require('node:child_process');
for (const name of ['spawn', 'spawnSync']) {
  const original = childProcess[name];
  childProcess[name] = function(command, args, options) {
    if (!Array.isArray(args)) {
      options = args;
      args = [];
    }
    return original.call(this, command, args, { ...(options || {}), windowsHide: true });
  };
}
require(process.argv[1]);
"#;

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
            // Publishing must never deploy a stale public directory, regardless of legacy settings.
            let mut steps = vec![clean, generate, deploy];
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
        HexoExecutor::Direct(path) => TaskStep {
            name,
            program: path.display().to_string(),
            args: vec![command.to_string()],
        },
        HexoExecutor::Node { script } => TaskStep {
            name,
            program: "node".to_string(),
            args: windows_hexo_args(script, command),
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
    if cfg!(windows) {
        if let Some(script) = local_hexo_node_candidates(root)
            .into_iter()
            .find(|candidate| candidate.is_file())
        {
            if available("node") {
                return Ok(HexoExecutor::Node { script });
            }
            return Err(AppError::new(
                "node_runtime_missing",
                "已找到项目本地 Hexo，但未找到 Node.js。请先安装或修复 Node.js。",
                true,
            ));
        }
        let local_exe = root.join("node_modules").join(".bin").join("hexo.exe");
        if local_exe.is_file() {
            return Ok(HexoExecutor::Direct(local_exe));
        }
        return Err(AppError::new(
            "hexo_dependency_missing",
            "未找到项目本地 Hexo。请先在博客项目中安装依赖。",
            true,
        ));
    }

    for local in local_hexo_candidates(root) {
        if local.is_file() {
            return Ok(HexoExecutor::Direct(local));
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
    vec![bin.join("hexo")]
}

fn local_hexo_node_candidates(root: &Path) -> Vec<PathBuf> {
    let bin = root.join("node_modules").join("hexo").join("bin");
    vec![bin.join("hexo"), bin.join("hexo.js")]
}

fn node_path_argument(path: &Path) -> String {
    if cfg!(windows) {
        let path = path.display().to_string();
        let path = path
            .strip_prefix(r"\\?\UNC\")
            .map(|value| format!(r"\\{value}"))
            .or_else(|| path.strip_prefix(r"\\?\").map(str::to_string))
            .unwrap_or(path);
        path.replace('\\', "/")
    } else {
        path.display().to_string()
    }
}

fn windows_hexo_args(script: &Path, command: &str) -> Vec<String> {
    #[cfg(windows)]
    {
        vec![
            "-e".to_string(),
            WINDOWS_HEXO_BOOTSTRAP.to_string(),
            node_path_argument(script),
            command.to_string(),
        ]
    }
    #[cfg(not(windows))]
    {
        vec![node_path_argument(script), command.to_string()]
    }
}

fn command_available(program: &str) -> bool {
    if cfg!(windows) {
        silent_command("where.exe")
            .arg(platform_program(program))
            .env("PATH", command_path())
            .output()
            .is_ok_and(|output| output.status.success())
    } else {
        silent_command("sh")
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
    fn publish_always_cleans_and_generates_before_deploy() {
        let temp = TempDir::new().unwrap();
        let local = if cfg!(windows) {
            temp.path().join("node_modules/hexo/bin/hexo")
        } else {
            temp.path().join("node_modules/.bin/hexo")
        };
        fs::create_dir_all(local.parent().unwrap()).unwrap();
        fs::write(local, "").unwrap();
        let mut config = AppConfigV3::default();
        config.publish.clean_before_generate = false;
        config.publish.generate_before_deploy = false;
        config.publish.git_push_after_deploy = true;
        let names: Vec<_> = build_task_steps(TaskType::Publish, &config, temp.path())
            .unwrap()
            .into_iter()
            .map(|step| step.name)
            .collect();
        assert_eq!(names, vec!["清理缓存", "生成站点", "部署站点", "推送 Git"]);
    }

    #[test]
    fn selects_local_then_pnpm_then_npm() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("node_modules/.bin")).unwrap();
        let local = temp.path().join(if cfg!(windows) {
            "node_modules/hexo/bin/hexo"
        } else {
            "node_modules/.bin/hexo"
        });
        fs::create_dir_all(local.parent().unwrap()).unwrap();
        fs::write(&local, "").unwrap();
        assert!(
            matches!(
                select_hexo_executor(temp.path(), |_| true).unwrap(),
                HexoExecutor::Node { .. } if cfg!(windows)
            ) || matches!(
                select_hexo_executor(temp.path(), |_| true).unwrap(),
                HexoExecutor::Direct(_) if !cfg!(windows)
            )
        );
        fs::remove_file(local).unwrap();
        if cfg!(windows) {
            assert_eq!(
                select_hexo_executor(temp.path(), |_| true)
                    .unwrap_err()
                    .code,
                "hexo_dependency_missing"
            );
            return;
        }
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

    #[cfg(windows)]
    #[test]
    fn windows_runs_local_hexo_with_node_instead_of_cmd() {
        let temp = TempDir::new().unwrap();
        let script = temp.path().join("node_modules/hexo/bin/hexo");
        fs::create_dir_all(script.parent().unwrap()).unwrap();
        fs::write(&script, "").unwrap();
        let mut config = AppConfigV3::default();
        config.publish.git_push_after_deploy = false;

        let steps = build_task_steps(TaskType::Publish, &config, temp.path()).unwrap();

        assert!(steps.iter().all(|step| !step.program.ends_with(".cmd")));
        assert!(steps.iter().all(|step| step.program == "node"));
        let commands: Vec<_> = steps
            .iter()
            .map(|step| step.args.last().map(String::as_str))
            .collect();
        assert_eq!(
            commands,
            vec![Some("clean"), Some("generate"), Some("deploy")]
        );
        assert!(steps.iter().all(|step| {
            step.args[..3]
                == [
                    "-e".to_string(),
                    WINDOWS_HEXO_BOOTSTRAP.to_string(),
                    node_path_argument(&script),
                ]
        }));
        assert!(steps
            .iter()
            .all(|step| step.args[1].contains("windowsHide: true")));
    }

    #[cfg(windows)]
    #[test]
    fn windows_node_path_removes_verbatim_prefix() {
        assert_eq!(
            node_path_argument(Path::new(r"\\?\D:\Hexo\node_modules\hexo\bin\hexo")),
            "D:/Hexo/node_modules/hexo/bin/hexo"
        );
        assert_eq!(
            node_path_argument(Path::new(r"\\?\UNC\server\share\hexo\bin\hexo")),
            "//server/share/hexo/bin/hexo"
        );
    }
}
