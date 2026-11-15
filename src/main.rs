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

use std::process::Command;

mod codegen;
mod css;
mod expr;
mod lang;
mod live;
mod mcp;
mod model;
mod png;
mod project;
mod runtime;

use std::path::{Path, PathBuf};

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
        Some("fmt") => cmd_fmt(&args[1..]),
        Some("png") => cmd_png(&args[1..]),
        Some("gen") => cmd_gen(&args[1..]),
        Some("build") => cmd_build(&args[1..]),
        Some("run") => cmd_run(&args[1..]),
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
  fmt FILE|DIR [--check]             привести модель к канону (--check: только проверить)
  png FILE [--raw OUT]               прочитать PNG своим декодером (RGBA в --raw)
  gen DIR                             перегенерировать app/
  build DIR
  run DIR                           cargo build --offline в app/
  headless DIR                        исполнить интерфейс один кадр без окна
  live FILE [--headless|--check|--problems|--trace]
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

/// Канонический вид модели: один и тот же смысл — один и тот же текст.
/// `--check` ничего не пишет, но возвращает 1, если файл отформатирован иначе.
/// Свой PNG-декодер «наружу»: можно посмотреть, что он прочитал, и сравнить
/// с любым другим инструментом по сырым RGBA.
fn cmd_png(args: &[String]) -> i32 {
    let opts = Opts::parse(args);
    let Some(path) = opts.positional().first().cloned() else {
        eprintln!("lazr png FILE [--raw OUT]");
        return 2;
    };
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("не прочитать {path}: {err}");
            return 1;
        }
    };
    let image = match png::decode(&bytes) {
        Ok(image) => image,
        Err(err) => {
            eprintln!("{path}: {err}");
            return 1;
        }
    };
    println!(
        "{path}: {}x{}, RGBA {} байт",
        image.width,
        image.height,
        image.rgba.len()
    );
    if let Some(out) = opts.flag("raw").map(str::to_string) {
        match std::fs::write(&out, &image.rgba) {
            Ok(()) => println!("сырые RGBA: {out}"),
            Err(err) => {
                eprintln!("не записать {out}: {err}");
                return 1;
            }
        }
    }
    0
}

fn cmd_fmt(args: &[String]) -> i32 {
    let opts = Opts::parse(args);
    let check = opts.has("check");
    let Some(target) = opts.positional().first().cloned() else {
        eprintln!("lazr fmt FILE|DIR [--check]");
        return 2;
    };
    let path = std::path::Path::new(&target);
    let file = if path.is_dir() {
        path.join("form.lazr")
    } else {
        path.to_path_buf()
    };
    if file.extension().and_then(|ext| ext.to_str()) == Some("lisp") {
        eprintln!(
            "{}: S-выражения — канон сам по себе, форматировать нечего; используйте lazr lisp",
            file.display()
        );
        return 2;
    }
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("не прочитать {}: {err}", file.display());
            return 1;
        }
    };
    let screen = match model::parse(&text) {
        Ok(screen) => screen,
        Err(errors) => {
            let list: Vec<String> = errors.iter().map(|e| e.to_json()).collect();
            eprintln!("{{\"ok\":false,\"errors\":[{}]}}", list.join(","));
            return 1;
        }
    };
    let canonical = model::render(&screen);
    if canonical == text {
        println!("{}: уже канон", file.display());
        return 0;
    }
    if check {
        eprintln!("{}: не канон — запустите lazr fmt", file.display());
        return 1;
    }
    if let Err(err) = std::fs::write(&file, &canonical) {
        eprintln!("не записать {}: {err}", file.display());
        return 1;
    }
    println!("{}: отформатировано", file.display());
    0
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

/// Собрать и запустить: одна команда вместо поиска бинаря в общем target dir.
fn cmd_run(args: &[String]) -> i32 {
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
    if let Err(err) = project.build() {
        eprintln!("{err}");
        return 1;
    }
    let binary = project.binary();
    println!("запуск: {}", binary.display());
    match Command::new(&binary).spawn() {
        Ok(mut child) => match child.wait() {
            Ok(status) => status.code().unwrap_or(0),
            Err(err) => {
                eprintln!("не дождаться приложения: {err}");
                1
            }
        },
        Err(err) => {
            eprintln!("не запустить {}: {err}", binary.display());
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
    // Рантайм понимает оба синтаксиса: и строчную модель, и S-выражения.
    let model_path = file.clone();
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
    if opts.has("trace") {
        command.arg("--trace");
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
            // Таблица стилей живёт рядом с моделью: копируем её в проект,
            // иначе сборка не найдёт файл относительно app/.
            if let Some(sheet) = project.screen.style_sheet.clone() {
                if let Some(source_dir) = std::path::Path::new(
                    positional
                        .first()
                        .map(String::as_str)
                        .unwrap_or("form.lisp"),
                )
                .parent()
                .map(Path::to_path_buf)
                {
                    let from = source_dir.join(&sheet);
                    let to = project.dir.join(&sheet);
                    if from.exists() {
                        if let Some(parent) = to.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        if let Err(err) = std::fs::copy(&from, &to) {
                            eprintln!("не скопировать таблицу стилей {}: {err}", to.display());
                            return 1;
                        }
                    }
                }
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
