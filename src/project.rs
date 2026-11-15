//! Проект на диске: `form.lazr` (модель) и `app/` (сгенерированное приложение).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::codegen;
use crate::model::{self, Field, FieldType, Screen, Widget, WidgetKind};

/// Иконка приложения: тот же логотип, что на странице lazr.
const ICON_SVG: &str = include_str!("../docs/logo.svg");

pub struct Project {
    pub dir: PathBuf,
    pub screen: Screen,
}

const FORM: &str = "form.lazr";

impl Project {
    pub fn load(dir: impl AsRef<Path>) -> Result<Project, String> {
        let dir = dir.as_ref().to_path_buf();
        let path = dir.join(FORM);
        let text = std::fs::read_to_string(&path)
            .map_err(|err| format!("не прочитать {}: {err}", path.display()))?;
        let screen = model::parse(&text).map_err(|errors| {
            let list: Vec<String> = errors.iter().map(|e| e.to_json()).collect();
            format!("{{\"ok\":false,\"errors\":[{}]}}", list.join(","))
        })?;
        Ok(Project { dir, screen })
    }

    pub fn create(
        dir: impl AsRef<Path>,
        title: &str,
        width: u32,
        height: u32,
    ) -> Result<Project, String> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)
            .map_err(|err| format!("не создать {}: {err}", dir.display()))?;
        let screen = Screen {
            title: title.to_string(),
            width,
            height,
            fields: Vec::new(),
            rust: Vec::new(),
            rust_fields: Vec::new(),
            theme: Default::default(),
            icon_font: None,
            ui_font: None,
            window: Default::default(),
            root: vec![Widget::container(WidgetKind::Vbox)],
        };
        let project = Project { dir, screen };
        project.save()?;
        Ok(project)
    }

    pub fn form_path(&self) -> PathBuf {
        self.dir.join(FORM)
    }

    pub fn app_dir(&self) -> PathBuf {
        self.dir.join("app")
    }

    pub fn main_path(&self) -> String {
        self.app_dir().join("src/main.rs").display().to_string()
    }

    pub fn save(&self) -> Result<(), String> {
        std::fs::write(self.form_path(), model::render(&self.screen))
            .map_err(|err| format!("не записать модель: {err}"))
    }

    /// Перегенерировать приложение. Ошибки модели возвращаются как JSON, а не паника.
    pub fn generate(&self) -> Result<(), String> {
        let bin = codegen::bin_name(&self.screen);
        let main = codegen::main_rs(&self.screen, &bin)?;
        let app = self.app_dir();
        std::fs::create_dir_all(app.join("src"))
            .map_err(|err| format!("не создать app/: {err}"))?;
        std::fs::write(app.join("Cargo.toml"), codegen::cargo_toml(&bin))
            .map_err(|err| format!("не записать Cargo.toml: {err}"))?;
        std::fs::write(app.join("src/main.rs"), main)
            .map_err(|err| format!("не записать main.rs: {err}"))?;
        Ok(())
    }

    /// Общий каталог сборки: агент пересобирает приложение десятки раз, и каждый
    /// новый проект не должен заново компилировать egui.
    fn target_dir(&self) -> PathBuf {
        if let Ok(explicit) = std::env::var("LAZR_TARGET_DIR") {
            return PathBuf::from(explicit);
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".cache/lazr/target")
    }

    /// Сеть не нужна по умолчанию: `--offline` быстрее и детерминированнее.
    /// `LAZR_NET=1` отключает это — для чистого реестра (CI, первый запуск).
    fn offline_flag() -> Option<&'static str> {
        if std::env::var("LAZR_NET").as_deref() == Ok("1") {
            None
        } else {
            Some("--offline")
        }
    }

    pub fn build(&self) -> Result<String, String> {
        self.generate()?;
        let mut command = Command::new("cargo");
        command.arg("build");
        if let Some(flag) = Self::offline_flag() {
            command.arg(flag);
        }
        let output = command
            .env("CARGO_TARGET_DIR", self.target_dir())
            .current_dir(self.app_dir())
            .output()
            .map_err(|err| format!("cargo не запустился: {err}"))?;
        let mut log = String::from_utf8_lossy(&output.stdout).to_string();
        log.push_str(&String::from_utf8_lossy(&output.stderr));
        if !output.status.success() {
            return Err(format!(
                "{{\"ok\":false,\"stage\":\"build\",\"log\":{}}}",
                model::json_str(&log)
            ));
        }
        Ok(log)
    }

    pub fn binary(&self) -> PathBuf {
        let bin = codegen::bin_name(&self.screen);
        self.target_dir().join("debug").join(bin)
    }

    /// Упаковка: бинарь, иконка, .desktop и архив в dist/.
    ///
    /// Установщика нет и не будет, пока не появится подпись: это честная
    /// граница — .desktop + иконка + tarball, а не инсталлятор.
    pub fn package(&self, release: bool) -> Result<Vec<String>, String> {
        let profile = if release { "release" } else { "debug" };
        let mut args = vec!["build".to_string()];
        if release {
            args.push("--release".to_string());
        }
        if let Some(flag) = Self::offline_flag() {
            args.push(flag.to_string());
        }
        let output = Command::new("cargo")
            .args(&args)
            .env("CARGO_TARGET_DIR", self.target_dir())
            .current_dir(self.app_dir())
            .output()
            .map_err(|err| format!("cargo не запустился: {err}"))?;
        if !output.status.success() {
            return Err(format!(
                "{{\"ok\":false,\"stage\":\"package\",\"log\":{}}}",
                model::json_str(&String::from_utf8_lossy(&output.stderr))
            ));
        }
        let bin = codegen::bin_name(&self.screen);
        let built = self.target_dir().join(profile).join(&bin);
        if !built.exists() {
            return Err(format!("сборка прошла, а бинаря нет: {}", built.display()));
        }
        let dist = self.dir.join("dist");
        std::fs::create_dir_all(&dist).map_err(|err| format!("dist: {err}"))?;
        std::fs::copy(&built, dist.join(&bin)).map_err(|err| format!("копия бинаря: {err}"))?;

        let icon_svg = dist.join("icon.svg");
        std::fs::write(&icon_svg, ICON_SVG).map_err(|err| format!("icon.svg: {err}"))?;
        let icon_png = dist.join("icon.png");
        let mut icon_line = "icon.png".to_string();
        let converted = Command::new("rsvg-convert")
            .arg("-w")
            .arg("256")
            .arg("-h")
            .arg("256")
            .arg(&icon_svg)
            .arg("-o")
            .arg(&icon_png)
            .status();
        if !matches!(converted, Ok(status) if status.success()) {
            icon_line = "icon.svg".to_string();
        }

        let desktop = format!(
            "[Desktop Entry]\nType=Application\nName={title}\nComment=Собрано lazr из модели экрана\nExec={bin}\nIcon={icon}\nTerminal=false\nCategories=Utility;\n",
            title = self.screen.title,
            bin = bin,
            icon = icon_line,
        );
        std::fs::write(dist.join(format!("{bin}.desktop")), desktop)
            .map_err(|err| format!("desktop: {err}"))?;

        let archive = dist.join(format!("{bin}-{profile}.tar.gz"));
        let status = Command::new("tar")
            .arg("-czf")
            .arg(&archive)
            .arg("-C")
            .arg(&dist)
            .arg(&bin)
            .arg(format!("{bin}.desktop"))
            .arg(&icon_line)
            .status();
        if !matches!(status, Ok(status) if status.success()) {
            return Err("tar не смог собрать архив".to_string());
        }

        Ok(vec![
            format!("бинарь: {}", dist.join(&bin).display()),
            format!("иконка: {}", dist.join(&icon_line).display()),
            format!("ярлык:  {}", dist.join(format!("{bin}.desktop")).display()),
            format!("архив:  {}", archive.display()),
        ])
    }

    /// Полная проверка собранной программы: компиляция, кадр без окна, прогон действий.
    ///
    /// «Работает» — это три факта, а не один: собралось, нарисовалось, обработчики
    /// не падают при вызове. Возвращает отчёт строками.
    pub fn verify(&self) -> Result<Vec<String>, String> {
        let mut report = Vec::new();
        self.build()?;
        report.push("сборка: ок".to_string());
        report.push(self.headless()?);
        let binary = self.binary();
        let out = Command::new(&binary)
            .arg("--selftest")
            .output()
            .map_err(|err| format!("не запустить {}: {err}", binary.display()))?;
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        if !out.status.success() {
            return Err(format!(
                "{{\"ok\":false,\"stage\":\"selftest\",\"stdout\":{},\"stderr\":{}}}",
                model::json_str(&stdout),
                model::json_str(&stderr)
            ));
        }
        report.push(stdout.trim().to_string());
        Ok(report)
    }

    /// Один кадр интерфейса без окна: проверка, что UI исполняется, а не только компилируется.
    pub fn headless(&self) -> Result<String, String> {
        let binary = self.binary();
        if !binary.exists() {
            return Err(format!("сначала сборка: нет {}", binary.display()));
        }
        let output = Command::new(&binary)
            .arg("--headless")
            .output()
            .map_err(|err| format!("не запустить {}: {err}", binary.display()))?;
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        if !output.status.success() {
            return Err(format!(
                "{{\"ok\":false,\"stage\":\"headless\",\"stderr\":{}}}",
                model::json_str(&String::from_utf8_lossy(&output.stderr))
            ));
        }
        Ok(text.trim().to_string())
    }

    pub fn add_state(&mut self, name: &str, ty: FieldType, value: &str) -> Result<(), String> {
        if self.screen.fields.iter().any(|f| f.name == name) {
            return Err(format!("поле «{name}» уже объявлено"));
        }
        self.screen.fields.push(Field {
            name: name.to_string(),
            ty,
            value: value.to_string(),
        });
        Ok(())
    }

    pub fn add_widget(
        &mut self,
        kind: WidgetKind,
        text: &str,
        bind: Option<&str>,
        effect: Option<&str>,
        container: Option<&str>,
    ) -> Result<(), String> {
        self.add_widget_with(kind, text, bind, effect, container, None, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_widget_with(
        &mut self,
        kind: WidgetKind,
        text: &str,
        bind: Option<&str>,
        effect: Option<&str>,
        container: Option<&str>,
        number: Option<f32>,
        style: Option<crate::model::Style>,
    ) -> Result<(), String> {
        let mut widget = if kind.is_container() {
            Widget::container(kind)
        } else {
            Widget::leaf(kind, text)
        };
        widget.bind = bind.map(str::to_string);
        widget.effect = effect.map(str::to_string);
        widget.number = number;
        if let Some(style) = style {
            widget.style = style;
        }
        // Без явного контейнера кладём в первый vbox/hbox: так ИИ не обязан
        // знать структуру, а виджет попадает туда, где его увидят.
        let default_container = container.map(str::to_string).or_else(|| {
            if self.screen.root.iter().any(|w| w.kind.is_container()) {
                self.screen
                    .root
                    .iter()
                    .find(|w| w.kind.is_container())
                    .map(|w| w.kind.as_str().to_string())
            } else {
                None
            }
        });
        model::add_widget(&mut self.screen, default_container.as_deref(), widget)
            .map_err(|err| err.message)?;
        // Сразу проверяем типы: ошибка видна на месте, а не при сборке.
        codegen::main_rs(&self.screen, "app")?;
        Ok(())
    }

    pub fn screen_text(&self) -> String {
        model::render(&self.screen)
    }

    pub fn count(&self) -> usize {
        codegen::count_widgets(&self.screen.root)
    }

    pub fn widgets_json(&self) -> String {
        fn walk(widgets: &[Widget], path: &str, out: &mut Vec<String>) {
            for (index, widget) in widgets.iter().enumerate() {
                let here = format!("{path}/{index}");
                let mut extra = String::new();
                if let Some(bind) = &widget.bind {
                    extra.push_str(&format!(",\"bind\":{}", model::json_str(bind)));
                }
                if let Some(effect) = &widget.effect {
                    extra.push_str(&format!(",\"effect\":{}", model::json_str(effect)));
                }
                out.push(format!(
                    "{{\"path\":{},\"kind\":{},\"text\":{}{}}}",
                    model::json_str(&here),
                    model::json_str(widget.kind.as_str()),
                    model::json_str(&widget.text),
                    extra
                ));
                walk(&widget.children, &here, out);
            }
        }
        let mut items = Vec::new();
        walk(&self.screen.root, "", &mut items);
        format!(
            "{{\"ok\":true,\"title\":{},\"count\":{},\"widgets\":[{}]}}",
            model::json_str(&self.screen.title),
            items.len(),
            items.join(",")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lazr-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn create_then_load_keeps_the_model() {
        let dir = temp("create");
        let mut project = Project::create(&dir, "Счётчик", 400, 300).unwrap();
        project.add_state("count", FieldType::Int, "0").unwrap();
        project
            .add_widget(WidgetKind::Button, "Плюс", None, Some("count += 1"), None)
            .unwrap();
        project.save().unwrap();
        let again = Project::load(&dir).unwrap();
        assert_eq!(again.screen.title, "Счётчик");
        assert_eq!(again.screen.fields.len(), 1);
        assert_eq!(again.count(), 2, "vbox + кнопка");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn generate_writes_a_compilable_looking_project() {
        let dir = temp("gen");
        let mut project = Project::create(&dir, "My App", 320, 200).unwrap();
        project.add_state("name", FieldType::Str, "").unwrap();
        project
            .add_widget(WidgetKind::TextInput, "", Some("name"), None, None)
            .unwrap();
        project.generate().unwrap();
        let main = std::fs::read_to_string(project.main_path()).unwrap();
        assert!(main.contains("pub name: String"));
        assert!(main.contains("text_edit_singleline"));
        let cargo = std::fs::read_to_string(project.app_dir().join("Cargo.toml")).unwrap();
        assert!(cargo.contains("eframe"));
        assert!(cargo.contains("name = \"my-app\""));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bad_binding_is_refused_at_add_time() {
        let dir = temp("badbind");
        let mut project = Project::create(&dir, "x", 100, 100).unwrap();
        project.add_state("count", FieldType::Int, "0").unwrap();
        let err = project
            .add_widget(WidgetKind::TextInput, "", Some("count"), None, None)
            .unwrap_err();
        assert!(err.contains("string"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn widgets_json_lists_paths() {
        let dir = temp("json");
        let mut project = Project::create(&dir, "x", 100, 100).unwrap();
        project
            .add_widget(WidgetKind::Label, "привет", None, None, None)
            .unwrap();
        let json = project.widgets_json();
        assert!(json.contains("\"kind\":\"vbox\""));
        assert!(json.contains("\"kind\":\"label\""));
        assert!(json.contains("\"path\":\"/0/0\""));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
