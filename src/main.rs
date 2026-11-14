//! `lazr` — RAD для Rust, заточенный под ИИ.
//!
//! Экран описывается моделью (`form.lazr`), а не кодом и не пикселями. ИИ правит
//! модель семантическими операциями, `lazr` генерирует настоящий проект на
//! egui/eframe, собирает его и исполняет один кадр без окна.
//!
//! ```text
//! lazr new DIR --title "Счётчик" --size 420x260
//! lazr state DIR count int 0
//! lazr add DIR label "Нажатий: {count}"
//! lazr add DIR button "Плюс" --do "count += 1"
//! lazr gen DIR && lazr build DIR && lazr headless DIR
//! lazr mcp          # то же самое для агента, по MCP
//! lazr demo
//! ```

mod codegen;
mod expr;
mod lang;
mod live;
mod mcp;
mod model;
mod project;
mod runtime;

use std::path::PathBuf;

use model::{FieldType, WidgetKind};
use project::Project;

struct Opts {
    args: Vec<String>,
}

impl Opts {
    fn parse(args: &[String]) -> Opts {
        Opts {
            args: args.to_vec(),
        }
    }
    fn positional(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut skip = false;
        for (index, item) in self.args.iter().enumerate() {
            if skip {
                skip = false;
                continue;
            }
            if item.starts_with("--") {
                let takes_value =
                    !matches!(item.as_str(), "--help" | "-h" | "--json" | "--no-build");
                if takes_value {
                    skip = true;
                }
                continue;
            }
            let _ = index;
            out.push(item.clone());
        }
        out
    }
    fn flag(&self, name: &str) -> Option<&str> {
        let spelled = format!("--{name}");
        self.args
            .iter()
            .position(|item| item.as_str() == spelled.as_str())
            .and_then(|index| self.args.get(index + 1))
            .map(String::as_str)
    }
    fn has(&self, name: &str) -> bool {
        let spelled = format!("--{name}");
        self.args
            .iter()
            .any(|item| item.as_str() == spelled.as_str())
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("new") => cmd_new(&args[1..]),
        Some("state") => cmd_state(&args[1..]),
        Some("add") => cmd_add(&args[1..]),
        Some("show") => cmd_show(&args[1..]),
        Some("gen") => cmd_gen(&args[1..]),
        Some("build") => cmd_build(&args[1..]),
        Some("package") => cmd_package(&args[1..]),
        Some("verify") => cmd_verify(&args[1..]),
        Some("headless") => cmd_headless(&args[1..]),
        Some("live") => cmd_live(&args[1..]),
        Some("lisp") => cmd_lisp(&args[1..]),
        Some("mcp") => mcp::serve(),
        Some("demo") => cmd_demo(),
        Some("--help") | Some("-h") | None => {
            print_help();
            0
        }
        Some(other) => {
            eprintln!("неизвестная команда: {other}\n");
            print_help();
            2
        }
    };
    std::process::exit(code);
}

fn print_help() {
    println!(
        "lazr — RAD для Rust, заточенный под ИИ: модель экрана → настоящий egui-код

  new DIR [--title T] [--size 480x320]
  state DIR NAME TYPE [VALUE]        int | float | bool | string
  add DIR KIND [TEXT] [--bind F] [--do \"поле += 1\"] [--container vbox|hbox]
  show DIR                           модель как текст
  gen DIR                             перегенерировать app/
  build DIR                           cargo build --offline в app/
  headless DIR                        исполнить интерфейс один кадр без окна
  live FILE [--headless|--check|--problems]
                                      рантайм: окно читает модель на ходу
  lisp FILE [--to DIR]                S-выражения → модель (и проект, если --to)
  mcp                                 MCP-сервер на stdin/stdout
  demo                                сквозная проверка на живом примере

Виджеты: label, button, checkbox, text_input, slider, separator, vbox, hbox

Код выхода: 0 ок, 1 модель/сборка отказали, 2 ошибка вызова."
    );
}

fn dir_of(opts: &Opts, position: usize) -> Result<PathBuf, i32> {
    let positional = opts.positional();
    match positional.get(position) {
        Some(value) => Ok(PathBuf::from(value)),
        None => {
            eprintln!("нужен каталог проекта");
            Err(2)
        }
    }
}

fn cmd_new(args: &[String]) -> i32 {
    let opts = Opts::parse(args);
    let dir = match dir_of(&opts, 0) {
        Ok(dir) => dir,
        Err(code) => return code,
    };
    let title = opts.flag("title").unwrap_or("Окно");
    let (width, height) = opts
        .flag("size")
        .and_then(|value| value.split_once('x'))
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        .unwrap_or((480, 320));
    match Project::create(&dir, title, width, height).and_then(|project| {
        project.generate()?;
        Ok(project)
    }) {
        Ok(project) => {
            println!("создан проект {}", project.dir.display());
            println!("  модель: {}", project.form_path().display());
            println!("  код:    {}", project.main_path());
            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn cmd_state(args: &[String]) -> i32 {
    let opts = Opts::parse(args);
    let dir = match dir_of(&opts, 0) {
        Ok(dir) => dir,
        Err(code) => return code,
    };
    let positional = opts.positional();
    let (Some(name), Some(ty)) = (positional.get(1), positional.get(2)) else {
        eprintln!("нужно: state DIR NAME TYPE [VALUE]");
        return 2;
    };
    let Some(ty) = FieldType::parse(ty) else {
        eprintln!("тип: int, float, bool, string");
        return 2;
    };
    let value = positional.get(3).cloned().unwrap_or_default();
    let outcome = Project::load(&dir).and_then(|mut project| {
        project.add_state(name, ty, &value)?;
        project.save()?;
        project.generate()?;
        Ok(project)
    });
    match outcome {
        Ok(project) => {
            println!(
                "поле {name} добавлено; виджетов в модели: {}",
                project.count()
            );
            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn cmd_add(args: &[String]) -> i32 {
    let opts = Opts::parse(args);
    let dir = match dir_of(&opts, 0) {
        Ok(dir) => dir,
        Err(code) => return code,
    };
    let positional = opts.positional();
    let Some(kind_text) = positional.get(1) else {
        eprintln!("нужно: add DIR KIND [TEXT]");
        return 2;
    };
    let kind = if kind_text == "custom" {
        match positional.get(2) {
            Some(name) => WidgetKind::Custom(name.clone()),
            None => {
                eprintln!("нужно: add DIR custom ИМЯ (виджет объявлен в rust widget ИМЯ)");
                return 2;
            }
        }
    } else {
        match WidgetKind::parse_kind(kind_text) {
            Some(kind) => kind,
            None => {
                eprintln!(
                    "виджет неизвестен; доступны: {}",
                    WidgetKind::all().join(", ")
                );
                return 2;
            }
        }
    };
    let text = if matches!(kind, WidgetKind::Custom(_)) {
        String::new()
    } else {
        positional.get(2).cloned().unwrap_or_default()
    };
    let bind = opts.flag("bind");
    let effect = opts.flag("do");
    let container = opts.flag("container");
    let number = opts
        .flag("number")
        .and_then(|value| value.parse::<f32>().ok());
    let style = opts
        .flag("style")
        .and_then(|text| model::parse_style_text(text).ok());
    let outcome = Project::load(&dir).and_then(|mut project| {
        project.add_widget_with(kind.clone(), &text, bind, effect, container, number, style)?;
        project.save()?;
        project.generate()?;
        Ok(project)
    });
    match outcome {
        Ok(project) => {
            println!(
                "добавлено: {}; виджетов: {}",
                kind.as_str(),
                project.count()
            );
            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn cmd_show(args: &[String]) -> i32 {
    let opts = Opts::parse(args);
    let dir = match dir_of(&opts, 0) {
        Ok(dir) => dir,
        Err(code) => return code,
    };
    match Project::load(&dir) {
        Ok(project) => {
            print!("{}", project.screen_text());
            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn cmd_gen(args: &[String]) -> i32 {
    let opts = Opts::parse(args);
    let dir = match dir_of(&opts, 0) {
        Ok(dir) => dir,
        Err(code) => return code,
    };
    match Project::load(&dir).and_then(|project| project.generate().map(|_| project)) {
        Ok(project) => {
            println!("сгенерировано: {}", project.main_path());
            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn cmd_build(args: &[String]) -> i32 {
    let opts = Opts::parse(args);
    let dir = match dir_of(&opts, 0) {
        Ok(dir) => dir,
        Err(code) => return code,
    };
    match Project::load(&dir).and_then(|project| project.build().map(|log| (project, log))) {
        Ok((project, log)) => {
            println!("собрано: {}", project.binary().display());
            let last: Vec<&str> = log.lines().rev().take(3).collect();
            for line in last.iter().rev() {
                println!("  {line}");
            }
            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

/// Проверка собранной программы: компиляция, кадр без окна, вызов всех действий.

/// Упаковка в dist/: бинарь, иконка, ярлык .desktop и tar.gz.
fn cmd_package(args: &[String]) -> i32 {
    let opts = Opts::parse(args);
    let dir = match dir_of(&opts, 0) {
        Ok(dir) => dir,
        Err(code) => return code,
    };
    let project = match Project::load(&dir) {
        Ok(project) => project,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let release = opts.has("release");
    match project.package(release) {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn cmd_verify(args: &[String]) -> i32 {
    let opts = Opts::parse(args);
    let dir = match dir_of(&opts, 0) {
        Ok(dir) => dir,
        Err(code) => return code,
    };
    let project = match Project::load(&dir) {
        Ok(project) => project,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    match project.verify() {
        Ok(report) => {
            for line in report {
                println!("{line}");
            }
            println!("итог: проверено");
            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn cmd_headless(args: &[String]) -> i32 {
    let opts = Opts::parse(args);
    let dir = match dir_of(&opts, 0) {
        Ok(dir) => dir,
        Err(code) => return code,
    };
    match Project::load(&dir).and_then(|project| project.headless()) {
        Ok(out) => {
            println!("{out}");
            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

/// DSL: S-выражения → модель → проект. Rust остаётся HAL.

/// Рантайм: окно читает модель и перечитывает её при изменении файла.
fn cmd_live(args: &[String]) -> i32 {
    let opts = Opts::parse(args);
    let file = match opts.positional().first() {
        Some(path) => PathBuf::from(path),
        None => {
            eprintln!("нужно: live FILE (модель form.lazr или .lisp)");
            return 2;
        }
    };
    // .lisp сначала переводим в строчную модель: рантайм читает её.
    let model_path = if file.extension().map(|e| e == "lisp").unwrap_or(false) {
        let source = match std::fs::read_to_string(&file) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("не прочитать {}: {err}", file.display());
                return 2;
            }
        };
        let screen = match lang::read(&source).and_then(|forms| lang::eval(&forms)) {
            Ok(evaluated) => evaluated.screen,
            Err(err) => {
                eprintln!("{}: {}", err.message, err.hint);
                return 1;
            }
        };
        let rendered = model::render(&screen);
        let path = file.with_extension("lazr");
        if let Err(err) = std::fs::write(&path, rendered) {
            eprintln!("не записать {}: {err}", path.display());
            return 1;
        }
        path
    } else {
        file.clone()
    };
    if file.extension().map(|e| e == "lisp").unwrap_or(false) {
        // Держим .lazr в актуальном виде: правишь .lisp — рантайм видит изменения.
        let source = file.clone();
        let rendered_path = model_path.clone();
        std::thread::spawn(move || {
            let mut stamp = std::fs::metadata(&source).and_then(|m| m.modified()).ok();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(250));
                let now = std::fs::metadata(&source).and_then(|m| m.modified()).ok();
                if now == stamp {
                    continue;
                }
                stamp = now;
                let Ok(text) = std::fs::read_to_string(&source) else {
                    continue;
                };
                let Ok(forms) = lang::read(&text) else {
                    continue;
                };
                let Ok(evaluated) = lang::eval(&forms) else {
                    continue;
                };
                let _ = std::fs::write(&rendered_path, model::render(&evaluated.screen));
            }
        });
    }
    let binary = match live::prepare(&model_path) {
        Ok(binary) => binary,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let mut command = std::process::Command::new(&binary);
    command.arg(&model_path);
    if opts.has("headless") {
        command.arg("--headless");
    }
    if opts.has("check") {
        command.arg("--check");
    }
    if opts.has("problems") {
        command.arg("--problems");
    }
    match command.status() {
        Ok(status) if status.success() => 0,
        Ok(status) => {
            eprintln!("рантайм завершился с кодом {status}");
            1
        }
        Err(err) => {
            eprintln!("не запустить {}: {err}", binary.display());
            1
        }
    }
}

fn cmd_lisp(args: &[String]) -> i32 {
    let opts = Opts::parse(args);
    let positional = opts.positional();
    let source = if let Some(path) = positional.first() {
        match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("не прочитать {path}: {err}");
                return 2;
            }
        }
    } else if let Some(inline) = opts.flag("eval") {
        inline.to_string()
    } else {
        eprintln!("нужен файл с S-выражениями или --eval \"(label …)\"");
        return 2;
    };
    let forms = match lang::read(&source) {
        Ok(forms) => forms,
        Err(err) => {
            eprintln!("{}: {}", err.message, err.hint);
            return 1;
        }
    };
    let evaluated = match lang::eval(&forms) {
        Ok(evaluated) => evaluated,
        Err(err) => {
            eprintln!("{}: {}", err.message, err.hint);
            return 1;
        }
    };
    let text = model::render(&evaluated.screen);
    match opts.flag("to") {
        Some(dir) => {
            let dir = PathBuf::from(dir);
            let mut project = match Project::create(
                &dir,
                &evaluated.screen.title,
                evaluated.screen.width,
                evaluated.screen.height,
            ) {
                Ok(project) => project,
                Err(err) => {
                    eprintln!("{err}");
                    return 1;
                }
            };
            project.screen = evaluated.screen;
            if let Err(err) = project.save().and_then(|()| project.generate()) {
                eprintln!("{err}");
                return 1;
            }
            println!("проект из DSL: {}", project.dir.display());
            println!("  модель: {}", project.form_path().display());
            println!("  код:    {}", project.main_path());
            0
        }
        None => {
            print!("{text}");
            0
        }
    }
}

fn cmd_demo() -> i32 {
    let dir = std::env::temp_dir().join(format!(
        "lazr-demo-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    println!("проект: {}", dir.display());

    let mut project = match Project::create(&dir, "Счётчик", 420, 260) {
        Ok(project) => project,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let steps: Vec<(&str, Result<(), String>)> = vec![
        (
            "поле count",
            project.add_state("count", FieldType::Int, "0"),
        ),
        ("поле name", project.add_state("name", FieldType::Str, "")),
        (
            "label с подстановкой",
            project.add_widget(WidgetKind::Label, "Нажатий: {count}", None, None, None),
        ),
        (
            "button с действием",
            project.add_widget(WidgetKind::Button, "Плюс", None, Some("count += 1"), None),
        ),
        (
            "text_input с привязкой",
            project.add_widget(WidgetKind::TextInput, "Имя", Some("name"), None, None),
        ),
        (
            "label с привязкой",
            project.add_widget(WidgetKind::Label, "Привет, {name}", None, None, None),
        ),
    ];
    let mut failed = 0;
    for (name, result) in &steps {
        match result {
            Ok(()) => println!("  ✓ {name}"),
            Err(err) => {
                failed += 1;
                println!("  ✗ {name}: {err}");
            }
        }
    }
    if let Err(err) = project.save() {
        eprintln!("{err}");
        return 1;
    }
    if let Err(err) = project.generate() {
        eprintln!("{err}");
        return 1;
    }
    println!("  ✓ код сгенерирован: {}", project.main_path());

    match project.build() {
        Ok(log) => {
            let tail: Vec<&str> = log.lines().rev().take(2).collect();
            println!(
                "  ✓ собрано: {} ({})",
                project.binary().display(),
                tail.join(" / ")
            );
        }
        Err(err) => {
            println!("  ✗ сборка: {err}");
            return 1;
        }
    }
    match project.headless() {
        Ok(out) => println!("  ✓ headless: {out}"),
        Err(err) => {
            println!("  ✗ headless: {err}");
            return 1;
        }
    }

    println!("\nмодель:\n{}", project.screen_text());
    let _ = std::fs::remove_dir_all(&dir);
    if failed == 0 {
        0
    } else {
        1
    }
}

#[allow(dead_code)]
fn unused(_: Project) {}
