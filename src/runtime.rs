//! `lazr live`: приложение, которое читает модель во время работы.
//!
//! Здесь нет генерации кода под конкретный экран: шаблон один, а экран берётся
//! из файла модели и перечитывается, когда файл меняется. Это и есть обещанная
//! «Lisp-машина» в минимальном виде — Rust остаётся HAL, модель правится на ходу.

/// Полный текст приложения-рантайма (пишется в проект как src/main.rs).
pub const RUNTIME_SRC: &str = r####"//! Сгенерировано lazr live. Правьте модель, а не этот файл.
#![allow(clippy::all)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

use eframe::egui;

/// --trace: печатать каждое применение обработчика (для проверок и отладки).
static TRACE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Int(i64),
    Float(f32),
    Bool(bool),
    Text(String),
    /// Список строк: то же, что Vec<String> в сгенерированном коде.
    List(Vec<String>),
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
            // в тексте список показывает количество, а не содержимое
            Value::List(items) => items.len().to_string(),
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
    /// Моноширинный текст (терминалы, код).
    mono: bool,
    /// Подсказка при наведении.
    hint: Option<String>,
    /// Абсолютная точка внутри place.
    at: Option<(f32, f32)>,
    /// Размер блока (у place — размер самого места).
    box_size: Option<(f32, f32)>,
    /// Якорь: (справа, снизу).
    anchor: (bool, bool),
    /// Обработчик как форма: исполняется, а не пересказывается строкой.
    handler: Option<Sexp>,
    /// Условие when формой (S-выражения) — строчное лежит в text.
    cond_form: Option<Sexp>,
    children: Vec<Node>,
}

#[derive(Clone, PartialEq)]
struct Theme {
    dark: bool,
    accent: egui::Color32,
    panel: Option<egui::Color32>,
    card: Option<egui::Color32>,
    muted: Option<egui::Color32>,
    font: f32,
    spacing: f32,
    round: u8,
}

impl Default for Theme {
    fn default() -> Theme {
        Theme {
            dark: true,
            accent: egui::Color32::from_rgb(56, 139, 253),
            panel: None,
            card: None,
            muted: None,
            font: 14.0,
            spacing: 8.0,
            round: 8,
        }
    }
}

/// Опции окна в рантайме: те же ключи, что в модели.
#[derive(Clone, PartialEq)]
struct WindowFlags {
    own: bool,
    resizable: bool,
    topmost: bool,
    transparent: bool,
    drag_anywhere: bool,
    center: bool,
    min: Option<(u32, u32)>,
    max: Option<(u32, u32)>,
}

impl WindowFlags {
    fn new() -> Self {
        Self {
            own: false,
            resizable: true,
            topmost: false,
            transparent: false,
            drag_anywhere: false,
            center: false,
            min: None,
            max: None,
        }
    }

    fn set(&mut self, key: &str, value: &str) {
        let flag = matches!(value.trim(), "yes" | "true" | "да" | "on" | "1");
        let size = |text: &str| -> Option<(u32, u32)> {
            let (w, h) = text.trim().trim_matches('"').split_once('x')?;
            Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
        };
        match key {
            "chrome" => self.own = matches!(value.trim(), "own" | "своя"),
            "resizable" => self.resizable = flag,
            "topmost" => self.topmost = flag,
            "center" => self.center = flag,
            "transparent" => self.transparent = flag,
            "drag-anywhere" => self.drag_anywhere = flag,
            "min" | "window-min" => self.min = size(value),
            "max" | "window-max" => self.max = size(value),
            _ => {}
        }
    }
}

struct Model {
    /// Подключённый TTF с иконками.
    icon_font: Option<String>,
    /// Свой TTF для текста интерфейса.
    ui_font: Option<String>,
    theme: Theme,
    title: String,
    width: u32,
    height: u32,
    window: WindowFlags,
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
            "mono" | "моно" => node.mono = true,
            "hint" | "подсказка" => node.hint = Some(value.trim_matches('"').to_string()),
            _ => {}
        }
    }
}

/// Модель в любом из двух синтаксисов: S-выражения или строчные формы.
fn parse_any(text: &str) -> (Model, Vec<String>) {
    // первая содержательная строка решает синтаксис: комментарии и пустые не в счёт
    let first = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with(';'))
        .unwrap_or("");
    if first.starts_with('(') {
        parse_sexpr_model(text)
    } else {
        parse_model(text)
    }
}

fn parse_model(text: &str) -> (Model, Vec<String>) {
    let mut problems: Vec<String> = Vec::new();
    let mut model = Model {
        icon_font: None,
        ui_font: None,
        theme: Theme::default(),
        title: "Окно".into(),
        width: 520,
        height: 380,
        window: WindowFlags::new(),
        fields: Vec::new(),
        root: Vec::new(),
    };
    // стек: (глубина, путь индексов в дереве)
    let mut stack: Vec<(usize, Vec<usize>)> = vec![(0, Vec::new())];
    let known = [
        "label", "button", "checkbox", "text_input", "slider", "space", "separator", "vbox", "hbox", "grid",
        "scroll", "place", "list", "when", "else", "card", "icon", "custom",
    ];
    let mut in_rust = false;
    // глубина фигурных скобок внутри блока rust: внутри бывают свои «}»,
    // поэтому одной строки «}» недостаточно
    let mut rust_depth: i32 = 0;
    let braces = |text: &str| -> i32 {
        text.chars().fold(0, |acc, ch| match ch {
            '{' => acc + 1,
            '}' => acc - 1,
            _ => acc,
        })
    };
    for (line_no, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if in_rust {
            rust_depth += braces(trimmed);
            if rust_depth <= 0 {
                in_rust = false;
            }
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with(';') {
            continue;
        }
        if trimmed.starts_with("rust ") {
            // код на Rust живёт в генераторе, не в рантайме
            in_rust = true;
            rust_depth = braces(trimmed);
            continue;
        }
        let depth = line.len() - line.trim_start().len();
        let words: Vec<String> = split_tokens(trimmed);
        let Some(head) = words.first().cloned() else { continue };
        // в DSL дефис (text-input), в отрисовке подчёркивание (text_input)
        let head = head.replace('-', "_");
        match head.as_str() {
            "window" => {
                if let Some(title) = words.get(1) {
                    model.title = unquote(title);
                }
                let mut rest = 2;
                if let Some(size) = words.get(2) {
                    if let Some((w, h)) = size.split_once('x') {
                        model.width = w.parse().unwrap_or(model.width);
                        model.height = h.parse().unwrap_or(model.height);
                        rest = 3;
                    } else if let (Ok(w), Some(Ok(h))) = (
                        size.parse::<u32>(),
                        words.get(3).map(|token| token.parse::<u32>()),
                    ) {
                        model.width = w;
                        model.height = h;
                        rest = 4;
                    }
                }
                for token in words.iter().skip(rest) {
                    if let Some((key, value)) = token.split_once(':') {
                        model.window.set(key, value);
                    }
                }
                continue;
            }
            // отдельные строки-параметры окна: chrome own / topmost yes / min 400x300
            "chrome" | "resizable" | "topmost" | "center" | "transparent" | "drag-anywhere"
            | "min" | "max" | "window-min" | "window-max" => {
                let value = words.get(1).cloned().unwrap_or_else(|| "yes".to_string());
                let key = match head.as_str() {
                    "window-min" => "min",
                    "window-max" => "max",
                    other => other,
                };
                model.window.set(key, &value);
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
                    "list" => Value::List(
                        words
                            .get(3)
                            .map(|v| {
                                unquote(v)
                                    .split(',')
                                    .map(str::trim)
                                    .filter(|item| !item.is_empty())
                                    .map(str::to_string)
                                    .collect()
                            })
                            .unwrap_or_default(),
                    ),
                    _ => Value::Text(words.get(3).map(|v| unquote(v)).unwrap_or_default()),
                };
                model.fields.push((name, value));
                continue;
            }
            "font-family" | "font_family" | "ui-font" => {
                model.ui_font = words.get(1).map(|path| unquote(path));
                continue;
            }
            "icon-font" | "icon_font" => {
                if let Some(path) = words.get(1) {
                    model.icon_font = Some(unquote(path));
                }
                continue;
            }
            "theme" => {
                for word in words.iter().skip(1) {
                    let (key, value) = match word.split_once(':') {
                        Some((key, value)) => (key, value),
                        None => (word.as_str(), ""),
                    };
                    match key {
                        "dark" => model.theme.dark = true,
                        "light" => model.theme.dark = false,
                        "accent" => model.theme.accent = hex_color(value),
                        "font" => model.theme.font = value.parse().unwrap_or(model.theme.font),
                        "spacing" => {
                            model.theme.spacing = value.parse().unwrap_or(model.theme.spacing)
                        }
                        "round" => model.theme.round = value.parse().unwrap_or(model.theme.round),
                        "panel" => model.theme.panel = Some(hex_color(value)),
                        "card" => model.theme.card = Some(hex_color(value)),
                        "muted" => model.theme.muted = Some(hex_color(value)),
                        _ => {}
                    }
                }
                continue;
            }
            "rust" => continue,
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
        if head == "else" {
            // else — ветка предыдущего when, а не отдельный виджет
            let Some((when_depth, when_path)) = stack.last().cloned() else {
                problems.push(format!("строка {line_no}: else без when"));
                continue;
            };
            if when_depth != depth {
                problems.push(format!("строка {line_no}: else без when"));
                continue;
            }
            let mut parent_path = when_path.clone();
            let Some(index) = parent_path.pop() else {
                problems.push(format!("строка {line_no}: else без when"));
                continue;
            };
            let parent = node_list(&mut model.root, &parent_path);
            let is_when = parent
                .get(index)
                .map(|n| n.kind == "when")
                .unwrap_or(false);
            if !is_when {
                problems.push(format!("строка {line_no}: else без when"));
                continue;
            }
            parent[index].children.push(Node {
                kind: "else".to_string(),
                ..Default::default()
            });
            let mut else_path = when_path;
            else_path.push(parent[index].children.len() - 1);
            stack.push((depth, else_path));
            continue;
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
            "vbox" | "hbox" | "grid" | "scroll" | "place" | "when" | "list" | "card"
        );
        if head == "when" {
            // условие — всё до первого стиля
            let cond: Vec<String> = words
                .iter()
                .skip(1)
                .take_while(|word| !word.starts_with('{'))
                .cloned()
                .collect();
            node.text = cond.join(" ");
        }
        let mut index = 1;
        if !container {
            if let Some(first) = words.get(1) {
                if !first.starts_with('{') && first != "do" && first != "->" {
                    node.text = unquote(first);
                    index = 2;
                }
            }
        }
        if head == "place" {
            if let Some(first) = words.get(1) {
                if let Some((w, h)) = first.split_once('x') {
                    if let (Ok(w), Ok(h)) = (w.parse::<f32>(), h.parse::<f32>()) {
                        node.box_size = Some((w, h));
                        index = 2;
                    }
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
                    if item.starts_with('{')
                        || item == "->"
                        || item == "at"
                        || item == "size"
                        || item == "anchor"
                    {
                        end = offset;
                        break;
                    }
                }
                node.effect = Some(words[i + 1..end].join(" "));
                break;
            } else if token == "->" {
                node.bind = words.get(i + 1).cloned();
                i += 2;
            } else if token == "at" {
                let x = words.get(i + 1).and_then(|v| v.parse::<f32>().ok());
                let y = words.get(i + 2).and_then(|v| v.parse::<f32>().ok());
                if let (Some(x), Some(y)) = (x, y) {
                    node.at = Some((x, y));
                }
                i += 3;
            } else if token == "size" {
                let raw = words.get(i + 1).cloned().unwrap_or_default();
                if let Some((w, h)) = raw.split_once('x') {
                    if let (Ok(w), Ok(h)) = (w.parse::<f32>(), h.parse::<f32>()) {
                        node.box_size = Some((w, h));
                    }
                }
                i += 2;
            } else if token == "anchor" {
                let mut right = false;
                let mut bottom = false;
                let mut step = 1;
                for extra in words.iter().skip(i + 1) {
                    match extra.as_str() {
                        "right" => {
                            right = true;
                            step += 1;
                        }
                        "bottom" => {
                            bottom = true;
                            step += 1;
                        }
                        "left" | "top" | "center" => step += 1,
                        _ => break,
                    }
                }
                node.anchor = (right, bottom);
                i += step;
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
                    // item — переменная шаблона строки списка, а не поле состояния
                    if name != "item" && !names.contains(&name.to_string()) {
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
    // {item} уже лежит в state как строка текущей строки списка
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
        // append items "текст"; remove items 0; clear items
        if let Some(rest) = statement.strip_prefix("append ") {
            let (name, value) = rest.split_once(' ').unwrap_or((rest, ""));
            let raw = value.trim();
            let text = if raw.starts_with('"') {
                unquote(raw)
            } else {
                match state.get(raw) {
                    Some(other) => other.show(),
                    None => raw.to_string(),
                }
            };
            if let Some(Value::List(items)) = state.get_mut(name.trim()) {
                items.push(text);
            }
            continue;
        }
        if let Some(rest) = statement.strip_prefix("remove ") {
            let (name, index) = rest.split_once(' ').unwrap_or((rest, ""));
            if index.trim().is_empty() {
                // «remove items» без индекса — удалить текущую строку шаблона
                let row = match state.get("__row") {
                    Some(Value::Int(row)) => *row,
                    _ => 0,
                };
                state.insert("__remove_row".to_string(), Value::Int(row));
                continue;
            }
            let index = eval_expr(state, index.trim()) as usize;
            if let Some(Value::List(items)) = state.get_mut(name.trim()) {
                if index < items.len() {
                    items.remove(index);
                }
            }
            continue;
        }
        if let Some(rest) = statement.strip_prefix("clear ") {
            if let Some(Value::List(items)) = state.get_mut(rest.trim()) {
                items.clear();
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
        let delta = eval_expr(state, value);
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

/// Арифметика в действиях: `count + 2`, `limit * 2`, число или имя поля.
fn eval_expr(state: &HashMap<String, Value>, text: &str) -> f32 {
    let text = text.trim();
    if text.is_empty() {
        return 0.0;
    }
    // сложение/вычитание — ищем последний оператор, чтобы разобрать «a + b - c»
    let mut split_at = None;
    for (index, ch) in text.char_indices() {
        if (ch == '+' || ch == '-') && index > 0 {
            split_at = Some(index);
        }
    }
    if let Some(index) = split_at {
        let left = eval_expr(state, &text[..index]);
        let right = eval_expr(state, &text[index + 1..]);
        return if text.as_bytes()[index] == b'+' {
            left + right
        } else {
            left - right
        };
    }
    // умножение/деление
    if let Some((index, _)) = text
        .char_indices()
        .skip(1)
        .find(|(_, ch)| *ch == '*' || *ch == '/')
    {
        let left = eval_expr(state, &text[..index]);
        let right = eval_expr(state, &text[index + 1..]);
        return match text.as_bytes()[index] {
            b'*' => left * right,
            _ if right.abs() > f32::EPSILON => left / right,
            _ => 0.0,
        };
    }
    let token = text.trim().trim_matches('"');
    token.parse().unwrap_or_else(|_| number_of(state, token))
}

fn current_is_text(value: &Value) -> bool {
    matches!(value, Value::Text(_))
}

/// Текст значения: литерал в кавычках или строковое поле.
fn text_of(state: &HashMap<String, Value>, text: &str) -> Option<String> {
    let text = text.trim();
    if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
        return Some(text[1..text.len() - 1].to_string());
    }
    match state.get(text) {
        Some(Value::Text(value)) => Some(value.clone()),
        _ => None,
    }
}

fn eval_condition(state: &HashMap<String, Value>, condition: &str) -> bool {
    let ops = [" >= ", " <= ", " == ", " != ", " > ", " < "];
    for op in ops {
        if let Some((left, right)) = condition.split_once(op) {
            // текст сравниваем текстом: иначе "42" == "42" превращалось в 0 == 42
            if matches!(op.trim(), "==" | "!=") {
                if let (Some(a), Some(b)) = (
                    text_of(state, left.trim()),
                    text_of(state, right.trim()),
                ) {
                    return if op.trim() == "==" { a == b } else { a != b };
                }
            }
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

// --- S-выражения: модель исполняется, а не пересказывается строкой ----------------

#[derive(Clone, Debug, PartialEq)]
enum Sexp {
    Str(String),
    Sym(String),
    Num(f64),
    List(Vec<Sexp>),
}

impl Sexp {
    fn text(&self) -> Option<String> {
        match self {
            Sexp::Str(text) | Sexp::Sym(text) => Some(text.clone()),
            Sexp::Num(value) => Some(format!("{value}")),
            Sexp::List(_) => None,
        }
    }
    fn num(&self) -> Option<f64> {
        match self {
            Sexp::Num(value) => Some(*value),
            Sexp::Sym(text) => text.parse().ok(),
            _ => None,
        }
    }
}

fn read_sexps(source: &str) -> Result<Vec<Sexp>, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut index = 0;
    let mut forms = Vec::new();
    while let Some(form) = read_form(&chars, &mut index)? {
        forms.push(form);
    }
    Ok(forms)
}

fn read_form(chars: &[char], index: &mut usize) -> Result<Option<Sexp>, String> {
    while *index < chars.len() && chars[*index].is_whitespace() {
        *index += 1;
    }
    if *index >= chars.len() {
        return Ok(None);
    }
    match chars[*index] {
        ';' => {
            while *index < chars.len() && chars[*index] != '\n' {
                *index += 1;
            }
            read_form(chars, index)
        }
        '(' => {
            *index += 1;
            let mut items = Vec::new();
            loop {
                while *index < chars.len() && chars[*index].is_whitespace() {
                    *index += 1;
                }
                if *index >= chars.len() {
                    return Err("скобка не закрыта".to_string());
                }
                if chars[*index] == ')' {
                    *index += 1;
                    return Ok(Some(Sexp::List(items)));
                }
                match read_form(chars, index)? {
                    Some(form) => items.push(form),
                    None => return Err("неожиданный конец формы".to_string()),
                }
            }
        }
        ')' => Err("лишняя «)»".to_string()),
        '"' if chars.get(*index + 1) == Some(&'"') && chars.get(*index + 2) == Some(&'"') => {
            // сырая строка """…""" — в ней живёт Rust-код, модель его не трогает
            *index += 3;
            let mut text = String::new();
            loop {
                if *index + 2 >= chars.len() {
                    return Err("сырая строка не закрыта".to_string());
                }
                if chars[*index] == '"' && chars[*index + 1] == '"' && chars[*index + 2] == '"' {
                    *index += 3;
                    break;
                }
                text.push(chars[*index]);
                *index += 1;
            }
            Ok(Some(Sexp::Str(text)))
        }
        '"' => {
            *index += 1;
            let mut text = String::new();
            while *index < chars.len() && chars[*index] != '"' {
                if chars[*index] == '\\' && *index + 1 < chars.len() {
                    *index += 1;
                    text.push(match chars[*index] {
                        'n' => '\n',
                        't' => '\t',
                        other => other,
                    });
                } else {
                    text.push(chars[*index]);
                }
                *index += 1;
            }
            if *index >= chars.len() {
                return Err("строка не закрыта".to_string());
            }
            *index += 1;
            Ok(Some(Sexp::Str(text)))
        }
        _ => {
            let start = *index;
            while *index < chars.len()
                && !chars[*index].is_whitespace()
                && chars[*index] != '('
                && chars[*index] != ')'
            {
                *index += 1;
            }
            let word: String = chars[start..*index].iter().collect();
            Ok(Some(match word.parse::<f64>() {
                Ok(value) => Sexp::Num(value),
                Err(_) => Sexp::Sym(word),
            }))
        }
    }
}

/// Модель из S-выражений: формы верхнего уровня → дерево и обработчики-формы.
fn parse_sexpr_model(source: &str) -> (Model, Vec<String>) {
    let mut problems = Vec::new();
    let mut model = Model {
        icon_font: None,
        ui_font: None,
        theme: Theme::default(),
        title: "Окно".into(),
        width: 520,
        height: 380,
        window: WindowFlags::new(),
        fields: Vec::new(),
        root: Vec::new(),
    };
    let forms = match read_sexps(source) {
        Ok(forms) => forms,
        Err(err) => {
            problems.push(format!("не прочитать S-выражения: {err}"));
            return (model, problems);
        }
    };
    let mut counter = 0usize;
    for form in forms {
        let Sexp::List(items) = &form else { continue };
        let head = items.first().and_then(Sexp::text).unwrap_or_default();
        let args = &items[1..];
        match head.as_str() {
            "window" => {
                if let Some(title) = args.first().and_then(Sexp::text) {
                    model.title = title;
                }
                if let Some(width) = args.get(1).and_then(Sexp::num) {
                    model.width = width as u32;
                }
                if let Some(height) = args.get(2).and_then(Sexp::num) {
                    model.height = height as u32;
                }
                let mut i = 3;
                while i + 1 < args.len() {
                    let key = args[i].text().unwrap_or_default();
                    let key = key.trim_start_matches(':').to_string();
                    let value = args[i + 1].text().unwrap_or_default();
                    model.window.set(&key, &value);
                    i += 2;
                }
            }
            "state" => {
                let name = args.first().and_then(Sexp::text).unwrap_or_default();
                let kind = args.get(1).and_then(Sexp::text).unwrap_or_default();
                let value = match kind.as_str() {
                    "int" => Value::Int(args.get(2).and_then(Sexp::num).unwrap_or(0.0) as i64),
                    "float" => Value::Float(args.get(2).and_then(Sexp::num).unwrap_or(0.0) as f32),
                    "bool" => Value::Bool(
                        args.get(2).and_then(Sexp::text).as_deref() == Some("true"),
                    ),
                    "list" => Value::List(
                        args.get(2)
                            .and_then(Sexp::text)
                            .map(|text| {
                                text.split(',')
                                    .map(str::trim)
                                    .filter(|item| !item.is_empty())
                                    .map(str::to_string)
                                    .collect()
                            })
                            .unwrap_or_default(),
                    ),
                    _ => Value::Text(args.get(2).and_then(Sexp::text).unwrap_or_default()),
                };
                model.fields.push((name, value));
            }
            "font-family" | "font_family" | "ui-font" => {
                model.ui_font = args.first().and_then(Sexp::text);
                continue;
            }
            "icon-font" | "icon_font" => {
                if let Some(path) = args.first().and_then(Sexp::text) {
                    model.icon_font = Some(path);
                }
                continue;
            }
            "theme" => {
                for item in args {
                    match item {
                        Sexp::Sym(word) if word == "dark" => model.theme.dark = true,
                        Sexp::Sym(word) if word == "light" => model.theme.dark = false,
                        Sexp::Sym(word) if word.starts_with(':') => {}
                        Sexp::Sym(word) => {
                            let _ = word;
                        }
                        _ => {}
                    }
                }
                // пары :ключ значение
                let mut index = 0;
                while index < args.len() {
                    if let Sexp::Sym(key) = &args[index] {
                        if let Some(name) = key.strip_prefix(':') {
                            let value = args.get(index + 1);
                            match name {
                                "accent" => {
                                    if let Some(text) = value.and_then(Sexp::text) {
                                        model.theme.accent = hex_color(&text);
                                    }
                                }
                                "font" => {
                                    if let Some(num) = value.and_then(Sexp::num) {
                                        model.theme.font = num as f32;
                                    }
                                }
                                "spacing" => {
                                    if let Some(num) = value.and_then(Sexp::num) {
                                        model.theme.spacing = num as f32;
                                    }
                                }
                                "round" => {
                                    if let Some(num) = value.and_then(Sexp::num) {
                                        model.theme.round = num as u8;
                                    }
                                }
                                _ => {}
                            }
                            index += 2;
                            continue;
                        }
                    }
                    index += 1;
                }
            }
            "rust-field" | "rust-init" | "rust-widget" | "rust-fn" | "rust-item" => {}
            _ => match build_node(&head, args, &mut counter, &mut problems) {
                Ok(Some(node)) => model.root.push(node),
                Ok(None) => {}
                Err(err) => problems.push(err),
            },
        }
    }
    let names: Vec<String> = model.fields.iter().map(|(name, _)| name.clone()).collect();
    for node in walk(&model.root) {
        if let Some(bind) = &node.bind {
            if !names.contains(bind) {
                problems.push(format!("поле «{bind}» не объявлено в state"));
            }
        }
    }
    problems.sort();
    problems.dedup();
    (model, problems)
}

fn build_node(
    head: &str,
    args: &[Sexp],
    counter: &mut usize,
    problems: &mut Vec<String>,
) -> Result<Option<Node>, String> {
    // в DSL дефис, в отрисовке подчёркивание: text-input → text_input
    let kind = head.replace('-', "_");
    let mut node = Node {
        kind: kind.clone(),
        ..Default::default()
    };
    let container = matches!(
        head,
        "vbox" | "hbox" | "grid" | "scroll" | "place" | "when" | "else" | "list" | "card"
    );
    let mut index = 0;
    if head == "when" {
        match args.first() {
            Some(Sexp::List(_)) => {
                node.cond_form = args.first().cloned();
                index = 1;
            }
            Some(Sexp::Sym(word)) => {
                node.text = word.clone();
                index = 1;
            }
            Some(other) => {
                node.text = other.text().unwrap_or_default();
                index = 1;
            }
            None => {}
        }
    }
    if !container {
        if let Some(first) = args.first() {
            match first {
                Sexp::List(_) => {}
                other => {
                    if let Some(text) = other.text() {
                        node.text = text;
                        index = 1;
                    }
                }
            }
        }
    }
    if head == "place" {
        if let (Some(width), Some(height)) = (
            args.first().and_then(Sexp::num),
            args.get(1).and_then(Sexp::num),
        ) {
            node.box_size = Some((width as f32, height as f32));
            index = 2;
        }
    }
    while index < args.len() {
        match &args[index] {
            Sexp::List(items) => {
                let child_head = items.first().and_then(Sexp::text).unwrap_or_default();
                if child_head == "else" {
                    // (else …) внутри when — вторая ветка
                    let mut branch = Node {
                        kind: "else".to_string(),
                        ..Default::default()
                    };
                    for form in &items[1..] {
                        if let Sexp::List(inner) = form {
                            let head = inner.first().and_then(Sexp::text).unwrap_or_default();
                            match build_node(&head, &inner[1..], counter, problems) {
                                Ok(Some(child)) => branch.children.push(child),
                                Ok(None) => {}
                                Err(err) => problems.push(err),
                            }
                        }
                    }
                    node.children.push(branch);
                    index += 1;
                    continue;
                }
                if matches!(
                    child_head.as_str(),
                    "set!" | "if" | "do" | "append" | "remove" | "clear"
                ) {
                    node.handler = Some(Sexp::List(items.clone()));
                } else {
                    match build_node(&child_head, &items[1..], counter, problems) {
                        Ok(Some(child)) => node.children.push(child),
                        Ok(None) => {}
                        Err(err) => problems.push(err),
                    }
                }
                index += 1;
            }
            Sexp::Sym(key) if key.starts_with(':') => {
                let name = key.trim_start_matches(':');
                let value = args.get(index + 1).cloned();
                match name {
                    "bind" => node.bind = value.and_then(|v| v.text()),
                    "size" => node.size = value.and_then(|v| v.num()).map(|v| v as f32),
                    "bold" => node.bold = true,
                    "mono" => node.mono = true,
                    "hint" => node.hint = value.and_then(|v| v.text()),
                    "color" => node.color = value.and_then(|v| v.text()),
                    "at" => {
                        if let (Some(x), Some(y)) = (
                            value.as_ref().and_then(Sexp::num),
                            args.get(index + 2).and_then(Sexp::num),
                        ) {
                            node.at = Some((x as f32, y as f32));
                            index += 1;
                        }
                    }
                    "box" => {
                        if let (Some(w), Some(h)) = (
                            value.as_ref().and_then(Sexp::num),
                            args.get(index + 2).and_then(Sexp::num),
                        ) {
                            node.box_size = Some((w as f32, h as f32));
                            index += 1;
                        }
                    }
                    "anchor" => {
                        let text = value.and_then(|v| v.text()).unwrap_or_default();
                        node.anchor = (text.contains("right"), text.contains("bottom"));
                    }
                    "range" => {}
                    _ => {}
                }
                index += 2;
            }
            Sexp::Sym(word) if word == "at" => {
                if let (Some(x), Some(y)) = (
                    args.get(index + 1).and_then(Sexp::num),
                    args.get(index + 2).and_then(Sexp::num),
                ) {
                    node.at = Some((x as f32, y as f32));
                    index += 3;
                } else {
                    index += 1;
                }
            }
            Sexp::Sym(word) if word == "size" => {
                if let Some(text) = args.get(index + 1).and_then(Sexp::text) {
                    if let Some((width, height)) = text.split_once('x') {
                        if let (Ok(width), Ok(height)) = (width.parse(), height.parse()) {
                            node.box_size = Some((width, height));
                        }
                    }
                }
                index += 2;
            }
            Sexp::Sym(word) if word == "anchor" => {
                let mut right = false;
                let mut bottom = false;
                let mut step = 1;
                for extra in args.iter().skip(index + 1) {
                    match extra.text().as_deref() {
                        Some("right") => {
                            right = true;
                            step += 1;
                        }
                        Some("bottom") => {
                            bottom = true;
                            step += 1;
                        }
                        Some("left") | Some("top") | Some("center") => step += 1,
                        _ => break,
                    }
                }
                node.anchor = (right, bottom);
                index += step;
            }
            Sexp::Sym(word) if word == "do" => {
                match args.get(index + 1) {
                    Some(Sexp::List(items)) => {
                        node.handler = Some(Sexp::List(items.clone()));
                    }
                    Some(Sexp::Str(text)) => node.effect = Some(text.clone()),
                    _ => {}
                }
                index += 2;
            }
            Sexp::Num(value) => {
                node.number = Some(*value as f32);
                index += 1;
            }
            Sexp::Sym(_) => index += 1,
            _ => index += 1,
        }
    }
    if !matches!(
        kind.as_str(),
        "label" | "button" | "checkbox" | "text_input" | "slider" | "space" | "separator" | "vbox"
            | "hbox" | "grid" | "scroll" | "place" | "list" | "when" | "else" | "card" | "icon"
            | "custom"
    ) {
        return Err(format!("форма «{head}» рантайму неизвестна"));
    }
    Ok(Some(node))
}

/// Исполнение обработчика как формы: (set! x (+ x 1)), (if ...), (do ...).
fn eval_handler(state: &mut HashMap<String, Value>, form: &Sexp) {
    let Sexp::List(items) = form else { return };
    let head = items.first().and_then(Sexp::text).unwrap_or_default();
    match head.as_str() {
        "do" => {
            for item in &items[1..] {
                eval_handler(state, item);
            }
        }
        "set!" => {
            let name = items.get(1).and_then(Sexp::text).unwrap_or_default();
            let Some(value) = items.get(2) else { return };
            let current = state.get(&name).cloned().unwrap_or(Value::Int(0));
            let next = match &current {
                // список меняется только через append/remove/clear
                Value::List(_) => current.clone(),
                Value::Int(_) => Value::Int(eval_expr_sexp(state, value) as i64),
                Value::Float(_) => Value::Float(eval_expr_sexp(state, value) as f32),
                Value::Bool(_) => Value::Bool(eval_expr_sexp(state, value) != 0.0),
                Value::Text(_) => match value {
                    Sexp::Str(text) => Value::Text(text.clone()),
                    other => Value::Text(other.text().unwrap_or_default()),
                },
            };
            state.insert(name, next);
        }
        "append" | "remove" | "clear" => {
            let name = items.get(1).and_then(Sexp::text).unwrap_or_default();
            let Some(Value::List(_)) = state.get(&name) else {
                return;
            };
            match head.as_str() {
                "clear" => {
                    if let Some(Value::List(list)) = state.get_mut(&name) {
                        list.clear();
                    }
                }
                "append" => {
                    let text = match items.get(2) {
                        Some(Sexp::Str(text)) => text.clone(),
                        Some(Sexp::Sym(word)) => match state.get(word) {
                            Some(Value::Text(text)) => text.clone(),
                            Some(other) => other.show(),
                            None => word.clone(),
                        },
                        Some(other) => other.text().unwrap_or_default(),
                        None => String::new(),
                    };
                    if let Some(Value::List(list)) = state.get_mut(&name) {
                        list.push(text);
                    }
                }
                _ => {
                    if items.get(2).is_none() {
                        let row = match state.get("__row") {
                            Some(Value::Int(row)) => *row,
                            _ => 0,
                        };
                        state.insert("__remove_row".to_string(), Value::Int(row));
                        return;
                    }
                    let index = items.get(2).map(|v| eval_expr_sexp(state, v)).unwrap_or(0.0) as usize;
                    if let Some(Value::List(list)) = state.get_mut(&name) {
                        if index < list.len() {
                            list.remove(index);
                        }
                    }
                }
            }
        }
        "if" => {
            let condition = items.get(1);
            let then_form = items.get(2);
            let else_form = items.get(3);
            let truth = condition.map(|c| eval_condition_sexp(state, c)).unwrap_or(false);
            if truth {
                if let Some(form) = then_form {
                    eval_handler(state, form);
                }
            } else if let Some(form) = else_form {
                eval_handler(state, form);
            }
        }
        _ => {}
    }
}

fn eval_condition_sexp(state: &HashMap<String, Value>, form: &Sexp) -> bool {
    let Sexp::List(items) = form else {
        // (when agree …) — условие одним символом
        return match form {
            Sexp::Sym(name) if name == "true" => true,
            Sexp::Sym(name) => matches!(state.get(name), Some(Value::Bool(true))),
            _ => false,
        };
    };
    let op = items.first().and_then(Sexp::text).unwrap_or_default();
    if matches!(op.as_str(), "==" | "!=") {
        let text_of_sexp = |value: Option<&Sexp>| -> Option<String> {
            match value {
                Some(Sexp::Str(text)) => Some(text.clone()),
                Some(Sexp::Sym(name)) => match state.get(name) {
                    Some(Value::Text(text)) => Some(text.clone()),
                    _ => None,
                },
                _ => None,
            }
        };
        if let (Some(a), Some(b)) = (text_of_sexp(items.get(1)), text_of_sexp(items.get(2))) {
            return if op == "==" { a == b } else { a != b };
        }
    }
    let left = items.get(1).map(|v| eval_expr_sexp(state, v)).unwrap_or(0.0);
    let right = items.get(2).map(|v| eval_expr_sexp(state, v)).unwrap_or(0.0);
    match op.as_str() {
        ">=" => left >= right,
        "<=" => left <= right,
        ">" => left > right,
        "<" => left < right,
        "==" => (left - right).abs() < f32::EPSILON,
        "!=" => (left - right).abs() >= f32::EPSILON,
        _ => false,
    }
}

fn eval_expr_sexp(state: &HashMap<String, Value>, form: &Sexp) -> f32 {
    match form {
        Sexp::Num(value) => *value as f32,
        Sexp::Str(text) => text.parse().unwrap_or(0.0),
        Sexp::Sym(name) => state.get(name).map(Value::as_f32).unwrap_or(0.0),
        Sexp::List(items) => {
            let op = items.first().and_then(Sexp::text).unwrap_or_default();
            let left = items.get(1).map(|v| eval_expr_sexp(state, v)).unwrap_or(0.0);
            let right = items.get(2).map(|v| eval_expr_sexp(state, v)).unwrap_or(0.0);
            match op.as_str() {
                "+" => left + right,
                "-" => left - right,
                "*" => left * right,
                "/" if right.abs() > f32::EPSILON => left / right,
                _ => left,
            }
        }
    }
}

/// Шрифт иконок из модели: семейство «icons».
fn apply_icon_font(ctx: &egui::Context, path: &str) {
    match std::fs::read(path) {
        Ok(bytes) => ctx.add_font(egui::FontInsert::new(
            "icons",
            egui::FontData::from_owned(bytes),
            vec![egui::InsertFontFamily {
                family: egui::FontFamily::Name("icons".into()),
                priority: egui::FontPriority::Highest,
            }],
        )),
        Err(err) => eprintln!("иконки: не прочитать {path}: {err}"),
    }
}

fn apply_ui_font(ctx: &egui::Context, path: &str) {
    // Свой шрифт текста — первым в proportional, встроенный остаётся запасным.
    match std::fs::read(path) {
        Ok(bytes) => ctx.add_font(egui::FontInsert::new(
            "ui",
            egui::FontData::from_owned(bytes),
            vec![egui::InsertFontFamily {
                family: egui::FontFamily::Proportional,
                priority: egui::FontPriority::Highest,
            }],
        )),
        Err(err) => eprintln!("шрифт: не прочитать {path}: {err}"),
    }
}

fn apply_theme(ctx: &egui::Context, theme: &Theme) {
    ctx.set_theme(if theme.dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    });
    let accent = theme.accent;
    let on_accent = if accent.r() as u32 + accent.g() as u32 + accent.b() as u32 > 420 {
        egui::Color32::from_rgb(16, 18, 24)
    } else {
        egui::Color32::WHITE
    };
    let dark = theme.dark;
    let panel = theme.panel;
    let card = theme.card;
    let muted = theme.muted;
    let round = egui::CornerRadius::same(theme.round);
    let spacing = theme.spacing;
    let font = theme.font;
    ctx.all_styles_mut(move |style| {
        style.visuals = if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        if let Some(panel) = panel {
            style.visuals.panel_fill = panel;
            style.visuals.window_fill = panel;
        }
        if let Some(card) = card {
            style.visuals.faint_bg_color = card;
        }
        if let Some(muted) = muted {
            style.visuals.weak_text_color = Some(muted);
            style.visuals.widgets.noninteractive.bg_stroke =
                egui::Stroke::new(1.0, muted.gamma_multiply(0.7));
        }
        style.visuals.selection.bg_fill = accent;
        style.visuals.selection.stroke = egui::Stroke::new(1.0, on_accent);
        style.visuals.hyperlink_color = accent;
        style.visuals.widgets.noninteractive.corner_radius = round;
        style.visuals.widgets.inactive.corner_radius = round;
        style.visuals.widgets.hovered.corner_radius = round;
        style.visuals.widgets.active.corner_radius = round;
        style.visuals.widgets.open.corner_radius = round;
        style.spacing.item_spacing = egui::vec2(spacing, spacing);
        style.spacing.button_padding = egui::vec2(spacing, spacing * 0.5);
        for (text_style, size) in [
            (egui::TextStyle::Body, font),
            (egui::TextStyle::Button, font),
            (egui::TextStyle::Heading, font * 1.4),
            (egui::TextStyle::Small, font * 0.8),
        ] {
            style
                .text_styles
                .insert(text_style, egui::FontId::proportional(size));
        }
    });
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
        if node.mono {
            out = out.monospace();
        }
        if let Some(color) = &node.color {
            out = out.color(hex_color(color));
        }
        out
    };
    match node.kind.as_str() {
        "label" => {
            let response = ui.label(rich(text_with(state, &node.text)));
            if let Some(hint) = &node.hint {
                response.on_hover_text(hint);
            }
        }
        "button" => {
            let label = ui.button(rich(text_with(state, &node.text)));
            if let Some(hint) = &node.hint {
                label.clone().on_hover_text(hint);
            }
            if label.clicked() {
                let before = state.clone();
                if let Some(handler) = &node.handler {
                    eval_handler(state, handler);
                } else if let Some(effect) = &node.effect {
                    run_effect(state, effect);
                }
                if TRACE.load(Ordering::Relaxed) {
                    let mut changes: Vec<String> = Vec::new();
                    for (name, value) in state.iter() {
                        // служебные ключи шаблона строки не показываем
                        if name.starts_with("__") || name == "item" {
                            continue;
                        }
                        if before.get(name) != Some(value) {
                            changes.push(format!(
                                "{name}: {} → {}",
                                before.get(name).map(Value::show).unwrap_or_default(),
                                value.show()
                            ));
                        }
                    }
                    eprintln!(
                        "click «{}»: {}",
                        node.text,
                        if changes.is_empty() {
                            "без изменений".to_string()
                        } else {
                            changes.join(", ")
                        }
                    );
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
        "place" => {
            let (cw, ch) = node.box_size.unwrap_or((600.0, 400.0));
            let origin = ui.available_rect_before_wrap().min;
            let canvas = egui::Rect::from_min_size(origin, egui::vec2(cw, ch));
            for child in &node.children {
                let (mut x, mut y) = child.at.unwrap_or((0.0, 0.0));
                let (bw, bh) = child
                    .box_size
                    .unwrap_or((cw, if child.at.is_some() { 24.0 } else { ch }));
                if child.anchor.0 {
                    x = cw - bw - x;
                }
                if child.anchor.1 {
                    y = ch - bh - y;
                }
                let rect = egui::Rect::from_min_size(
                    canvas.min + egui::vec2(x, y),
                    egui::vec2(bw, bh),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    draw_node(ui, child, state);
                });
            }
            ui.advance_cursor_after_rect(canvas);
        }
        "when" => {
            let truth = match &node.cond_form {
                Some(form) => eval_condition_sexp(state, form),
                None => eval_condition(state, &node.text),
            };
            let mut branch: Option<&Node> = None;
            for child in &node.children {
                if child.kind == "else" {
                    branch = Some(child);
                } else if truth {
                    draw_node(ui, child, state);
                }
            }
            if !truth {
                if let Some(other) = branch {
                    for child in &other.children {
                        draw_node(ui, child, state);
                    }
                }
            }
        }
        "list" => {
            // строки с удалением; идём по копии, чтобы не спорить с заимствованием
            let field = node.bind.clone().unwrap_or_default();
            let rows = match state.get(&field) {
                Some(Value::List(items)) => items.clone(),
                _ => Vec::new(),
            };
            if !rows.is_empty() && !node.children.is_empty() {
                // шаблон строки: {item} — текущая строка, «remove items» — эта строка
                let mut remove_row: Option<usize> = None;
                for (index, item) in rows.iter().enumerate() {
                    state.insert("item".to_string(), Value::Text(item.clone()));
                    state.insert("__row".to_string(), Value::Int(index as i64));
                    ui.horizontal(|ui| {
                        for child in &node.children {
                            draw_node(ui, child, state);
                        }
                    });
                    if let Some(Value::Int(_)) = state.get("__row") {
                        // кнопка могла попросить удалить строку
                        if let Some(request) = state.get("__remove_row").cloned() {
                            if let Value::Int(row) = request {
                                if row as usize == index {
                                    remove_row = Some(index);
                                }
                            }
                        }
                    }
                }
                state.remove("item");
                state.remove("__row");
                state.remove("__remove_row");
                if let Some(row) = remove_row {
                    if let Some(Value::List(items)) = state.get_mut(&field) {
                        if row < items.len() {
                            items.remove(row);
                        }
                    }
                }
                return;
            }
            if rows.is_empty() {
                let hint = if node.text.is_empty() {
                    "пусто"
                } else {
                    node.text.as_str()
                };
                ui.label(
                    egui::RichText::new(hint)
                        .size(12.0)
                        .color(egui::Color32::from_gray(160)),
                );
            } else {
                let mut remove_at = None;
                for (i, item) in rows.iter().enumerate() {
                    ui.horizontal(|ui| {
                        if ui.small_button("✖").clicked() {
                            remove_at = Some(i);
                        }
                        ui.label(item);
                    });
                }
                if let Some(i) = remove_at {
                    if let Some(Value::List(items)) = state.get_mut(&field) {
                        if i < items.len() {
                            items.remove(i);
                        }
                    }
                }
            }
        }
        "card" => {
            egui::Frame::new()
                .fill(ui.visuals().faint_bg_color)
                .stroke(egui::Stroke::new(
                    1.0,
                    ui.visuals().widgets.noninteractive.bg_stroke.color,
                ))
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(node.number.unwrap_or(12.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    for child in &node.children {
                        draw_node(ui, child, state);
                    }
                });
        }
        "icon" => {
            // глиф из подключённого шрифта иконок
            let size = node.size.unwrap_or(18.0);
            let mut rich = egui::RichText::new(node.text.clone())
                .family(egui::FontFamily::Name("icons".into()))
                .size(size);
            if let Some(color) = &node.color {
                rich = rich.color(hex_color(color));
            }
            ui.label(rich);
        }
        "custom" => {
            // Свой виджет — это Rust-код, ему нужна сборка. В рантайме честно говорим об этом,
            // а не пропускаем молча.
            let name = if node.text.is_empty() {
                "custom".to_string()
            } else {
                node.text.clone()
            };
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(28, 30, 38))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(120, 96, 48),
                ))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(format!("⚙ {name}"))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(230, 180, 100)),
                    );
                    ui.label(
                        egui::RichText::new("свой виджет: нужна сборка (lazr build)")
                            .size(11.0)
                            .color(egui::Color32::from_rgb(150, 140, 120)),
                    );
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
    applied_theme: Option<Theme>,
    applied_icon_font: Option<String>,
    applied_ui_font: Option<String>,
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
        let (model, problems) = parse_any(&text);
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

/// Своя рамка окна для живого режима: тот же заголовок, что в собранном
/// приложении — тянуть за полосу, закрыть крестиком.
fn titlebar(ui: &mut egui::Ui, title: &str) {
    let height = 30.0;
    let rect = egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(ui.available_width(), height));
    let drag = ui.interact(rect, egui::Id::new("lazr:titlebar"), egui::Sense::click_and_drag());
    if drag.drag_started() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    let close = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 16.0, rect.center().y),
        egui::vec2(24.0, 24.0),
    );
    let painter = ui.painter().clone();
    let fill = if drag.hovered() {
        ui.visuals().widgets.hovered.bg_fill
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };
    painter.rect_filled(rect, egui::CornerRadius::ZERO, fill);
    let ink = ui.visuals().strong_text_color();
    painter.text(
        egui::pos2(rect.left() + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(12.0),
        ink,
    );
    let cross = egui::Stroke::new(1.4, ink);
    let center = close.center();
    painter.line_segment(
        [
            center + egui::vec2(-4.0, -4.0),
            center + egui::vec2(4.0, 4.0),
        ],
        cross,
    );
    painter.line_segment(
        [
            center + egui::vec2(4.0, -4.0),
            center + egui::vec2(-4.0, 4.0),
        ],
        cross,
    );
    if ui
        .interact(close, egui::Id::new("lazr:close"), egui::Sense::click())
        .clicked()
    {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
    }
    ui.add_space(height);
}

impl eframe::App for LiveApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.watch();
        let wanted = self.model.theme.clone();
        if self.applied_theme.as_ref() != Some(&wanted) {
            apply_theme(ui.ctx(), &wanted);
            self.applied_theme = Some(wanted);
        }
        if self.applied_ui_font.as_ref() != self.model.ui_font.as_ref() {
            if let Some(path) = self.model.ui_font.clone() {
                apply_ui_font(ui.ctx(), &path);
            }
            self.applied_ui_font = self.model.ui_font.clone();
        }
        // шрифт иконок ставим один раз на путь
        if self.applied_icon_font.as_ref() != self.model.icon_font.as_ref() {
            if let Some(path) = self.model.icon_font.clone() {
                apply_icon_font(ui.ctx(), &path);
            }
            self.applied_icon_font = self.model.icon_font.clone();
        }
        if self.model.window.own {
            let title = self.model.title.clone();
            titlebar(ui, &title);
        }
        if self.model.window.center
            && !ui.data(|data| data.get_temp::<bool>(egui::Id::new("lazr:centered")).unwrap_or(false))
        {
            if let Some(monitor) = ui.ctx().input(|input| input.viewport().monitor_size) {
                let (width, height) = (self.model.width as f32, self.model.height as f32);
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                        (monitor.x - width) / 2.0,
                        (monitor.y - height) / 2.0,
                    )));
                ui.data_mut(|data| data.insert_temp(egui::Id::new("lazr:centered"), true));
            }
        }
        if self.model.window.drag_anywhere
            && ui
                .interact(
                    ui.max_rect(),
                    egui::Id::new("lazr:drag"),
                    egui::Sense::drag(),
                )
                .drag_started()
        {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
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
    let (model, problems) = parse_any(&text);
    let mut state = HashMap::new();
    for (name, value) in &model.fields {
        state.insert(name.clone(), value.clone());
    }
    let width = model.width as f32;
    let height = model.height as f32 + 44.0;
    let title = model.title.clone();
    let mut app = LiveApp {
        applied_icon_font: None,
        applied_ui_font: None,
        path: path.clone(),
        model,
        problems,
        state,
        modified: std::fs::metadata(&path).and_then(|m| m.modified()).ok(),
        applied_theme: None,
    };
    if std::env::args().any(|arg| arg == "--trace") {
        TRACE.store(true, Ordering::Relaxed);
    }
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
            if node.handler.is_none() && node.effect.is_none() {
                continue;
            }
            let before = app.state.clone();
            match (&node.handler, &node.effect) {
                (Some(handler), _) => eval_handler(&mut app.state, handler),
                (None, Some(effect)) => run_effect(&mut app.state, effect),
                _ => {}
            }
            actions += 1;
            let mut changes: Vec<String> = Vec::new();
            for (name, value) in &app.state {
                match before.get(name) {
                    Some(old) if old == value => {}
                    Some(old) => changes.push(format!("{name}: {} → {}", old.show(), value.show())),
                    None => changes.push(format!("{name}: появилось {}", value.show())),
                }
            }
            let shown = node
                .effect
                .clone()
                .unwrap_or_else(|| "форма".to_string());
            if changes.is_empty() {
                println!("  «{}»: без изменений ({shown})", node.text);
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
    // Опции окна из модели — те же, что у сгенерированного приложения.
    let window = &app.model.window;
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([width, height])
        .with_title(title);
    if window.own {
        viewport = viewport.with_decorations(false);
    }
    if !window.resizable {
        viewport = viewport.with_resizable(false);
    }
    if window.transparent {
        viewport = viewport.with_transparent(true);
    }
    if window.topmost {
        viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnTop);
    }
    if let Some((w, h)) = window.min {
        viewport = viewport.with_min_inner_size([w as f32, h as f32]);
    }
    if let Some((w, h)) = window.max {
        viewport = viewport.with_max_inner_size([w as f32, h as f32]);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let _ = eframe::run_native(
        "lazr live",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    );
}
"####;

#[cfg(test)]
mod tests {
    use super::RUNTIME_SRC;

    /// Рантайм живёт строкой RUNTIME_SRC, поэтому «знает ли он форму» проверяем
    /// по самой строке: неизвестная форма превращается в «форма … неизвестна»
    /// и валит `lazr live --problems` на живой демке.
    #[test]
    fn runtime_source_knows_font_family_mono_and_hint() {
        assert!(
            RUNTIME_SRC.contains(r#""font-family" | "font_family" | "ui-font" =>"#),
            "рантайм не разбирает font-family"
        );
        assert!(
            RUNTIME_SRC.contains(r#""mono" | "моно" => node.mono = true"#),
            "рантайм не понимает моноРежим"
        );
        assert!(
            RUNTIME_SRC.contains(r#""hint" | "подсказка" => node.hint"#),
            "рантайм не понимает подсказки"
        );
    }
}
