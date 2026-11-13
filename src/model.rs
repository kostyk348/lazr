//! Модель экрана: декларативное описание, из которого генерируется настоящий Rust.
//!
//! Формат нарочно текстовый и строчный: ИИ правит его семантическими операциями,
//! а не пикселями, и каждая ошибка несёт номер строки и подсказку.
//!
//! ```text
//! window "Счётчик" 420x260
//! state count int 0
//! state name string ""
//! vbox
//!   label "Нажатий: {count}"
//!   button "Плюс" do count += 1
//!   text_input "Имя" -> name
//! ```

#![allow(dead_code)]

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
    /// Rust-тип поля.
    pub fn rust(&self) -> &'static str {
        match self {
            FieldType::Int => "i64",
            FieldType::Float => "f64",
            FieldType::Bool => "bool",
            FieldType::Str => "String",
        }
    }
    /// Литерал по умолчанию, когда значение не задано.
    pub fn default_lit(&self) -> &'static str {
        match self {
            FieldType::Int => "0",
            FieldType::Float => "0.0",
            FieldType::Bool => "false",
            FieldType::Str => "String::new()",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: FieldType,
    /// Литерал значения (для строк — без кавычек).
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WidgetKind {
    Label,
    Button,
    Checkbox,
    TextInput,
    Slider,
    Separator,
    Vbox,
    Hbox,
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
            WidgetKind::Vbox => "vbox",
            WidgetKind::Hbox => "hbox",
        }
    }
    pub fn parse(text: &str) -> Option<WidgetKind> {
        Some(match text {
            "label" => WidgetKind::Label,
            "button" => WidgetKind::Button,
            "checkbox" => WidgetKind::Checkbox,
            "text_input" => WidgetKind::TextInput,
            "slider" => WidgetKind::Slider,
            "separator" => WidgetKind::Separator,
            "vbox" => WidgetKind::Vbox,
            "hbox" => WidgetKind::Hbox,
            _ => return None,
        })
    }
    pub fn is_container(&self) -> bool {
        matches!(self, WidgetKind::Vbox | WidgetKind::Hbox)
    }
    pub fn all() -> Vec<&'static str> {
        vec![
            "label",
            "button",
            "checkbox",
            "text_input",
            "slider",
            "separator",
            "vbox",
            "hbox",
        ]
    }
}

#[derive(Debug, Clone)]
pub struct Widget {
    pub kind: WidgetKind,
    /// Текст/подпись виджета.
    pub text: String,
    /// Действие на нажатие: `count += 1`, `agree = !agree`, `name = "x"`.
    pub effect: Option<String>,
    /// Привязка к полю состояния (checkbox, text_input, slider).
    pub bind: Option<String>,
    /// Диапазон ползунка.
    pub range: Option<(f64, f64)>,
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
            children: Vec::new(),
        }
    }
    pub fn container(kind: WidgetKind) -> Widget {
        Widget::leaf(kind, "")
    }
}

#[derive(Debug, Clone)]
pub struct Screen {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub fields: Vec<Field>,
    pub root: Vec<Widget>,
}

/// Ошибка модели: всегда с номером строки, кодом и подсказкой.
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
            "{{\"ok\":false,\"error\":{{\"line\":{},\"code\":\"{}\",\"message\":{},\"hint\":{}}}}}",
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

fn tokens(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut escaped = false;
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
        match ch {
            '\\' if quoted => escaped = true,
            '"' => {
                if quoted {
                    out.push(current.clone());
                    current.clear();
                    quoted = false;
                } else {
                    quoted = true;
                }
            }
            c if c.is_whitespace() && !quoted => {
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

pub fn parse(text: &str) -> Result<Screen, Vec<ModelError>> {
    let mut errors: Vec<ModelError> = Vec::new();
    let mut screen = Screen {
        title: "Окно".to_string(),
        width: 480,
        height: 320,
        fields: Vec::new(),
        root: Vec::new(),
    };
    // Стек вложенности: (глубина, путь в дереве).
    let mut stack: Vec<(usize, Vec<usize>)> = vec![(0, Vec::new())];

    for (index, raw) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.trim_end();
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let depth = indent / 2;
        let words = tokens(line.trim_start());
        let Some(head) = words.first() else { continue };

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
                let Some(kind) = WidgetKind::parse(head) else {
                    errors.push(ModelError {
                        line: line_no,
                        code: "UNKNOWN_WIDGET",
                        message: format!("виджет «{head}» неизвестен"),
                        hint: format!("доступны: {}", WidgetKind::all().join(", ")),
                    });
                    continue;
                };
                // найти контейнер по глубине
                while stack.len() > 1 && stack.last().map(|(d, _)| *d >= depth).unwrap_or(false) {
                    stack.pop();
                }
                let mut widget = if kind.is_container() {
                    Widget::container(kind.clone())
                } else {
                    Widget::leaf(kind.clone(), words.get(1).map(String::as_str).unwrap_or(""))
                };

                let mut rest = &words[2.min(words.len())..];
                if !kind.is_container() {
                    rest = &words[1.min(words.len())..];
                }
                let mut i = 0;
                while i < rest.len() {
                    match rest[i].as_str() {
                        "do" => {
                            let effect: Vec<&str> =
                                rest[i + 1..].iter().map(String::as_str).collect();
                            widget.effect = Some(effect.join(" "));
                            break;
                        }
                        "->" => {
                            widget.bind = rest.get(i + 1).cloned();
                            i += 2;
                        }
                        token if token.contains("..") => {
                            if let Some((a, b)) = token.split_once("..") {
                                if let (Ok(a), Ok(b)) = (a.parse(), b.parse()) {
                                    widget.range = Some((a, b));
                                }
                            }
                            i += 1;
                        }
                        _ => i += 1,
                    }
                }

                if widget.kind.is_container() {
                    let path = stack.last().map(|(_, p)| p.clone()).unwrap_or_default();
                    let child_index = child_list(&mut screen.root, &path).len();
                    child_list(&mut screen.root, &path).push(widget);
                    let mut child_path = path.clone();
                    child_path.push(child_index);
                    stack.push((depth, child_path));
                } else {
                    let path = stack.last().map(|(_, p)| p.clone()).unwrap_or_default();
                    child_list(&mut screen.root, &path).push(widget);
                }
            }
        }
    }

    // Проверки ссылок: поля, привязки, эффекты.
    let field_names: Vec<String> = screen.fields.iter().map(|f| f.name.clone()).collect();
    fn walk(widgets: &Vec<Widget>, field_names: &[String], errors: &mut Vec<ModelError>) {
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
                let field = effect.split_whitespace().next().unwrap_or("");
                if !field_names.contains(&field.to_string()) {
                    errors.push(ModelError {
                        line: 0,
                        code: "EFFECT_UNKNOWN_FIELD",
                        message: format!("действие меняет неизвестное поле «{field}»"),
                        hint: "поле должно быть объявлено через state".into(),
                    });
                }
            }
            walk(&widget.children, field_names, errors);
        }
    }
    walk(&screen.root, &field_names, &mut errors);

    if errors.is_empty() {
        Ok(screen)
    } else {
        Err(errors)
    }
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
    render_widgets(&screen.root, 0, &mut out);
    out
}

fn render_widgets(widgets: &[Widget], depth: usize, out: &mut String) {
    for widget in widgets {
        let pad = "  ".repeat(depth);
        let mut line = format!("{pad}{}", widget.kind.as_str());
        if !widget.text.is_empty() {
            line.push(' ');
            line.push_str(&quote(&widget.text));
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
        out.push_str(&line);
        out.push('\n');
        render_widgets(&widget.children, depth + 1, out);
    }
}

pub fn quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Добавить виджет в контейнер (по умолчанию — в корень или в vbox).
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
            hint: "vbox/hbox, существующий в модели".into(),
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
state agree bool false
vbox
  label "Нажатий: {count}"
  button "Плюс" do count += 1
  hbox
    checkbox "Согласен" -> agree
    text_input "Имя" -> name
  label "Привет, {name}"
"#;

    #[test]
    fn sample_parses_into_a_tree() {
        let screen = parse(SAMPLE).expect("модель валидна");
        assert_eq!(screen.title, "Счётчик");
        assert_eq!((screen.width, screen.height), (420, 260));
        assert_eq!(screen.fields.len(), 3);
        assert_eq!(screen.root.len(), 1);
        let vbox = &screen.root[0];
        assert_eq!(vbox.kind, WidgetKind::Vbox);
        assert_eq!(vbox.children.len(), 4);
        assert_eq!(vbox.children[2].kind, WidgetKind::Hbox);
        assert_eq!(vbox.children[2].children.len(), 2);
    }

    #[test]
    fn roundtrip_keeps_the_meaning() {
        let screen = parse(SAMPLE).unwrap();
        let again = parse(&render(&screen)).unwrap();
        assert_eq!(again.title, screen.title);
        assert_eq!(again.fields.len(), screen.fields.len());
        assert_eq!(again.root.len(), screen.root.len());
        assert_eq!(
            again.root[0].children[1].effect.as_deref(),
            Some("count += 1")
        );
        assert_eq!(
            again.root[0].children[2].children[1].bind.as_deref(),
            Some("name")
        );
    }

    #[test]
    fn unknown_widget_names_the_line_and_offers_the_list() {
        let broken = "window \"x\"\nvbox\n  frobnicator \"a\"\n";
        let errors = parse(broken).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "UNKNOWN_WIDGET");
        assert_eq!(errors[0].line, 3, "номер строки важен для ИИ");
        assert!(
            errors[0].hint.contains("label"),
            "подсказка перечисляет виджеты"
        );
        assert!(errors[0].to_json().contains("\"code\":\"UNKNOWN_WIDGET\""));
    }

    #[test]
    fn bind_to_unknown_field_is_refused() {
        let broken = "state count int 0\ncheckbox \"x\" -> missing\n";
        let errors = parse(broken).unwrap_err();
        assert!(errors.iter().any(|e| e.code == "NO_SUCH_FIELD"));
    }

    #[test]
    fn effect_on_unknown_field_is_refused() {
        let broken = "state count int 0\nbutton \"x\" do nope += 1\n";
        let errors = parse(broken).unwrap_err();
        assert!(errors.iter().any(|e| e.code == "EFFECT_UNKNOWN_FIELD"));
    }

    #[test]
    fn add_widget_goes_into_the_named_container() {
        let mut screen = parse(SAMPLE).unwrap();
        let widget = Widget::leaf(WidgetKind::Button, "Минус");
        add_widget(&mut screen, Some("hbox"), widget).unwrap();
        assert_eq!(screen.root[0].children[2].children.len(), 3);
        assert!(add_widget(
            &mut screen,
            Some("grid"),
            Widget::leaf(WidgetKind::Label, "x")
        )
        .is_err());
    }

    #[test]
    fn quoted_text_with_spaces_survives() {
        let screen = parse("window \"Мой счётчик\"\nlabel \"Привет, мир\"\n").unwrap();
        assert_eq!(screen.title, "Мой счётчик");
        assert_eq!(screen.root[0].text, "Привет, мир");
    }
}
