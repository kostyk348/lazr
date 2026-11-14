//! `lazr live`: приложение, которое читает модель во время работы.
//!
//! Здесь нет генерации кода под конкретный экран: шаблон один, а экран берётся
//! из файла модели и перечитывается, когда файл меняется. Это и есть обещанная
//! «Lisp-машина» в минимальном виде — Rust остаётся HAL, модель правится на ходу.

/// Полный текст приложения-рантайма (пишется в проект как src/main.rs).
pub const RUNTIME_SRC: &str = r####"//! Сгенерировано lazr live. Правьте модель, а не этот файл.
#![allow(clippy::all)]

use std::collections::HashMap;
use std::time::SystemTime;

use eframe::egui;

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Int(i64),
    Float(f32),
    Bool(bool),
    Text(String),
}

impl Value {
    fn as_f32(&self) -> f32 {
        match self {
            Value::Int(v) => *v as f32,
            Value::Float(v) => *v,
            _ => 0.0,
        }
    }
    fn show(&self) -> String {
        match self {
            Value::Int(v) => v.to_string(),
            Value::Float(v) => format!("{v}"),
            Value::Bool(v) => v.to_string(),
            Value::Text(v) => v.clone(),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Node {
    kind: String,
    text: String,
    bind: Option<String>,
    effect: Option<String>,
    number: Option<f32>,
    color: Option<String>,
    size: Option<f32>,
    bold: bool,
    children: Vec<Node>,
}

struct Model {
    title: String,
    width: u32,
    height: u32,
    fields: Vec<(String, Value)>,
    root: Vec<Node>,
}

fn unquote(token: &str) -> String {
    token.trim().trim_matches('"').to_string()
}

fn parse_style(token: &str, node: &mut Node) {
    let inner = token.trim().trim_start_matches('{').trim_end_matches('}');
    for part in inner.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (key, value) = match part.split_once(':') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => (part, ""),
        };
        match key {
            "color" => node.color = Some(value.to_string()),
            "size" => node.size = value.parse().ok(),
            "bold" => node.bold = true,
            _ => {}
        }
    }
}

fn parse_model(text: &str) -> (Model, Vec<String>) {
    let mut problems: Vec<String> = Vec::new();
    let mut model = Model {
        title: "Окно".into(),
        width: 520,
        height: 380,
        fields: Vec::new(),
        root: Vec::new(),
    };
    // стек: (глубина, путь индексов в дереве)
    let mut stack: Vec<(usize, Vec<usize>)> = vec![(0, Vec::new())];
    let known = [
        "label", "button", "checkbox", "text_input", "slider", "space", "separator", "vbox", "hbox", "grid",
        "scroll", "place", "custom",
    ];
    for (line_no, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') {
            continue;
        }
        let depth = line.len() - line.trim_start().len();
        let words: Vec<String> = split_tokens(trimmed);
        let Some(head) = words.first().cloned() else { continue };
        match head.as_str() {
            "window" => {
                if let Some(title) = words.get(1) {
                    model.title = unquote(title);
                }
                if let Some(size) = words.get(2) {
                    if let Some((w, h)) = size.split_once('x') {
                        model.width = w.parse().unwrap_or(model.width);
                        model.height = h.parse().unwrap_or(model.height);
                    }
                }
                continue;
            }
            "state" => {
                if words.len() < 3 {
                    continue;
                }
                let name = words[1].clone();
                let value = match words[2].as_str() {
                    "int" => Value::Int(words.get(3).and_then(|v| v.parse().ok()).unwrap_or(0)),
                    "float" => {
                        Value::Float(words.get(3).and_then(|v| v.parse().ok()).unwrap_or(0.0))
                    }
                    "bool" => Value::Bool(words.get(3).map(|v| v == "true").unwrap_or(false)),
                    _ => Value::Text(words.get(3).map(|v| unquote(v)).unwrap_or_default()),
                };
                model.fields.push((name, value));
                continue;
            }
            "theme" | "rust" => continue,
            _ => {
                if !known.contains(&head.as_str()) && !problems.iter().any(|p| p.contains(&head)) {
                    problems.push(format!(
                        "строка {}: виджет «{}» рантайм не знает",
                        line_no + 1,
                        head
                    ));
                }
            }
        }
        while stack.len() > 1 && stack.last().map(|(d, _)| *d >= depth).unwrap_or(false) {
            stack.pop();
        }
        let mut node = Node {
            kind: head.clone(),
            ..Default::default()
        };
        let container = matches!(
            head.as_str(),
            "vbox" | "hbox" | "grid" | "scroll" | "place"
        );
        let mut index = 1;
        if !container {
            if let Some(first) = words.get(1) {
                if !first.starts_with('{') && first != "do" && first != "->" {
                    node.text = unquote(first);
                    index = 2;
                }
            }
        }
        let mut i = index;
        while i < words.len() {
            let token = &words[i];
            if token.starts_with('{') {
                parse_style(token, &mut node);
                i += 1;
            } else if token == "do" {
                // Действие кончается там, где начинается стиль {…} или привязка ->.
                let mut end = words.len();
                for (offset, item) in words.iter().enumerate().skip(i + 1) {
                    if item.starts_with('{') || item == "->" {
                        end = offset;
                        break;
                    }
                }
                node.effect = Some(words[i + 1..end].join(" "));
                break;
            } else if token == "->" {
                node.bind = words.get(i + 1).cloned();
                i += 2;
            } else if let Ok(number) = token.parse::<f32>() {
                node.number = Some(number);
                i += 1;
            } else if let Some((a, b)) = token.split_once("..") {
                let _ = (a, b);
                i += 1;
            } else {
                i += 1;
            }
        }
        let path = stack.last().map(|(_, p)| p.clone()).unwrap_or_default();
        let list = node_list(&mut model.root, &path);
        list.push(node);
        if container {
            let mut child_path = path;
            child_path.push(list.len() - 1);
            stack.push((depth, child_path));
        }
    }
    // привязки и подстановки должны указывать на существующие поля
    let names: Vec<String> = model.fields.iter().map(|(name, _)| name.clone()).collect();
    for node in walk(&model.root) {
        if let Some(bind) = &node.bind {
            if !names.contains(bind) {
                problems.push(format!("поле «{bind}» не объявлено в state"));
            }
        }
        let mut rest = node.text.as_str();
        while let Some(open) = rest.find('{') {
            rest = &rest[open + 1..];
            match rest.find('}') {
                Some(close) => {
                    let name = &rest[..close];
                    if !names.contains(&name.to_string()) {
                        problems.push(format!("в тексте подстановка «{{{name}}}» без поля"));
                    }
                    rest = &rest[close + 1..];
                }
                None => break,
            }
        }
    }
    problems.sort();
    problems.dedup();
    (model, problems)
}

/// Токены строки: кавычки и {стиль} держатся вместе.
fn split_tokens(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut depth: usize = 0;
    for ch in line.chars() {
        if quoted {
            if ch == '"' {
                quoted = false;
                out.push(current.clone());
                current.clear();
            } else {
                current.push(ch);
            }
            continue;
        }
        match ch {
            '"' => quoted = true,
            '{' => {
                if depth == 0 && !current.is_empty() {
                    out.push(current.clone());
                    current.clear();
                }
                depth += 1;
                current.push(ch);
            }
            '}' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
                if depth == 0 {
                    out.push(current.clone());
                    current.clear();
                }
            }
            c if c.is_whitespace() && depth == 0 => {
                if !current.is_empty() {
                    out.push(current.clone());
                    current.clear();
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn node_list<'a>(list: &'a mut Vec<Node>, path: &[usize]) -> &'a mut Vec<Node> {
    if path.is_empty() {
        return list;
    }
    let mut node = &mut list[path[0]];
    for index in &path[1..] {
        node = &mut node.children[*index];
    }
    &mut node.children
}

fn text_with(state: &HashMap<String, Value>, text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        rest = &rest[open + 1..];
        match rest.find('}') {
            Some(close) => {
                let name = &rest[..close];
                if let Some(value) = state.get(name) {
                    out.push_str(&value.show());
                } else {
                    out.push('{');
                    out.push_str(name);
                    out.push('}');
                }
                rest = &rest[close + 1..];
            }
            None => {
                out.push('{');
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

fn hex_color(text: &str) -> egui::Color32 {
    let hex = text.trim().trim_start_matches('#');
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0);
    if hex.len() == 6 {
        egui::Color32::from_rgb(byte(0), byte(2), byte(4))
    } else {
        egui::Color32::WHITE
    }
}

/// Упрощённый вычислитель действий: присваивание, +=, -=, "!", if с одной ветвью.
fn run_effect(state: &mut HashMap<String, Value>, effect: &str) {
    for statement in effect.split(';') {
        let statement = statement.trim();
        if statement.is_empty() {
            continue;
        }
        if let Some(condition) = statement.strip_prefix("if ") {
            // if a OP b { ... } else { ... }
            if let Some((head, tail)) = condition.split_once('{') {
                let (cond, then_part) = (head.trim(), tail);
                let (then_part, else_part) = match then_part.split_once("} else {") {
                    Some((t, e)) => (t, e.trim_end_matches('}')),
                    None => (then_part.trim_end_matches('}'), ""),
                };
                if eval_condition(state, cond) {
                    run_effect(state, then_part);
                } else {
                    run_effect(state, else_part);
                }
            }
            continue;
        }
        let (name, kind, value) = if let Some((n, v)) = statement.split_once("+=") {
            (n.trim(), "+=", v.trim())
        } else if let Some((n, v)) = statement.split_once("-=") {
            (n.trim(), "-=", v.trim())
        } else if let Some((n, v)) = statement.split_once('=') {
            (n.trim(), "=", v.trim())
        } else {
            continue;
        };
        let current = state.get(name).cloned().unwrap_or(Value::Int(0));
        let delta: f32 = value.trim_start_matches('!').parse().unwrap_or(0.0);
        let next = match (&current, kind, value) {
            (Value::Bool(flag), "=", "!bool") => Value::Bool(!flag),
            (Value::Text(_), "=", text) => Value::Text(unquote(text)),
            (Value::Bool(flag), "=", "true") => Value::Bool(true),
            (Value::Bool(_), "=", "false") => Value::Bool(false),
            (_, "=", _) if current_is_text(&current) => Value::Text(unquote(value)),
            (Value::Int(v), "+=", _) => Value::Int(*v + delta as i64),
            (Value::Int(v), "-=", _) => Value::Int(*v - delta as i64),
            (Value::Int(_), "=", _) => Value::Int(delta as i64),
            (Value::Float(v), "+=", _) => Value::Float(*v + delta),
            (Value::Float(v), "-=", _) => Value::Float(*v - delta),
            (Value::Float(_), "=", _) => Value::Float(delta),
            _ => current.clone(),
        };
        state.insert(name.to_string(), next);
    }
}

fn current_is_text(value: &Value) -> bool {
    matches!(value, Value::Text(_))
}

fn eval_condition(state: &HashMap<String, Value>, condition: &str) -> bool {
    let ops = [" >= ", " <= ", " == ", " != ", " > ", " < "];
    for op in ops {
        if let Some((left, right)) = condition.split_once(op) {
            let lhs = number_of(state, left.trim());
            let rhs: f32 = right.trim().trim_start_matches('"').trim_end_matches('"').parse().unwrap_or(0.0);
            return match op.trim() {
                ">=" => lhs >= rhs,
                "<=" => lhs <= rhs,
                "==" => (lhs - rhs).abs() < f32::EPSILON,
                "!=" => (lhs - rhs).abs() >= f32::EPSILON,
                ">" => lhs > rhs,
                _ => lhs < rhs,
            };
        }
    }
    false
}

fn number_of(state: &HashMap<String, Value>, name: &str) -> f32 {
    state
        .get(name)
        .map(Value::as_f32)
        .or_else(|| name.parse().ok())
        .unwrap_or(0.0)
}

fn walk<'a>(nodes: &'a [Node]) -> Vec<&'a Node> {
    let mut out = Vec::new();
    for node in nodes {
        out.push(node);
        out.extend(walk(&node.children));
    }
    out
}

fn draw_node(ui: &mut egui::Ui, node: &Node, state: &mut HashMap<String, Value>) {
    let mut rich = |text: String| {
        let mut out = egui::RichText::new(text);
        if let Some(size) = node.size {
            out = out.size(size);
        }
        if node.bold {
            out = out.strong();
        }
        if let Some(color) = &node.color {
            out = out.color(hex_color(color));
        }
        out
    };
    match node.kind.as_str() {
        "label" => {
            ui.label(rich(text_with(state, &node.text)));
        }
        "button" => {
            if ui.button(rich(text_with(state, &node.text))).clicked() {
                if let Some(effect) = &node.effect {
                    run_effect(state, effect);
                }
            }
        }
        "checkbox" => {
            if let Some(field) = &node.bind {
                let mut flag = matches!(state.get(field), Some(Value::Bool(true)));
                if ui.checkbox(&mut flag, rich(text_with(state, &node.text))).changed() {
                    state.insert(field.clone(), Value::Bool(flag));
                }
            }
        }
        "text_input" => {
            if let Some(field) = &node.bind {
                let mut text = match state.get(field) {
                    Some(Value::Text(v)) => v.clone(),
                    Some(other) => other.show(),
                    None => String::new(),
                };
                if ui.text_edit_singleline(&mut text).changed() {
                    state.insert(field.clone(), Value::Text(text));
                }
            }
        }
        "slider" => {
            if let Some(field) = &node.bind {
                match state.get(field).cloned() {
                    Some(Value::Int(mut v)) => {
                        let (a, b) = node.number.map(|n| (0, n as i64)).unwrap_or((0, 100));
                        if ui.add(egui::Slider::new(&mut v, a..=b)).changed() {
                            state.insert(field.clone(), Value::Int(v));
                        }
                    }
                    Some(Value::Float(mut v)) => {
                        let b = node.number.unwrap_or(100.0);
                        if ui.add(egui::Slider::new(&mut v, 0.0..=b)).changed() {
                            state.insert(field.clone(), Value::Float(v));
                        }
                    }
                    _ => {}
                }
            }
        }
        "space" => ui.add_space(node.number.unwrap_or(8.0)),
        "separator" => {
            ui.separator();
        }
        "vbox" => {
            ui.vertical(|ui| {
                for child in &node.children {
                    draw_node(ui, child, state);
                }
            });
        }
        "hbox" => {
            ui.horizontal(|ui| {
                for child in &node.children {
                    draw_node(ui, child, state);
                }
            });
        }
        "grid" => {
            let columns = node.number.unwrap_or(2.0).max(1.0) as usize;
            egui::Grid::new(format!("grid-{:p}", node))
                .num_columns(columns)
                .show(ui, |ui| {
                    for (index, child) in node.children.iter().enumerate() {
                        draw_node(ui, child, state);
                        if (index + 1) % columns == 0 {
                            ui.end_row();
                        }
                    }
                });
        }
        "scroll" => {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for child in &node.children {
                    draw_node(ui, child, state);
                }
            });
        }
        _ => {}
    }
}

struct LiveApp {
    path: String,
    model: Model,
    problems: Vec<String>,
    state: HashMap<String, Value>,
    modified: Option<SystemTime>,
    themed: bool,
}

impl LiveApp {
    fn reload(&mut self) {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(err) => {
                self.problems = vec![format!("не прочитать {}: {err}", self.path)];
                return;
            }
        };
        let (model, problems) = parse_model(&text);
        self.problems = problems;
        // сохраняем значения полей, которые уже были: перезагрузка не должна сбрасывать ввод
        let mut state = HashMap::new();
        for (name, value) in &model.fields {
            state.insert(name.clone(), self.state.get(name).cloned().unwrap_or(value.clone()));
        }
        self.model = model;
        self.state = state;
    }

    fn watch(&mut self) {
        let stamp = std::fs::metadata(&self.path).and_then(|m| m.modified()).ok();
        if stamp != self.modified {
            self.modified = stamp;
            self.reload();
        }
    }
}

impl eframe::App for LiveApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.watch();
        if !self.themed {
            ui.ctx().set_theme(egui::ThemePreference::Dark);
            self.themed = true;
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(400));
        ui.label(
            egui::RichText::new(format!(
                "live: {} — правьте файл, окно перечитает его само",
                self.path
            ))
            .size(11.0)
            .color(egui::Color32::from_rgb(139, 148, 158)),
        );
        if !self.problems.is_empty() {
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(48, 20, 22))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(format!("модель с замечаниями: {}", self.problems.len()))
                            .strong()
                            .color(egui::Color32::from_rgb(229, 72, 77)),
                    );
                    for problem in self.problems.iter().take(4) {
                        ui.label(
                            egui::RichText::new(format!("• {problem}"))
                                .size(12.0)
                                .color(egui::Color32::from_rgb(240, 180, 180)),
                        );
                    }
                });
            ui.add_space(6.0);
        }
        ui.separator();
        for node in self.model.root.clone() {
            draw_node(ui, &node, &mut self.state);
        }
    }
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "form.lazr".to_string());
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let (model, problems) = parse_model(&text);
    let mut state = HashMap::new();
    for (name, value) in &model.fields {
        state.insert(name.clone(), value.clone());
    }
    let width = model.width as f32;
    let height = model.height as f32 + 44.0;
    let title = model.title.clone();
    let mut app = LiveApp {
        path: path.clone(),
        model,
        problems,
        state,
        modified: std::fs::metadata(&path).and_then(|m| m.modified()).ok(),
        themed: false,
    };
    if std::env::args().any(|arg| arg == "--problems") {
        // Машинная проверка модели: то же, что видно в красной плашке.
        for problem in &app.problems {
            println!("{problem}");
        }
        println!("problems: {}", app.problems.len());
        if !app.problems.is_empty() {
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|arg| arg == "--check") {
        // Проверка действий без окна: каждое действие применяется к состоянию,
        // печатается, какие поля изменились. Это и есть «функция работает».
        let mut actions = 0;
        for node in walk(&app.model.root) {
            if node.kind != "button" {
                continue;
            }
            let Some(effect) = &node.effect else { continue };
            let before = app.state.clone();
            run_effect(&mut app.state, effect);
            actions += 1;
            let mut changes: Vec<String> = Vec::new();
            for (name, value) in &app.state {
                match before.get(name) {
                    Some(old) if old == value => {}
                    Some(old) => changes.push(format!("{name}: {} → {}", old.show(), value.show())),
                    None => changes.push(format!("{name}: появилось {}", value.show())),
                }
            }
            if changes.is_empty() {
                println!("  «{}»: без изменений ({effect})", node.text);
            } else {
                println!("  «{}»: {}", node.text, changes.join(", "));
            }
        }
        println!("check ok: действий {actions}");
        return;
    }
    if std::env::args().any(|arg| arg == "--headless") {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            for node in app.model.root.clone() {
                draw_node(ui, &node, &mut app.state);
            }
        });
        output.textures_delta.clear();
        println!("live headless ok: {} фигур", output.shapes.len());
        return;
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([width, height])
            .with_title(title),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "lazr live",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    );
}
"####;
