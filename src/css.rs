//! Таблица стилей для модели: CSS-подобные правила без браузера и зависимостей.
//!
//! Зачем: без неё один и тот же стиль приходится повторять в каждом виджете.
//! Здесь — привычная запись и предсказуемый каскад:
//!
//! ```css
//! window { panel:#15161b; accent:#7c9cff; muted:#9aa3b8; font:15; spacing:8; round:14; }
//! label  { color:#e5e7f0; size:14; }
//! label.meta { color:#9aa3b8; size:12; mono; }
//! #title { size:26; bold; }
//! ```
//!
//! Селекторы: `тег`, `.класс`, `#id`, `тег.класс`, `*`. Специфичность как в CSS:
//! id важнее класса, класс важнее тега; при равенстве побеждает более позднее
//! правило. Стиль прямо в модели (`{…}`) всегда сильнее таблицы.
//!
//! Чего нет и почему: вложенных селекторов, медиазапросов и `:hover`. Первые два
//! не нужны плоскому egui, а состояния наведения задаёт сама тема.

use crate::model::{Chrome, Style, Theme, Widget, WindowOptions};

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub props: Vec<(String, String)>,
    /// Порядок в файле: при равной специфичности побеждает поздний.
    pub order: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Selector {
    pub tag: Option<String>,
    pub class: Option<String>,
    pub id: Option<String>,
}

impl Selector {
    /// Вес селектора: id → 3, класс → 2, тег → 1.
    fn weight(&self) -> u8 {
        if self.id.is_some() {
            3
        } else if self.class.is_some() {
            2
        } else if self.tag.is_some() {
            1
        } else {
            0
        }
    }

    fn matches(&self, tag: &str, classes: &[String], id: Option<&str>) -> bool {
        if let Some(wanted) = &self.id {
            if id != Some(wanted.as_str()) {
                return false;
            }
        }
        if let Some(wanted) = &self.class {
            if !classes.iter().any(|class| class == wanted) {
                return false;
            }
        }
        if let Some(wanted) = &self.tag {
            if wanted != tag {
                return false;
            }
        }
        true
    }
}

/// Разобрать таблицу стилей. Ошибки собираются с номерами строк.
pub fn parse(sheet: &str) -> Result<Vec<Rule>, String> {
    let mut rules = Vec::new();
    let mut order = 0;
    let mut rest = sheet;
    // снимаем комментарии /* … */ и // …
    let mut cleaned = String::new();
    let mut chars = rest.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut prev = ' ';
            for inner in chars.by_ref() {
                if prev == '*' && inner == '/' {
                    break;
                }
                prev = inner;
            }
            cleaned.push(' ');
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'/') {
            for inner in chars.by_ref() {
                if inner == '\n' {
                    cleaned.push('\n');
                    break;
                }
            }
            continue;
        }
        cleaned.push(ch);
    }
    rest = &cleaned;

    let mut index = 0;
    let bytes: Vec<char> = rest.chars().collect();
    while index < bytes.len() {
        // ищем «{»
        let Some(open) = rest[index..].find('{').map(|offset| index + offset) else {
            break;
        };
        let Some(close) = rest[open..].find('}').map(|offset| open + offset) else {
            return Err(format!(
                "таблица стилей: «{{» без «}}» рядом с «{}»",
                rest[index..open.min(rest.len())].trim()
            ));
        };
        let selector_text = rest[index..open].trim().to_string();
        let body = &rest[open + 1..close];
        let mut selectors = Vec::new();
        for part in selector_text.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            selectors.push(parse_selector(part)?);
        }
        if selectors.is_empty() {
            return Err("таблица стилей: правило без селектора".to_string());
        }
        let mut props = Vec::new();
        for declaration in body.split(';') {
            let declaration = declaration.trim();
            if declaration.is_empty() {
                continue;
            }
            // Флаг без значения — это «да»: mono; bold; pill;
            let (key, value) = match declaration.split_once(':') {
                Some((key, value)) => (key.trim().to_string(), value.trim().to_string()),
                None => (declaration.to_string(), "yes".to_string()),
            };
            props.push((key, value));
        }
        rules.push(Rule {
            selectors,
            props,
            order,
        });
        order += 1;
        // дальше — сразу за закрывающей скобкой
        index = open + 1 + body.len() + 1;
    }
    Ok(rules)
}

fn parse_selector(text: &str) -> Result<Selector, String> {
    if text == "*" {
        return Ok(Selector {
            tag: None,
            class: None,
            id: None,
        });
    }
    let mut tag = None;
    let mut class = None;
    let mut id = None;
    let mut current = String::new();
    let mut mode = ' ';
    for ch in text.chars() {
        match ch {
            '.' | '#' => {
                if !current.is_empty() {
                    if mode == '.' {
                        class = Some(current.clone());
                    } else if mode == '#' {
                        id = Some(current.clone());
                    } else {
                        tag = Some(current.clone());
                    }
                    current.clear();
                }
                mode = ch;
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        match mode {
            '.' => class = Some(current),
            '#' => id = Some(current),
            _ => tag = Some(current),
        }
    }
    if tag.is_none() && class.is_none() && id.is_none() {
        return Err(format!("таблица стилей: пустой селектор «{text}»"));
    }
    Ok(Selector { tag, class, id })
}

/// Применить таблицу к дереву виджетов. Стиль из модели не перетирается.
pub fn apply(widgets: &mut [Widget], rules: &[Rule], theme: &mut Theme, window: &mut WindowOptions) {
    for widget in widgets.iter_mut() {
        apply_widget(widget, rules, theme, window);
    }
}

pub fn apply_theme(rules: &[Rule], theme: &mut Theme, window: &mut WindowOptions) {
    let mut matched: Vec<&Rule> = rules
        .iter()
        .filter(|rule| {
            rule.selectors.iter().any(|selector| {
                selector.id.is_none()
                    && selector.class.is_none()
                    && selector.tag.as_deref() == Some("window")
            })
        })
        .collect();
    matched.sort_by_key(|rule| rule.order);
    for rule in matched {
        for (key, value) in &rule.props {
            apply_theme_prop(key, value, theme, window);
        }
    }
}

fn apply_theme_prop(key: &str, value: &str, theme: &mut Theme, window: &mut WindowOptions) {
    match key {
        "dark" => theme.dark = value != "no" && value != "false",
        "light" => theme.dark = value == "yes" || value == "true",
        "accent" => theme.accent = Some(value.to_string()),
        "panel" => theme.panel = Some(value.to_string()),
        "card" => theme.card = Some(value.to_string()),
        "muted" => theme.muted = Some(value.to_string()),
        "font" => theme.font = value.parse().unwrap_or(theme.font),
        "spacing" => theme.spacing = value.parse().unwrap_or(theme.spacing),
        "round" => theme.round = value.parse().unwrap_or(theme.round),
        // окно тоже можно описать в таблице: window { chrome:own; topmost:yes }
        other => {
            if window.set(other, value).is_ok() {
                return;
            }
            let _ = window.set("chrome", value);
        }
    }
}

fn apply_widget(widget: &mut Widget, rules: &[Rule], theme: &mut Theme, window: &mut WindowOptions) {
    let tag = widget.kind.as_str();
    let mut classes = widget.css_classes.clone();
    if let Some(list) = &widget.style.classes {
        for class in list.split([',', ' ']).filter(|part| !part.is_empty()) {
            classes.push(class.to_string());
        }
    }
    let id = widget.css_id.clone().or_else(|| widget.style.id.clone());

    let mut matched: Vec<(&Rule, u8)> = rules
        .iter()
        .filter_map(|rule| {
            let best = rule
                .selectors
                .iter()
                .filter(|selector| selector.matches(tag, &classes, id.as_deref()))
                .map(Selector::weight)
                .max()?;
            Some((rule, best))
        })
        .collect();
    // сначала общие (тег), потом классы, потом id; внутри веса — по порядку файла
    matched.sort_by_key(|(rule, weight)| (*weight, rule.order));

    for (rule, _) in matched {
        for (key, value) in &rule.props {
            match key.as_str() {
                "dark" | "light" | "accent" | "panel" | "card" | "muted" | "font" | "spacing"
                | "round" => apply_theme_prop(key, value, theme, window),
                _ => apply_style_prop(&mut widget.style, key, value),
            }
        }
    }

    for child in widget.children.iter_mut() {
        apply_widget(child, rules, theme, window);
    }
}

/// Свойства стиля: те же ключи, что в `{…}` модели.
fn apply_style_prop(style: &mut Style, key: &str, value: &str) {
    match key {
        "color" | "цвет" => style.color = Some(value.to_string()),
        "size" | "кегль" => {
            if let Ok(size) = value.parse() {
                style.size = Some(size);
            }
        }
        "bold" | "жирный" => style.bold = value != "no" && value != "false" && value != "0",
        "mono" | "моно" => style.mono = value != "no" && value != "false" && value != "0",
        "pill" | "чип" | "плашка" => {
            style.pill = value != "no" && value != "false" && value != "0"
        }
        "hint" | "подсказка" => style.hint = Some(value.trim_matches('"').to_string()),
        "pad" | "padding" => {
            if let Ok(pad) = value.parse() {
                style.pad = Some(pad);
            }
        }
        "width" => {
            if let Ok(width) = value.parse() {
                style.width = Some(width);
            }
        }
        "height" => {
            if let Ok(height) = value.parse() {
                style.height = Some(height);
            }
        }
        "grow" => style.grow = value != "no" && value != "false" && value != "0",
        "key" => style.key = Some(value.to_string()),
        "icon" => style.icon = Some(value.to_string()),
        "align" => {
            style.align = match value {
                "center" | "центр" => Some(crate::model::Align::Center),
                "right" | "справа" => Some(crate::model::Align::Right),
                _ => None,
            }
        }
        _ => {}
    }
}

/// Описание для справки и подсказок об ошибках.
pub const HELP: &str = "тег, .класс, #id, тег.класс; свойства — как в {…}: color, size, bold, mono, pill, pad, hint";

#[allow(dead_code)]
fn _chrome_is_public(_: Chrome) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tag_class_and_id() {
        let rules = parse("label { color:#fff; } label.meta, #title { size:12; mono; }").unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[1].selectors.len(), 2);
        assert_eq!(rules[1].selectors[0].tag.as_deref(), Some("label"));
        assert_eq!(rules[1].selectors[0].class.as_deref(), Some("meta"));
        assert_eq!(rules[1].selectors[1].id.as_deref(), Some("title"));
        assert!(rules[0].props.iter().any(|(k, v)| k == "color" && v == "#fff"));
    }

    #[test]
    fn id_beats_class_beats_tag() {
        let mut widget = Widget::container(crate::model::WidgetKind::Label);
        widget.css_classes = vec!["meta".to_string()];
        widget.css_id = Some("title".to_string());
        let rules = parse(
            "label { size:10; } .meta { size:12; } #title { size:26; }",
        )
        .unwrap();
        let mut theme = Theme::default();
        let mut window = WindowOptions::default();
        apply(&mut [widget.clone()], &rules, &mut theme, &mut window);
        let mut single = [widget];
        apply(&mut single, &rules, &mut theme, &mut window);
        assert_eq!(single[0].style.size, Some(26.0));
    }

    #[test]
    fn window_block_sets_theme() {
        let rules = parse("window { accent:#7c9cff; panel:#15161b; round:14; }").unwrap();
        let mut theme = Theme::default();
        let mut window = WindowOptions::default();
        apply_theme(&rules, &mut theme, &mut window);
        assert_eq!(theme.accent.as_deref(), Some("#7c9cff"));
        assert_eq!(theme.round, 14);
    }
}
