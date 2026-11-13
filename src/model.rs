//! Модель экрана v2: стили, сетки, прокрутка, размеры, выравнивание, Rust-блоки.
//!
//! ```text
//! window "Счётчик" 420x260
//! state count int 0
//! rust init { state.count = 42; }
//! rust widget badge { ui.label(format!("бейдж: {}", state.count)); }
//! vbox {pad:12}
//!   label "Нажатий: {count}" {size:22, bold}
//!   grid 2
//!     button "Плюс" do count += 1
//!     button "Минус" do count -= 1
//!   custom badge
//! ```

use crate::expr;

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Int,
    Float,
    Bool,
    Str,
}

impl FieldType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FieldType::Int => "int",
            FieldType::Float => "float",
            FieldType::Bool => "bool",
            FieldType::Str => "string",
        }
    }
    pub fn parse(text: &str) -> Option<FieldType> {
        Some(match text {
            "int" => FieldType::Int,
            "float" => FieldType::Float,
            "bool" => FieldType::Bool,
            "string" | "str" => FieldType::Str,
            _ => return None,
        })
    }
    pub fn rust(&self) -> &'static str {
        match self {
            FieldType::Int => "i64",
            FieldType::Float => "f32",
            FieldType::Bool => "bool",
            FieldType::Str => "String",
        }
    }
    pub fn numeric(&self) -> bool {
        matches!(self, FieldType::Int | FieldType::Float)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Align {
    Left,
    Center,
    Right,
}

impl Align {
    pub fn parse(text: &str) -> Option<Align> {
        Some(match text {
            "left" => Align::Left,
            "center" => Align::Center,
            "right" => Align::Right,
            _ => return None,
        })
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Align::Left => "left",
            Align::Center => "center",
            Align::Right => "right",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Style {
    pub color: Option<String>,
    pub size: Option<f32>,
    pub bold: bool,
    pub pad: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub align: Option<Align>,
    pub grow: bool,
    /// Горячая клавиша вида "Ctrl+S" — срабатывает без фокуса на кнопке.
    pub key: Option<String>,
    /// Имя иконки из встроенного набора lazr.
    pub icon: Option<String>,
}

impl Style {
    pub fn is_empty(&self) -> bool {
        self.color.is_none()
            && self.size.is_none()
            && !self.bold
            && self.pad.is_none()
            && self.width.is_none()
            && self.height.is_none()
            && self.align.is_none()
            && !self.grow
            && self.key.is_none()
            && self.icon.is_none()
    }
    pub fn as_str(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(color) = &self.color {
            parts.push(format!("color:{color}"));
        }
        if let Some(size) = self.size {
            parts.push(format!("size:{}", trim_float(size)));
        }
        if let Some(key) = &self.key {
            parts.push(format!("key:{key}"));
        }
        if let Some(icon) = &self.icon {
            parts.push(format!("icon:{icon}"));
        }
        if self.bold {
            parts.push("bold".to_string());
        }
        if let Some(pad) = self.pad {
            parts.push(format!("pad:{}", trim_float(pad)));
        }
        if let Some(width) = self.width {
            parts.push(format!("width:{}", trim_float(width)));
        }
        if let Some(height) = self.height {
            parts.push(format!("height:{}", trim_float(height)));
        }
        if let Some(align) = &self.align {
            parts.push(format!("align:{}", align.as_str()));
        }
        if self.grow {
            parts.push("grow".to_string());
        }
        parts.join(", ")
    }
}

pub fn trim_float(value: f32) -> String {
    if (value - value.round()).abs() < 0.001 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value}")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WidgetKind {
    Label,
    Button,
    Checkbox,
    TextInput,
    Slider,
    Separator,
    Space,
    Vbox,
    Hbox,
    Grid,
    Scroll,
    Custom(String),
}

impl WidgetKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            WidgetKind::Label => "label",
            WidgetKind::Button => "button",
            WidgetKind::Checkbox => "checkbox",
            WidgetKind::TextInput => "text_input",
            WidgetKind::Slider => "slider",
            WidgetKind::Separator => "separator",
            WidgetKind::Space => "space",
            WidgetKind::Vbox => "vbox",
            WidgetKind::Hbox => "hbox",
            WidgetKind::Grid => "grid",
            WidgetKind::Scroll => "scroll",
            WidgetKind::Custom(_) => "custom",
        }
    }
    pub fn is_container(&self) -> bool {
        matches!(
            self,
            WidgetKind::Vbox | WidgetKind::Hbox | WidgetKind::Grid | WidgetKind::Scroll
        )
    }
    pub fn parse_kind(text: &str) -> Option<WidgetKind> {
        Some(match text {
            "label" => WidgetKind::Label,
            "button" => WidgetKind::Button,
            "checkbox" => WidgetKind::Checkbox,
            "text_input" => WidgetKind::TextInput,
            "slider" => WidgetKind::Slider,
            "separator" => WidgetKind::Separator,
            "space" => WidgetKind::Space,
            "vbox" => WidgetKind::Vbox,
            "hbox" => WidgetKind::Hbox,
            "grid" => WidgetKind::Grid,
            "scroll" => WidgetKind::Scroll,
            _ => return None,
        })
    }
    pub fn all() -> Vec<&'static str> {
        vec![
            "label",
            "button",
            "checkbox",
            "text_input",
            "slider",
            "separator",
            "space",
            "vbox",
            "hbox",
            "grid",
            "scroll",
            "custom",
        ]
    }
}

#[derive(Debug, Clone)]
pub struct Widget {
    pub kind: WidgetKind,
    pub text: String,
    pub effect: Option<String>,
    pub bind: Option<String>,
    pub range: Option<(f64, f64)>,
    /// Число колонок у grid или пиксели отступа у space.
    pub number: Option<f32>,
    pub style: Style,
    pub children: Vec<Widget>,
}

impl Widget {
    pub fn leaf(kind: WidgetKind, text: &str) -> Widget {
        Widget {
            kind,
            text: text.to_string(),
            effect: None,
            bind: None,
            range: None,
            number: None,
            style: Style::default(),
            children: Vec::new(),
        }
    }
    pub fn container(kind: WidgetKind) -> Widget {
        Widget::leaf(kind, "")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RustKind {
    Init,
    Widget,
    /// Функция уровня модуля: полный Rust-код, видимый всем виджетам.
    Fn,
}

#[derive(Debug, Clone)]
pub struct RustBlock {
    pub name: String,
    pub kind: RustKind,
    pub code: String,
}

#[derive(Debug, Clone)]
pub struct RustField {
    pub name: String,
    pub ty: String,
    pub default: String,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: FieldType,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Screen {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub fields: Vec<Field>,
    pub rust: Vec<RustBlock>,
    pub rust_fields: Vec<RustField>,
    pub root: Vec<Widget>,
}

#[derive(Debug, Clone)]
pub struct ModelError {
    pub line: usize,
    pub code: &'static str,
    pub message: String,
    pub hint: String,
}

impl ModelError {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"line\":{},\"code\":\"{}\",\"message\":{},\"hint\":{}}}",
            self.line,
            self.code,
            json_str(&self.message),
            json_str(&self.hint)
        )
    }
}

pub fn json_str(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Разбор строки на слова: кавычки и `{…}` — один токен, даже с пробелами внутри.
fn tokens(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut depth = 0;
    for ch in line.chars() {
        if escaped {
            current.push(match ch {
                'n' => '\n',
                't' => '\t',
                other => other,
            });
            escaped = false;
            continue;
        }
        if quoted {
            match ch {
                '\\' => escaped = true,
                '"' => {
                    out.push(current.clone());
                    current.clear();
                    quoted = false;
                }
                c => current.push(c),
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
                if depth > 1 {
                    current.push('{');
                }
            }
            '}' => {
                if depth == 0 {
                    current.push('}');
                    continue;
                }
                depth -= 1;
                if depth == 0 {
                    out.push(format!("{{{current}}}"));
                    current.clear();
                } else {
                    current.push('}');
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

pub fn parse_style_text(text: &str) -> Result<Style, String> {
    parse_style(&format!("{{{text}}}"))
}

fn parse_style(text: &str) -> Result<Style, String> {
    let inner = text.trim().trim_start_matches('{').trim_end_matches('}');
    let mut style = Style::default();
    for part in inner.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (key, value) = match part.split_once(':') {
            Some((key, value)) => (key.trim(), value.trim()),
            None => (part, ""),
        };
        match key {
            "color" | "colour" => style.color = Some(value.to_string()),
            "size" => {
                style.size = Some(
                    value
                        .parse()
                        .map_err(|_| format!("size «{value}» не число"))?,
                )
            }
            "bold" => style.bold = true,
            "key" => style.key = Some(value.to_string()),
            "icon" => style.icon = Some(value.to_string()),
            "pad" | "padding" => {
                style.pad = Some(
                    value
                        .parse()
                        .map_err(|_| format!("pad «{value}» не число"))?,
                )
            }
            "width" => {
                style.width = Some(
                    value
                        .parse()
                        .map_err(|_| format!("width «{value}» не число"))?,
                )
            }
            "height" => {
                style.height = Some(
                    value
                        .parse()
                        .map_err(|_| format!("height «{value}» не число"))?,
                )
            }
            "align" => {
                style.align = Some(
                    Align::parse(value)
                        .ok_or_else(|| format!("align «{value}»: доступно left, center, right"))?,
                )
            }
            "grow" => style.grow = true,
            other => return Err(format!("неизвестное свойство стиля «{other}»")),
        }
    }
    Ok(style)
}

pub fn parse(text: &str) -> Result<Screen, Vec<ModelError>> {
    let mut errors: Vec<ModelError> = Vec::new();
    let mut screen = Screen {
        title: "Окно".to_string(),
        width: 480,
        height: 320,
        fields: Vec::new(),
        rust: Vec::new(),
        rust_fields: Vec::new(),
        root: Vec::new(),
    };
    let lines: Vec<&str> = text.lines().collect();
    let mut stack: Vec<(usize, Vec<usize>)> = vec![(0, Vec::new())];
    let mut index = 0;

    while index < lines.len() {
        let line_no = index + 1;
        let line = lines[index].trim_end();
        index += 1;
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let depth = indent / 2;
        let words = tokens(line.trim_start());
        let Some(head) = words.first().cloned() else {
            continue;
        };

        if head == "rust" {
            let Some(kind_word) = words.get(1).cloned() else {
                errors.push(ModelError {
                    line: line_no,
                    code: "RUST_NEEDS_KIND",
                    message: "у блока rust нет вида".into(),
                    hint: "rust init { … } или rust widget имя { … }".into(),
                });
                continue;
            };
            if kind_word == "field" {
                let (Some(name), Some(ty)) = (words.get(2).cloned(), words.get(3).cloned()) else {
                    errors.push(ModelError {
                        line: line_no,
                        code: "RUST_FIELD_INCOMPLETE",
                        message: "у rust field нет имени или типа".into(),
                        hint: "rust field points Vec<egui::Pos2> Vec::new()".into(),
                    });
                    continue;
                };
                let default = words
                    .get(4..)
                    .map(|rest| rest.join(" "))
                    .unwrap_or_default();
                screen.rust_fields.push(RustField {
                    name,
                    ty,
                    default: if default.is_empty() {
                        "Default::default()".to_string()
                    } else {
                        default
                    },
                });
                continue;
            }
            let (kind, name) = match kind_word.as_str() {
                "init" => (RustKind::Init, "init".to_string()),
                "fn" => (
                    RustKind::Fn,
                    words.get(2).cloned().unwrap_or_else(|| "f".into()),
                ),
                "widget" => match words.get(2) {
                    Some(name) => (RustKind::Widget, name.clone()),
                    None => {
                        errors.push(ModelError {
                            line: line_no,
                            code: "RUST_WIDGET_NEEDS_NAME",
                            message: "у rust widget нет имени".into(),
                            hint: "rust widget badge { … }".into(),
                        });
                        continue;
                    }
                },
                other => {
                    errors.push(ModelError {
                        line: line_no,
                        code: "RUST_UNKNOWN_KIND",
                        message: format!("вид блока «{other}» неизвестен"),
                        hint: "init или widget".into(),
                    });
                    continue;
                }
            };
            let rest: String = words[3.min(words.len())..].join(" ");
            let mut body = String::new();
            let mut closed = false;
            if let Some(inner) = rest.strip_prefix('{') {
                if inner.ends_with('}') {
                    body.push_str(inner.trim_end_matches('}'));
                    closed = true;
                } else {
                    body.push_str(inner);
                }
            }
            while !closed && index < lines.len() {
                let next = lines[index];
                index += 1;
                let next_indent = next.len() - next.trim_start().len();
                if next.trim() == "}" && next_indent <= indent {
                    closed = true;
                    break;
                }
                body.push_str(next);
                body.push('\n');
            }
            if !closed {
                errors.push(ModelError {
                    line: line_no,
                    code: "RUST_BLOCK_UNCLOSED",
                    message: "блок rust не закрыт".into(),
                    hint: "закройте блок строкой «}» на той же глубине".into(),
                });
                continue;
            }
            if kind == RustKind::Widget && !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                errors.push(ModelError {
                    line: line_no,
                    code: "RUST_BAD_NAME",
                    message: format!("имя виджета «{name}» не годится"),
                    hint: "буквы, цифры и _".into(),
                });
                continue;
            }
            screen.rust.push(RustBlock {
                name,
                kind,
                code: body.trim().to_string(),
            });
            continue;
        }

        match head.as_str() {
            "window" => {
                if words.len() < 2 {
                    errors.push(ModelError {
                        line: line_no,
                        code: "WINDOW_NEEDS_TITLE",
                        message: "у window нет заголовка".into(),
                        hint: "window \"Заголовок\" 480x320".into(),
                    });
                    continue;
                }
                screen.title = words[1].clone();
                if let Some(size) = words.get(2) {
                    if let Some((w, h)) = size.split_once('x') {
                        screen.width = w.parse().unwrap_or(screen.width);
                        screen.height = h.parse().unwrap_or(screen.height);
                    }
                }
            }
            "state" => {
                if words.len() < 3 {
                    errors.push(ModelError {
                        line: line_no,
                        code: "STATE_NEEDS_TYPE",
                        message: "у state нет типа".into(),
                        hint: "state count int 0".into(),
                    });
                    continue;
                }
                let Some(ty) = FieldType::parse(&words[2]) else {
                    errors.push(ModelError {
                        line: line_no,
                        code: "UNKNOWN_TYPE",
                        message: format!("тип «{}» неизвестен", words[2]),
                        hint: "int, float, bool, string".into(),
                    });
                    continue;
                };
                let value = words.get(3).cloned().unwrap_or_else(|| match ty {
                    FieldType::Str => String::new(),
                    FieldType::Bool => "false".into(),
                    FieldType::Int => "0".into(),
                    FieldType::Float => "0.0".into(),
                });
                screen.fields.push(Field {
                    name: words[1].clone(),
                    ty,
                    value,
                });
            }
            _ => {
                let (kind, mut text_start): (WidgetKind, usize) = if head == "custom" {
                    match words.get(1) {
                        Some(name) => (WidgetKind::Custom(name.clone()), 2),
                        None => {
                            errors.push(ModelError {
                                line: line_no,
                                code: "CUSTOM_NEEDS_NAME",
                                message: "у custom нет имени виджета".into(),
                                hint: "custom badge — где badge объявлен в rust widget".into(),
                            });
                            continue;
                        }
                    }
                } else {
                    match WidgetKind::parse_kind(&head) {
                        Some(kind) => (kind, 1),
                        None => {
                            errors.push(ModelError {
                                line: line_no,
                                code: "UNKNOWN_WIDGET",
                                message: format!("виджет «{head}» неизвестен"),
                                hint: format!("доступны: {}", WidgetKind::all().join(", ")),
                            });
                            continue;
                        }
                    }
                };
                // Текст есть только у листовых виджетов и не может быть стилем/числом.
                let mut text = String::new();
                if kind.is_container() {
                    text_start = 1;
                } else if matches!(kind, WidgetKind::Custom(_)) {
                    text_start = 2;
                } else {
                    match words.get(1) {
                        Some(candidate)
                            if !candidate.starts_with('{')
                                && candidate != "do"
                                && candidate != "->"
                                && candidate.parse::<f32>().is_err() =>
                        {
                            text = candidate.clone();
                            text_start = 2;
                        }
                        _ => text_start = 1,
                    }
                }

                while stack.len() > 1 && stack.last().map(|(d, _)| *d >= depth).unwrap_or(false) {
                    stack.pop();
                }
                let mut widget = if kind.is_container() {
                    Widget::container(kind.clone())
                } else if matches!(kind, WidgetKind::Custom(_)) {
                    Widget::leaf(kind.clone(), "")
                } else {
                    Widget::leaf(kind.clone(), &text)
                };
                let rest: Vec<String> = words
                    .get(text_start.min(words.len())..)
                    .unwrap_or(&[])
                    .to_vec();
                let mut i = 0;
                while i < rest.len() {
                    let token = rest[i].clone();
                    if token.starts_with('{') {
                        match parse_style(&token) {
                            Ok(style) => widget.style = style,
                            Err(err) => errors.push(ModelError {
                                line: line_no,
                                code: "BAD_STYLE",
                                message: err,
                                hint:
                                    "{color:#e5484d, size:18, bold, pad:8, width:120, align:center}"
                                        .into(),
                            }),
                        }
                        i += 1;
                    } else if token == "do" {
                        // Стиль и привязка живут после действия — в него не входят.
                        let mut end = rest.len();
                        for (offset, item) in rest.iter().enumerate().skip(i + 1) {
                            // Стиль — это {ключ:значение}, а не блок «{ count = 0 }».
                            let is_style = item.starts_with('{')
                                && item.ends_with('}')
                                && item.contains(':')
                                && !item.contains(' ');
                            if is_style || item == "->" {
                                end = offset;
                                break;
                            }
                        }
                        widget.effect = Some(rest[i + 1..end].join(" "));
                        i = end;
                        continue;
                    } else if token == "->" {
                        widget.bind = rest.get(i + 1).cloned();
                        i += 2;
                    } else if token.contains("..") {
                        if let Some((a, b)) = token.split_once("..") {
                            if let (Ok(a), Ok(b)) = (a.parse(), b.parse()) {
                                widget.range = Some((a, b));
                            }
                        }
                        i += 1;
                    } else if let Ok(number) = token.parse::<f32>() {
                        widget.number = Some(number);
                        i += 1;
                    } else {
                        i += 1;
                    }
                }

                if widget.kind.is_container() {
                    let path = stack.last().map(|(_, p)| p.clone()).unwrap_or_default();
                    let parent = child_list(&mut screen.root, &path);
                    parent.push(widget);
                    let index_in_parent = parent.len() - 1;
                    let mut child_path = path;
                    child_path.push(index_in_parent);
                    stack.push((depth, child_path));
                } else {
                    let path = stack.last().map(|(_, p)| p.clone()).unwrap_or_default();
                    child_list(&mut screen.root, &path).push(widget);
                }
            }
        }
    }

    validate(&screen, &mut errors);
    if errors.is_empty() {
        Ok(screen)
    } else {
        Err(errors)
    }
}

fn validate(screen: &Screen, errors: &mut Vec<ModelError>) {
    let field_names: Vec<String> = screen.fields.iter().map(|f| f.name.clone()).collect();
    let custom: Vec<String> = screen
        .rust
        .iter()
        .filter(|b| b.kind == RustKind::Widget)
        .map(|b| b.name.clone())
        .collect();

    fn walk(
        widgets: &[Widget],
        field_names: &[String],
        custom: &[String],
        screen: &Screen,
        errors: &mut Vec<ModelError>,
    ) {
        for widget in widgets {
            if let Some(bind) = &widget.bind {
                if !field_names.contains(bind) {
                    errors.push(ModelError {
                        line: 0,
                        code: "NO_SUCH_FIELD",
                        message: format!("привязка к неизвестному полю «{bind}»"),
                        hint: format!("объявите: state {bind} <тип>"),
                    });
                }
            }
            if let Some(effect) = &widget.effect {
                if let Err(err) = expr::compile(screen, effect) {
                    errors.push(ModelError {
                        line: 0,
                        code: "BAD_EFFECT",
                        message: err.message,
                        hint: err.hint,
                    });
                }
            }
            if let WidgetKind::Custom(name) = &widget.kind {
                if !custom.contains(name) {
                    errors.push(ModelError {
                        line: 0,
                        code: "NO_SUCH_CUSTOM",
                        message: format!("свой виджет «{name}» не объявлен"),
                        hint: format!("добавьте: rust widget {name} {{ … }}"),
                    });
                }
            }
            walk(&widget.children, field_names, custom, screen, errors);
        }
    }
    walk(&screen.root, &field_names, &custom, screen, errors);
}

fn child_list<'a>(root: &'a mut Vec<Widget>, path: &[usize]) -> &'a mut Vec<Widget> {
    let mut list = root;
    for index in path {
        list = &mut list[*index].children;
    }
    list
}

pub fn render(screen: &Screen) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "window {} {}x{}\n",
        quote(&screen.title),
        screen.width,
        screen.height
    ));
    for field in &screen.fields {
        out.push_str(&format!(
            "state {} {} {}\n",
            field.name,
            field.ty.as_str(),
            field.value
        ));
    }
    for field in &screen.rust_fields {
        out.push_str(&format!(
            "rust field {} {} {}\n",
            field.name, field.ty, field.default
        ));
    }
    for block in &screen.rust {
        match block.kind {
            RustKind::Init => out.push_str("rust init {\n"),
            RustKind::Widget => out.push_str(&format!("rust widget {} {{\n", block.name)),
            RustKind::Fn => out.push_str(&format!("rust fn {} {{\n", block.name)),
        }
        for line in block.code.lines() {
            out.push_str(&format!("    {line}\n"));
        }
        out.push_str("}\n");
    }
    render_widgets(&screen.root, 0, &mut out);
    out
}

fn render_widgets(widgets: &[Widget], depth: usize, out: &mut String) {
    for widget in widgets {
        let pad = "  ".repeat(depth);
        let mut line = format!("{pad}{}", widget.kind.as_str());
        match &widget.kind {
            WidgetKind::Custom(name) => line.push_str(&format!(" {name}")),
            _ => {
                if !widget.text.is_empty() {
                    line.push(' ');
                    line.push_str(&quote(&widget.text));
                }
            }
        }
        if let Some(number) = widget.number {
            line.push_str(&format!(" {}", trim_float(number)));
        }
        if let Some(bind) = &widget.bind {
            line.push_str(&format!(" -> {bind}"));
        }
        if let Some((a, b)) = widget.range {
            line.push_str(&format!(" {a}..{b}"));
        }
        if let Some(effect) = &widget.effect {
            line.push_str(&format!(" do {effect}"));
        }
        if !widget.style.is_empty() {
            line.push_str(&format!(" {{{}}}", widget.style.as_str()));
        }
        out.push_str(&line);
        out.push('\n');
        render_widgets(&widget.children, depth + 1, out);
    }
}

pub fn quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

pub fn add_widget(
    screen: &mut Screen,
    container: Option<&str>,
    widget: Widget,
) -> Result<(), ModelError> {
    if let Some(name) = container {
        if let Some(list) = find_container(&mut screen.root, name) {
            list.push(widget);
            return Ok(());
        }
        return Err(ModelError {
            line: 0,
            code: "NO_SUCH_CONTAINER",
            message: format!("контейнер «{name}» не найден"),
            hint: "vbox, hbox, grid или scroll".into(),
        });
    }
    screen.root.push(widget);
    Ok(())
}

fn find_container<'a>(widgets: &'a mut Vec<Widget>, kind: &str) -> Option<&'a mut Vec<Widget>> {
    for widget in widgets.iter_mut() {
        if widget.kind.is_container() && widget.kind.as_str() == kind {
            return Some(&mut widget.children);
        }
        if let Some(found) = find_container(&mut widget.children, kind) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
window "Счётчик" 420x260
state count int 0
state name string ""
vbox {pad:12}
  label "Нажатий: {count}" {size:22, bold, color:#e5484d}
  grid 2
    button "Плюс" do count += 1
    button "Минус" do count -= 1
  hbox {align:center}
    text_input "Имя" -> name {grow}
"#;

    #[test]
    fn styles_and_layouts_parse() {
        let screen = parse(SAMPLE).expect("модель валидна");
        let vbox = &screen.root[0];
        assert_eq!(vbox.style.pad, Some(12.0));
        let label = &vbox.children[0];
        assert_eq!(label.style.size, Some(22.0));
        assert!(label.style.bold);
        assert_eq!(label.style.color.as_deref(), Some("#e5484d"));
        let grid = &vbox.children[1];
        assert_eq!(grid.kind, WidgetKind::Grid);
        assert_eq!(grid.number, Some(2.0));
        assert_eq!(grid.children.len(), 2);
        let hbox = &vbox.children[2];
        assert_eq!(hbox.style.align, Some(Align::Center));
        assert!(hbox.children[0].style.grow);
    }

    #[test]
    fn unknown_style_property_is_refused() {
        let errors = parse("label \"x\" {shade:red}\n").unwrap_err();
        assert!(errors.iter().any(|e| e.code == "BAD_STYLE"), "{errors:?}");
    }

    #[test]
    fn rust_blocks_are_read_and_checked() {
        let screen = parse(
            "state count int 0\nrust init {\n  state.count = 42;\n}\nrust widget badge {\n  ui.label(\"x\");\n}\ncustom badge\n",
        )
        .unwrap();
        assert_eq!(screen.rust.len(), 2);
        assert!(screen.rust[0].code.contains("state.count = 42;"));
        assert_eq!(screen.rust[1].name, "badge");
        assert_eq!(screen.root[0].kind, WidgetKind::Custom("badge".into()));
    }

    #[test]
    fn custom_without_block_is_refused() {
        let errors = parse("custom ghost\n").unwrap_err();
        assert!(errors.iter().any(|e| e.code == "NO_SUCH_CUSTOM"));
    }

    #[test]
    fn roundtrip_keeps_styles_and_rust() {
        let screen = parse(SAMPLE).unwrap();
        let again = parse(&render(&screen)).unwrap();
        assert_eq!(again.root[0].style.pad, Some(12.0));
        assert_eq!(again.root[0].children[1].number, Some(2.0));
        assert_eq!(
            again.root[0].children[0].style.color.as_deref(),
            Some("#e5484d")
        );
    }

    #[test]
    fn effect_errors_surface_with_hint() {
        let errors = parse("state count int 0\nbutton \"x\" do count += nope\n").unwrap_err();
        assert!(errors.iter().any(|e| e.code == "BAD_EFFECT"));
    }
}
