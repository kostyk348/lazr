//! `lazr live FILE` — приложение, читающее модель на ходу.
//!
//! Один раз собираем рантайм (шаблон без генерации под экран), дальше запускаем
//! его с путём к модели: правка файла меняет окно без пересборки.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::runtime::RUNTIME_SRC;

fn cache_root() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cache/lazr/live")
}

fn cargo_toml() -> String {
    let version = crate::codegen::EGUI_VERSION;
    format!(
        r#"[package]
name = "live"
version = "0.1.0"
edition = "2021"

[dependencies]
eframe = {{ version = "{version}", default-features = false, features = ["glow", "default_fonts", "x11", "wayland"] }}
egui = {{ version = "{version}", features = ["monochrome_emoji_fonts"] }}

[[bin]]
name = "live"
path = "src/main.rs"
"#
    )
}

fn target_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("LAZR_TARGET_DIR") {
        return PathBuf::from(dir);
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cache/lazr/target")
}

/// Собирает рантайм (если ещё не собран) и запускает его с моделью `file`.
///
/// Возвращает путь к бинарю: сама сборка отделена, чтобы вызывающий мог
/// прогнать `--headless` или передать аргументы.
pub fn prepare(file: &Path) -> Result<PathBuf, String> {
    if !file.exists() {
        return Err(format!("нет файла модели: {}", file.display()));
    }
    let dir = cache_root();
    let src = dir.join("src");
    std::fs::create_dir_all(&src).map_err(|err| format!("{}: {err}", src.display()))?;
    std::fs::write(dir.join("Cargo.toml"), cargo_toml())
        .map_err(|err| format!("Cargo.toml: {err}"))?;
    let main_path = src.join("main.rs");
    // перезаписываем только если шаблон изменился — иначе cargo пересоберёт зря
    let changed = std::fs::read_to_string(&main_path)
        .map(|current| current != RUNTIME_SRC)
        .unwrap_or(true);
    if changed {
        std::fs::write(&main_path, RUNTIME_SRC).map_err(|err| format!("main.rs: {err}"))?;
    }
    let mut command = Command::new("cargo");
    command.arg("build").arg("--release");
    if std::env::var("LAZR_NET").as_deref() != Ok("1") {
        command.arg("--offline");
    }
    let output = command
        .env("CARGO_TARGET_DIR", target_dir())
        .current_dir(&dir)
        .output()
        .map_err(|err| format!("cargo не запустился: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "сборка рантайма не прошла:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(target_dir().join("release/live"))
}
