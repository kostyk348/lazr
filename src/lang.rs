//! DSL: S-выражения как единственный язык автора. Rust — только HAL.
//!
//! Идея Lisp-машины: приложение описывается не кодом на языке хоста, а формами
//! DSL. Здесь DSL — S-выражения; они разбираются, вычисляются в дерево экрана и
//! дальше идут в тот же генератор, что и строчная модель (`form.lazr`).
//!
//! ```lisp
//! (window "Магазин" 520 340)
//! (state count int 0)
//! (vbox :pad 12
//!   (label "Нажатий: {count}" :size 22 :bold true :color "#e5484d")
//!   (button "Плюс" (if (>= count 9) (set! count 0) (set! count (+ count 1))))
//!   (text-input "Имя" :bind name))
//! ```
//!
//! Что это даёт: у DSL нет проблем с кавычками, отступами и порядком флагов;
//! одна форма — одна операция, и её легко собирать программно. Rust при этом
//! остаётся тем, чем и должен быть: памятью, окном, отрисовкой.

use std::collections::HashMap;

use crate::model::{
    self, Align, Field, FieldType, RustBlock, RustKind, Screen, Style, Widget, WidgetKind,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Num(f64),
    Str(String),
    /// Голое слово: имя поля или формы (в отличие от строки в кавычках).
    Sym(String),
    Bool(bool),
    List(Vec<Value>),
    Nil,
}

impl Value {
    fn num(&self) -> Option<f64> {
        match self {
            Value::Num(value) => Some(*value),
            _ => None,
        }
    }
    fn text(&self) -> Option<String> {
        match self {
            Value::Str(text) | Value::Sym(text) => Some(text.clone()),
            Value::Num(value) => Some(format!("{}", model::trim_float(*value as f32))),
            Value::Bool(value) => Some(value.to_string()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LispError {
    pub message: String,
    pub hint: String,
}

impl LispError {
    fn new(message: impl Into<String>, hint: impl Into<String>) -> LispError {
        LispError {
            message: message.into(),
            hint: hint.into(),
        }
    }
}

// --- читатель ---------------------------------------------------------------------

pub fn read(source: &str) -> Result<Vec<Value>, LispError> {
    let mut tokens = tokenize(source)?;
    let mut out = Vec::new();
    while !tokens.is_empty() {
        out.push(read_form(&mut tokens)?);
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Open,
    Close,
    Str(String),
    Word(String),
}

/// Токен вместе с номером строки: ошибки читалки должны указывать место.
type Placed = (Token, usize);

fn tokenize(source: &str) -> Result<Vec<Placed>, LispError> {
    let mut out: Vec<Placed> = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let mut line = 1;
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        match ch {
            '\n' => {
                line += 1;
                i += 1;
            }
            '(' => {
                out.push((Token::Open, line));
                i += 1;
            }
            ')' => {
                out.push((Token::Close, line));
                i += 1;
            }
            ';' => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            c if c.is_whitespace() => i += 1,
            '"' if chars.get(i + 1) == Some(&'"') && chars.get(i + 2) == Some(&'"') => {
                let started = line;
                i += 3;
                let mut text = String::new();
                loop {
                    if i + 2 >= chars.len() {
                        return Err(LispError::new(
                            format!("сырая строка не закрыта (строка {started})"),
                            "закройте тремя кавычками: \"\"\"…\"\"\"",
                        ));
                    }
                    if chars[i] == '"' && chars[i + 1] == '"' && chars[i + 2] == '"' {
                        i += 3;
                        break;
                    }
                    if chars[i] == '\n' {
                        line += 1;
                    }
                    text.push(chars[i]);
                    i += 1;
                }
                out.push((Token::Str(text), started));
            }
            '"' => {
                let started = line;
                i += 1;
                let mut text = String::new();
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                        text.push(match chars[i] {
                            'n' => '\n',
                            't' => '\t',
                            other => other,
                        });
                    } else {
                        text.push(chars[i]);
                    }
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(LispError::new(
                        format!("строка не закрыта (строка {started})"),
                        "закройте кавычку: \"текст\"",
                    ));
                }
                i += 1;
                out.push((Token::Str(text), started));
            }
            _ => {
                let mut word = String::new();
                while i < chars.len()
                    && !chars[i].is_whitespace()
                    && chars[i] != '('
                    && chars[i] != ')'
                {
                    word.push(chars[i]);
                    i += 1;
                }
                out.push((Token::Word(word), line));
            }
        }
    }
    Ok(out)
}

fn read_form(tokens: &mut Vec<Placed>) -> Result<Value, LispError> {
    if tokens.is_empty() {
        return Err(LispError::new("неожиданный конец", "проверьте скобки"));
    }
    let (token, line) = tokens.remove(0);
    match token {
        Token::Open => {
            let mut items = Vec::new();
            loop {
                match tokens.first() {
                    Some((Token::Close, _)) => {
                        tokens.remove(0);
                        return Ok(Value::List(items));
                    }
                    None => {
                        return Err(LispError::new(
                            format!("скобка не закрыта (открыта на строке {line})"),
                            "добавьте «)»",
                        ))
                    }
                    _ => items.push(read_form(tokens)?),
                }
            }
        }
        Token::Close => Err(LispError::new(
            format!("лишняя «)» на строке {line}"),
            "уберите лишнюю скобку или закройте сырую строку тремя кавычками",
        )),
        Token::Str(text) => Ok(Value::Str(text)),
        Token::Word(word) => Ok(parse_atom(&word)),
    }
}

fn parse_atom(word: &str) -> Value {
    if word == "nil" {
        return Value::Nil;
    }
    if word == "true" {
        return Value::Bool(true);
    }
    if word == "false" {
        return Value::Bool(false);
    }
    if let Ok(number) = word.parse::<f64>() {
        return Value::Num(number);
    }
    Value::Sym(word.to_string())
}

// --- вычислитель ------------------------------------------------------------------

#[derive(Debug)]
pub struct Evaluated {
    pub screen: Screen,
    /// Значения, вычисленные формами верхнего уровня (def/set!), — для REPL.
    #[allow(dead_code)]
    pub env: HashMap<String, Value>,
}

struct Builder {
    screen: Screen,
    env: HashMap<String, Value>,
}

pub fn eval(forms: &[Value]) -> Result<Evaluated, LispError> {
    let mut builder = Builder {
        screen: Screen {
            title: "Окно".to_string(),
            width: 480,
            height: 320,
            fields: Vec::new(),
            rust: Vec::new(),
            rust_fields: Vec::new(),
            theme: Default::default(),
            root: vec![Widget::container(WidgetKind::Vbox)],
        },
        env: HashMap::new(),
    };
    let mut top: Vec<Widget> = Vec::new();
    // Корневой vbox в модели не нужен, если формы верхнего уровня — уже виджеты.
    for form in forms {
        if let Some(widget) = builder.widget(form)? {
            top.push(widget);
        }
    }
    builder.screen.root = if top.is_empty() {
        vec![Widget::container(WidgetKind::Vbox)]
    } else {
        top
    };
    Ok(Evaluated {
        screen: builder.screen,
        env: builder.env,
    })
}

impl Builder {
    fn widget(&mut self, form: &Value) -> Result<Option<Widget>, LispError> {
        let Value::List(items) = form else {
            return Err(LispError::new(
                "ожидалась форма в скобках",
                "(label \"привет\")",
            ));
        };
        if items.is_empty() {
            return Ok(None);
        }
        let head = items[0].text().unwrap_or_default();
        let args = &items[1..];

        match head.as_str() {
            "window" => {
                self.screen.title = args
                    .first()
                    .and_then(Value::text)
                    .unwrap_or_else(|| "Окно".to_string());
                if let Some(width) = args.get(1).and_then(Value::num) {
                    self.screen.width = width as u32;
                }
                if let Some(height) = args.get(2).and_then(Value::num) {
                    self.screen.height = height as u32;
                }
                Ok(None)
            }
            "state" => {
                let name = args
                    .first()
                    .and_then(Value::text)
                    .ok_or_else(|| LispError::new("(state имя тип значение)", "state count int 0"))?;
                let ty = args
                    .get(1)
                    .and_then(Value::text)
                    .ok_or_else(|| LispError::new("(state имя тип значение)", "state count int 0"))?;
                let ty = FieldType::parse(&ty).ok_or_else(|| {
                    LispError::new(format!("тип «{ty}» неизвестен"), "int, float, bool, string")
                })?;
                let value = args.get(2).and_then(Value::text).unwrap_or_default();
                self.screen.fields.push(Field { name, ty, value });
                Ok(None)
            }
            "theme" => {
                let mut theme = crate::model::Theme::default();
                let mut i = 0;
                if let Some(first) = args.first() {
                    match first.text().as_deref() {
                        Some("dark") => {
                            theme.dark = true;
                            i = 1;
                        }
                        Some("light") => {
                            theme.dark = false;
                            i = 1;
                        }
                        _ => {}
                    }
                }
                while i < args.len() {
                    let key = match &args[i] {
                        // Допускаем и :accent, и accent — тема не место для педантизма.
                        Value::Sym(key) => key.trim_start_matches(':').to_string(),
                        other => {
                            return Err(LispError::new(
                                format!("в теме ожидался параметр :имя, а не {other:?}"),
                                "(theme dark :accent \"#388bfd\" :font 15)",
                            ))
                        }
                    };
                    let value = args.get(i + 1).cloned().unwrap_or(Value::Nil);
                    match key.as_str() {
                        "accent" => theme.accent = value.text(),
                        "font" => {
                            theme.font = value.num().unwrap_or(theme.font as f64) as f32
                        }
                        "spacing" => {
                            theme.spacing = value.num().unwrap_or(theme.spacing as f64) as f32
                        }
                        "round" => theme.round = value.num().unwrap_or(theme.round as f64) as u8,
                        other => {
                            return Err(LispError::new(
                                format!("параметр темы :{other} неизвестен"),
                                ":accent :font :spacing :round",
                            ))
                        }
                    }
                    i += 2;
                }
                self.screen.theme = theme;
                Ok(None)
            }
            "rust-field" => {
                let (Some(name), Some(ty)) = (
                    args.first().and_then(Value::text),
                    args.get(1).and_then(Value::text),
                ) else {
                    return Err(LispError::new(
                        "(rust-field имя \"тип\" \"значение\")",
                        "rust-field points \"Vec<egui::Pos2>\" \"Vec::new()\"",
                    ));
                };
                let default = args
                    .get(2)
                    .and_then(Value::text)
                    .unwrap_or_else(|| "Default::default()".to_string());
                self.screen.rust_fields.push(crate::model::RustField {
                    name,
                    ty,
                    default,
                });
                Ok(None)
            }
            "rust-item" | "rust-fn" => {
                let name = args
                    .first()
                    .and_then(Value::text)
                    .ok_or_else(|| LispError::new("(rust-fn имя \"код\")", "имя функции и код"))?;
                let code = args
                    .get(1)
                    .and_then(Value::text)
                    .ok_or_else(|| LispError::new("(rust-fn имя \"код\")", "код строкой"))?;
                self.screen.rust.push(RustBlock {
                    name,
                    kind: RustKind::Fn,
                    code,
                });
                Ok(None)
            }
            "rust-init" | "rust-widget" => {
                let (name, code) = if head == "rust-widget" {
                    let name = args
                        .first()
                        .and_then(Value::text)
                        .ok_or_else(|| LispError::new("(rust-widget имя \"код\")", "имя и код"))?;
                    let code = args
                        .get(1)
                        .and_then(Value::text)
                        .ok_or_else(|| LispError::new("(rust-widget имя \"код\")", "код строкой"))?;
                    (name, code)
                } else {
                    let code = args
                        .first()
                        .and_then(Value::text)
                        .ok_or_else(|| LispError::new("(rust-init \"код\")", "код строкой"))?;
                    ("init".to_string(), code)
                };
                self.screen.rust.push(RustBlock {
                    name,
                    kind: if head == "rust-init" {
                        RustKind::Init
                    } else {
                        RustKind::Widget
                    },
                    code,
                });
                Ok(None)
            }
            "label" | "button" | "checkbox" | "text-input" | "slider" | "space" | "separator"
            | "vbox" | "hbox" | "grid" | "scroll" | "place" | "list" | "custom" => {
                self.node(&head, args)
            }
            other => Err(LispError::new(
                format!("форма «{other}» неизвестна"),
                "window, state, label, button, checkbox, text-input, slider, space, separator, vbox, hbox, grid, scroll, custom, rust-init, rust-widget",
            )),
        }
    }

    fn node(&mut self, head: &str, args: &[Value]) -> Result<Option<Widget>, LispError> {
        let kind = match head {
            "label" => WidgetKind::Label,
            "button" => WidgetKind::Button,
            "checkbox" => WidgetKind::Checkbox,
            "text-input" => WidgetKind::TextInput,
            "slider" => WidgetKind::Slider,
            "space" => WidgetKind::Space,
            "separator" => WidgetKind::Separator,
            "vbox" => WidgetKind::Vbox,
            "hbox" => WidgetKind::Hbox,
            "grid" => WidgetKind::Grid,
            "scroll" => WidgetKind::Scroll,
            "place" => WidgetKind::Place,
            "list" => WidgetKind::List,
            "custom" => WidgetKind::Custom(
                args.first()
                    .and_then(Value::text)
                    .ok_or_else(|| LispError::new("(custom имя)", "имя своего виджета"))?,
            ),
            _ => unreachable!(),
        };
        let mut widget = if kind.is_container() || matches!(kind, WidgetKind::Custom(_)) {
            Widget::container(kind.clone())
        } else {
            Widget::leaf(kind.clone(), "")
        };

        let mut index = 0;
        // Текст и обработчик — позиционные, остальное — ключевые слова.
        let numeric_head = matches!(head, "space" | "grid" | "place");
        if !kind.is_container() && !matches!(kind, WidgetKind::Custom(_)) && !numeric_head {
            if let Some(first) = args.first() {
                match first {
                    Value::List(_) => {}
                    other => {
                        let candidate = other.text().unwrap_or_default();
                        // «:bind» — ключ, а не текст: у виджета может не быть подписи
                        if !candidate.starts_with(':') {
                            widget.text = candidate;
                            index = 1;
                        }
                    }
                }
            }
            if head == "button" {
                if let Some(Value::List(_)) = args.get(index) {
                    widget.effect = Some(handler(&args[index])?);
                    index += 1;
                }
            }
        }
        if head == "space" {
            if let Some(value) = args.first().and_then(Value::num) {
                widget.number = Some(value as f32);
                index = 1;
            }
        }
        if head == "place" {
            if let (Some(w), Some(h)) = (
                args.first().and_then(Value::num),
                args.get(1).and_then(Value::num),
            ) {
                widget.box_size = Some((w as f32, h as f32));
                index = 2;
            }
        }
        if head == "grid" {
            if let Some(value) = args.first().and_then(Value::num) {
                widget.number = Some(value as f32);
                index = 1;
            }
        }

        let mut style = Style::default();
        let mut i = index;
        while i < args.len() {
            match &args[i] {
                Value::List(child) => {
                    let nested = self.widget(&Value::List(child.clone()))?;
                    if let Some(child_widget) = nested {
                        widget.children.push(child_widget);
                    }
                    i += 1;
                }
                // Позиционные атрибуты места: at X Y, size WxH, anchor right bottom.
                Value::Sym(word) if word == "at" => {
                    let x = args.get(i + 1).and_then(Value::num);
                    let y = args.get(i + 2).and_then(Value::num);
                    match (x, y) {
                        (Some(x), Some(y)) => {
                            widget.at = Some((x as f32, y as f32));
                            i += 3;
                        }
                        _ => {
                            return Err(LispError::new(
                                "у at нужны две координаты",
                                "(label \"x\" at 20 30)",
                            ))
                        }
                    }
                }
                Value::Sym(word) if word == "size" => {
                    let next = args.get(i + 1);
                    if let Some(Value::Sym(spec)) = next {
                        match spec.split_once('x') {
                            Some((w, h)) => {
                                if let (Ok(w), Ok(h)) = (w.parse::<f32>(), h.parse::<f32>()) {
                                    widget.box_size = Some((w, h));
                                    i += 2;
                                } else {
                                    return Err(LispError::new(
                                        format!("размер «{spec}» не разобрать"),
                                        "size 160x32",
                                    ));
                                }
                            }
                            None => {
                                return Err(LispError::new(
                                    format!("размер «{spec}» без высоты"),
                                    "size 160x32",
                                ))
                            }
                        }
                    } else if let (Some(w), Some(h)) = (
                        next.and_then(Value::num),
                        args.get(i + 2).and_then(Value::num),
                    ) {
                        widget.box_size = Some((w as f32, h as f32));
                        i += 3;
                    } else {
                        return Err(LispError::new("у size нет размера", "size 160x32"));
                    }
                }
                Value::Sym(word) if word == "anchor" => {
                    let mut right = false;
                    let mut bottom = false;
                    let mut step = 1;
                    for extra in args.iter().skip(i + 1) {
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
                    widget.anchor = (right, bottom);
                    i += step;
                }
                Value::Sym(key) if key.starts_with(':') => {
                    let name = key[1..].to_string();
                    // :at и :box принимают и пару чисел, и список (10 20).
                    if name == "at" || name == "box" {
                        if let (Some(x), Some(y)) = (
                            args.get(i + 1).and_then(Value::num),
                            args.get(i + 2).and_then(Value::num),
                        ) {
                            if name == "at" {
                                widget.at = Some((x as f32, y as f32));
                            } else {
                                widget.box_size = Some((x as f32, y as f32));
                            }
                            i += 3;
                            continue;
                        }
                    }
                    let value = args.get(i + 1).cloned().unwrap_or(Value::Nil);
                    self.apply_keyword(&name, &value, &mut widget, &mut style)?;
                    i += 2;
                }
                _ => i += 1,
            }
        }
        widget.style = style;
        Ok(Some(widget))
    }

    fn apply_keyword(
        &mut self,
        key: &str,
        value: &Value,
        widget: &mut Widget,
        style: &mut Style,
    ) -> Result<(), LispError> {
        match key {
            "bind" => widget.bind = value.text(),
            // Действие можно задать и строкой — тем же языком, что и в строчной модели.
            "do" => widget.effect = value.text(),
            "at" => {
                if let Value::List(items) = value {
                    if let (Some(x), Some(y)) = (
                        items.first().and_then(Value::num),
                        items.get(1).and_then(Value::num),
                    ) {
                        widget.at = Some((x as f32, y as f32));
                    }
                }
            }
            "box" => {
                if let Value::List(items) = value {
                    if let (Some(w), Some(h)) = (
                        items.first().and_then(Value::num),
                        items.get(1).and_then(Value::num),
                    ) {
                        widget.box_size = Some((w as f32, h as f32));
                    }
                }
            }
            "anchor" => {
                let text = value.text().unwrap_or_default();
                widget.anchor = (
                    text.contains("right"),
                    text.contains("bottom"),
                );
            }
            "size" => style.size = value.num().map(|v| v as f32),
            "bold" => style.bold = matches!(value, Value::Bool(true)),
            "key" => style.key = value.text(),
            "icon" => style.icon = value.text(),
            "color" => style.color = value.text(),
            "pad" => style.pad = value.num().map(|v| v as f32),
            "width" => style.width = value.num().map(|v| v as f32),
            "height" => style.height = value.num().map(|v| v as f32),
            "grow" => style.grow = matches!(value, Value::Bool(true)),
            "align" => {
                let text = value
                    .text()
                    .ok_or_else(|| LispError::new(":align left|center|right", "строка"))?;
                style.align = Some(Align::parse(&text).ok_or_else(|| {
                    LispError::new(format!("align «{text}»"), "left, center, right")
                })?);
            }
            "range" => {
                if let Value::List(items) = value {
                    if let (Some(a), Some(b)) = (
                        items.first().and_then(Value::num),
                        items.get(1).and_then(Value::num),
                    ) {
                        widget.range = Some((a, b));
                    }
                }
            }
            other => {
                return Err(LispError::new(
                    format!("параметр :{other} неизвестен"),
                    ":bind :do :size :bold :color :pad :width :height :align :grow :range :icon :key :at :box :anchor",
                ))
            }
        }
        Ok(())
    }
}

/// Обработчик формы → строка действий модели.
fn handler(form: &Value) -> Result<String, LispError> {
    let Value::List(items) = form else {
        return Err(LispError::new(
            "обработчик должен быть формой",
            "(set! count (+ count 1))",
        ));
    };
    if items.is_empty() {
        return Ok(String::new());
    }
    match items[0].text().as_deref() {
        Some("do") => {
            let mut parts = Vec::new();
            for item in &items[1..] {
                parts.push(handler(item)?);
            }
            Ok(parts.join("; "))
        }
        Some("set!") => {
            let name = items
                .get(1)
                .and_then(Value::text)
                .ok_or_else(|| LispError::new("(set! поле значение)", "имя поля"))?;
            let value = items
                .get(2)
                .ok_or_else(|| LispError::new("(set! поле значение)", "значение"))?;
            // (set! count (+ count 1)) → count += 1
            if let Value::List(inner) = value {
                if let (Some(op), Some(_same), Some(delta)) = (
                    inner.first().and_then(Value::text),
                    inner.get(1).and_then(Value::text),
                    inner.get(2).and_then(Value::num),
                ) {
                    if (op == "+" || op == "-") && delta >= 0.0 {
                        let sign = if op == "+" { "+=" } else { "-=" };
                        return Ok(format!("{name} {sign} {}", model::trim_float(delta as f32)));
                    }
                }
            }
            Ok(format!("{name} = {}", expr_text(value)?))
        }
        Some("append") | Some("remove") | Some("clear") => {
            let word = items[0].text().unwrap_or_default();
            let name = items
                .get(1)
                .and_then(Value::text)
                .ok_or_else(|| LispError::new(format!("({word} список …)"), "имя списка"))?;
            match word.as_str() {
                "clear" => Ok(format!("clear {name}")),
                _ => {
                    let value = items.get(2).ok_or_else(|| {
                        LispError::new(format!("({word} список значение)"), "что положить")
                    })?;
                    // remove ждёт индекс, append — значение
                    Ok(format!("{word} {name} {}", expr_text(value)?))
                }
            }
        }
        Some("if") => {
            let cond = items
                .get(1)
                .ok_or_else(|| LispError::new("(if условие тогда иначе)", "условие"))?;
            let then_code = handler(items.get(2).unwrap_or(&Value::Nil))?;
            let else_code = match items.get(3) {
                Some(form) => format!(" else {{ {} }}", handler(form)?),
                None => String::new(),
            };
            Ok(format!(
                "if {} {{ {} }}{else_code}",
                condition(cond)?,
                then_code
            ))
        }
        other => Err(LispError::new(
            format!("обработчик «{other:?}» неизвестен"),
            "do, set!, if, append, remove, clear",
        )),
    }
}

fn condition(form: &Value) -> Result<String, LispError> {
    let Value::List(items) = form else {
        return Err(LispError::new("условие должно быть формой", "(>= count 9)"));
    };
    let op = items.first().and_then(Value::text).unwrap_or_default();
    let ops = [">", "<", ">=", "<=", "==", "!="];
    if !ops.contains(&op.as_str()) {
        return Err(LispError::new(
            format!("оператор сравнения «{op}» неизвестен"),
            ">, <, >=, <=, ==, !=",
        ));
    }
    let left = expr_text(items.get(1).unwrap_or(&Value::Nil))?;
    let right = expr_text(items.get(2).unwrap_or(&Value::Nil))?;
    Ok(format!("{left} {op} {right}"))
}

fn expr_text(value: &Value) -> Result<String, LispError> {
    match value {
        Value::Num(number) => Ok(model::trim_float(*number as f32)),
        Value::Sym(name) => Ok(name.clone()),
        Value::Str(text) => Ok(format!("\"{text}\"")),
        Value::Bool(flag) => Ok(flag.to_string()),
        Value::List(items) => {
            let op = items.first().and_then(Value::text).unwrap_or_default();
            if !["+", "-", "*", "/"].contains(&op.as_str()) {
                return Err(LispError::new(
                    format!("в выражении «{op}» не арифметика"),
                    "+, -, *, /",
                ));
            }
            let left = expr_text(items.get(1).unwrap_or(&Value::Nil))?;
            let right = expr_text(items.get(2).unwrap_or(&Value::Nil))?;
            Ok(format!("{left} {op} {right}"))
        }
        Value::Nil => Err(LispError::new(
            "пустое выражение",
            "число, строка или имя поля",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"
(window "Магазин" 520 340)
(state count int 0)
(state name string "")
(rust-widget badge "ui.label(\"x\");")
(vbox :pad 12
  (label "Нажатий: {count}" :size 22 :bold true :color "#e5484d")
  (grid 2
    (button "Плюс" (if (>= count 9) (set! count 0) (set! count (+ count 1))))
    (button "Минус" (set! count (- count 1))))
  (text-input "Имя" :bind name :grow true)
  (custom badge))
"##;

    #[test]
    fn reader_handles_strings_comments_and_nesting() {
        let forms = read("(a \"b c\" ; комментарий\n (d 1))").unwrap();
        assert_eq!(forms.len(), 1);
        let Value::List(items) = &forms[0] else {
            panic!("список")
        };
        assert_eq!(items.first().and_then(Value::text).as_deref(), Some("a"));
        assert_eq!(items.get(1).and_then(Value::text).as_deref(), Some("b c"));
    }

    #[test]
    fn sample_becomes_a_screen_with_styles_and_handlers() {
        let forms = read(SAMPLE).unwrap();
        let evaluated = eval(&forms).unwrap();
        let screen = evaluated.screen;
        assert_eq!(screen.title, "Магазин");
        assert_eq!(screen.fields.len(), 2);
        let vbox = &screen.root[0];
        assert_eq!(vbox.style.pad, Some(12.0));
        let label = &vbox.children[0];
        assert_eq!(label.style.size, Some(22.0));
        assert!(label.style.bold);
        assert_eq!(label.style.color.as_deref(), Some("#e5484d"));
        let grid = &vbox.children[1];
        assert_eq!(grid.number, Some(2.0));
        let button = &grid.children[0];
        let effect = button.effect.clone().unwrap_or_default();
        assert!(effect.starts_with("if count >= 9"), "{effect}");
        assert!(effect.contains("count = 0"), "{effect}");
        assert!(effect.contains("count += 1"), "{effect}");
        assert_eq!(grid.children[1].effect.as_deref(), Some("count -= 1"));
        assert_eq!(vbox.children[3].kind, WidgetKind::Custom("badge".into()));
    }

    #[test]
    fn model_text_is_valid_and_reparses() {
        let forms = read(SAMPLE).unwrap();
        let evaluated = eval(&forms).unwrap();
        let text = model::render(&evaluated.screen);
        let reparsed = model::parse(&text).expect("модель из DSL валидна");
        assert_eq!(reparsed.title, "Магазин");
        assert_eq!(reparsed.root[0].style.pad, Some(12.0));
    }

    #[test]
    fn unknown_form_names_the_alternatives() {
        let err = eval(&read("(frobnicate 1)").unwrap()).unwrap_err();
        assert!(err.message.contains("frobnicate"));
        assert!(err.hint.contains("label"));
    }

    #[test]
    fn unknown_keyword_is_refused() {
        let err = eval(&read("(label \"x\" :shade 1)").unwrap()).unwrap_err();
        assert!(err.message.contains("shade"));
    }

    #[test]
    fn nested_do_compiles_to_sequenced_handler() {
        let forms = read(
            "(state count int 0)\n(button \"x\" (do (set! count (+ count 1)) (set! count 0)))",
        )
        .unwrap();
        let screen = eval(&forms).unwrap().screen;
        assert_eq!(
            screen.root[0].effect.as_deref(),
            Some("count += 1; count = 0")
        );
    }

    #[test]
    fn list_forms_become_actions() {
        let forms = crate::lang::read(
            "(window \"Список\" 400 300)\n(state items list)\n(button \"д\" (append items \"молоко\"))\n(button \"о\" (clear items))\n",
        )
        .unwrap();
        let model = crate::lang::eval(&forms).unwrap();
        let effect = model.screen.root[0].effect.clone().unwrap();
        assert!(effect.contains("append items"), "{effect}");
        let clear = model.screen.root[1].effect.clone().unwrap();
        assert_eq!(clear, "clear items");
    }
}
