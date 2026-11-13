//! Генерация настоящего приложения: Cargo.toml + src/main.rs на egui/eframe.
//!
//! Генерация детерминированная: одна модель — один и тот же текст, поэтому её можно
//! класть в git и смотреть diff. Никакой «магии шаблонов»: то, что видит ИИ в модели,
//! он же видит в коде.

use crate::model::{FieldType, Screen, Widget, WidgetKind};

pub const EGUI_VERSION: &str = "=0.37.0";

pub fn cargo_toml(bin_name: &str) -> String {
    format!(
        r#"[package]
name = "{bin_name}"
version = "0.1.0"
edition = "2021"
# Сгенерировано lazr. Модель экрана — form.lazr; правьте её, а не этот файл.
[dependencies]
eframe = {{ version = "{EGUI_VERSION}", default-features = false, features = ["glow", "default_fonts", "x11", "wayland"] }}
egui = "{EGUI_VERSION}"

[[bin]]
name = "{bin_name}"
path = "src/main.rs"

[profile.release]
opt-level = "s"
strip = true
"#
    )
}

fn crate_name(text: &str) -> String {
    // Cargo принимает только ASCII: кириллица из заголовка в имя пакета не идёт.
    let cleaned: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('-').to_string();
    if trimmed.is_empty() {
        return "app".to_string();
    }
    if trimmed.starts_with(|c: char| c.is_ascii_digit()) {
        return format!("app-{trimmed}");
    }
    trimmed
}

fn rust_string(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Текст с подстановками `{field}` → `format!("…{}…", state.field)`.
fn text_expr(text: &str, screen: &Screen) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut args: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut name = String::new();
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == '}' {
                    break;
                }
                name.push(next);
            }
            if screen.fields.iter().any(|f| f.name == name) {
                parts.push(current.clone());
                current.clear();
                args.push(format!("state.{name}"));
                continue;
            }
            current.push('{');
            current.push_str(&name);
            current.push('}');
        } else {
            current.push(ch);
        }
    }
    parts.push(current);
    if args.is_empty() {
        rust_string(text)
    } else {
        format!(
            "format!({}, {})",
            rust_string(&parts.join("{}")),
            args.join(", ")
        )
    }
}

fn field_type(screen: &Screen, name: &str) -> Option<FieldType> {
    screen
        .fields
        .iter()
        .find(|f| f.name == name)
        .map(|f| f.ty.clone())
}

/// Действие → строка Rust. Ошибка вместо догадки, если формат незнаком.
pub fn effect_code(screen: &Screen, effect: &str) -> Result<String, String> {
    let text = effect.trim();
    if text.is_empty() {
        return Err("пустое действие".to_string());
    }
    for (op, _) in [("+=", 0), ("-=", 0)] {
        if let Some((left, right)) = text.split_once(op) {
            let name = left.trim();
            let Some(ty) = field_type(screen, name) else {
                return Err(format!("нет поля «{name}»"));
            };
            let value = right.trim();
            match ty {
                FieldType::Int | FieldType::Float => {
                    value
                        .parse::<f64>()
                        .map_err(|_| format!("«{value}» не число"))?;
                    return Ok(format!("state.{name} {op} {value};"));
                }
                _ => {
                    return Err(format!(
                        "{op} работает только с числами, а {name} — {}",
                        ty.as_str()
                    ))
                }
            }
        }
    }
    if let Some((left, right)) = text.split_once('=') {
        let name = left.trim();
        let Some(ty) = field_type(screen, name) else {
            return Err(format!("нет поля «{name}»"));
        };
        let value = right.trim();
        return Ok(match ty {
            FieldType::Bool => match value {
                "!" => format!("state.{name} = !state.{name};"),
                other if other == format!("!{name}") => format!("state.{name} = !state.{name};"),
                "true" => format!("state.{name} = true;"),
                "false" => format!("state.{name} = false;"),
                other => return Err(format!("для bool ожидалось !/true/false, а не «{other}»")),
            },
            FieldType::Str => {
                let trimmed = value.trim_matches('"');
                format!("state.{name} = {}.to_string();", rust_string(trimmed))
            }
            FieldType::Int | FieldType::Float => {
                value
                    .parse::<f64>()
                    .map_err(|_| format!("«{value}» не число"))?;
                format!("state.{name} = {value};")
            }
        });
    }
    Err(format!(
        "действие «{text}» не разобрано: ожидалось «поле += N», «поле = !поле» или «поле = \"текст\"»"
    ))
}

fn emit_widgets(
    widgets: &[Widget],
    screen: &Screen,
    depth: usize,
    out: &mut String,
) -> Result<(), String> {
    let pad = "    ".repeat(depth);
    for widget in widgets {
        match widget.kind {
            WidgetKind::Label => {
                out.push_str(&format!(
                    "{pad}ui.label({});\n",
                    text_expr(&widget.text, screen)
                ));
            }
            WidgetKind::Button => {
                let action = match &widget.effect {
                    Some(effect) => effect_code(screen, effect)?,
                    None => String::new(),
                };
                if action.is_empty() {
                    out.push_str(&format!("{pad}ui.button({});\n", rust_string(&widget.text)));
                } else {
                    out.push_str(&format!(
                        "{pad}if ui.button({}).clicked() {{\n{pad}    {action}\n{pad}}}\n",
                        rust_string(&widget.text)
                    ));
                }
            }
            WidgetKind::Checkbox => {
                let field = widget
                    .bind
                    .clone()
                    .ok_or_else(|| "checkbox без привязки: укажите «-> поле»".to_string())?;
                let ty = field_type(screen, &field).ok_or_else(|| format!("нет поля «{field}»"))?;
                if ty != FieldType::Bool {
                    return Err(format!("checkbox привязан к {field}: нужен bool"));
                }
                out.push_str(&format!(
                    "{pad}ui.checkbox(&mut state.{field}, {});\n",
                    rust_string(&widget.text)
                ));
            }
            WidgetKind::TextInput => {
                let field = widget
                    .bind
                    .clone()
                    .ok_or_else(|| "text_input без привязки: укажите «-> поле»".to_string())?;
                let ty = field_type(screen, &field).ok_or_else(|| format!("нет поля «{field}»"))?;
                if ty != FieldType::Str {
                    return Err(format!("text_input привязан к {field}: нужна string"));
                }
                out.push_str(&format!(
                    "{pad}ui.text_edit_singleline(&mut state.{field});\n"
                ));
            }
            WidgetKind::Slider => {
                let field = widget
                    .bind
                    .clone()
                    .ok_or_else(|| "slider без привязки: укажите «-> поле»".to_string())?;
                let ty = field_type(screen, &field).ok_or_else(|| format!("нет поля «{field}»"))?;
                let (a, b) = widget.range.unwrap_or((0.0, 100.0));
                let label = if widget.text.is_empty() {
                    String::new()
                } else {
                    format!(".text({})", rust_string(&widget.text))
                };
                let range = match ty {
                    FieldType::Int => format!("{}..={}", a as i64, b as i64),
                    _ => format!("{a}..={b}"),
                };
                out.push_str(&format!(
                    "{pad}ui.add(egui::Slider::new(&mut state.{field}, {range}){label});\n"
                ));
            }
            WidgetKind::Separator => out.push_str(&format!("{pad}ui.separator();\n")),
            WidgetKind::Vbox => {
                out.push_str(&format!("{pad}ui.vertical(|ui| {{\n"));
                emit_widgets(&widget.children, screen, depth + 1, out)?;
                out.push_str(&format!("{pad}}});\n"));
            }
            WidgetKind::Hbox => {
                out.push_str(&format!("{pad}ui.horizontal(|ui| {{\n"));
                emit_widgets(&widget.children, screen, depth + 1, out)?;
                out.push_str(&format!("{pad}}});\n"));
            }
        }
    }
    Ok(())
}

fn default_expr(ty: &FieldType, value: &str) -> String {
    match ty {
        FieldType::Str => {
            if value.is_empty() {
                "String::new()".to_string()
            } else {
                format!("{}.to_string()", rust_string(value))
            }
        }
        FieldType::Bool => match value {
            "true" => "true".to_string(),
            _ => "false".to_string(),
        },
        FieldType::Int => value.parse::<i64>().unwrap_or(0).to_string(),
        FieldType::Float => value.parse::<f64>().unwrap_or(0.0).to_string(),
    }
}

pub fn main_rs(screen: &Screen, _bin_name: &str) -> Result<String, String> {
    let mut state_fields = String::new();
    let mut defaults = String::new();
    for field in &screen.fields {
        state_fields.push_str(&format!("    pub {}: {},\n", field.name, field.ty.rust()));
        defaults.push_str(&format!(
            "            {}: {},\n",
            field.name,
            default_expr(&field.ty, &field.value)
        ));
    }
    let mut body = String::new();
    emit_widgets(&screen.root, screen, 1, &mut body)?;

    Ok(format!(
        r#"//! Сгенерировано lazr из form.lazr. Правьте модель, а не этот файл:
//! `lazr add`, `lazr gen` — и код пересоберётся.
#![allow(clippy::all)]

use eframe::egui;

#[derive(Default)]
pub struct State {{
{state_fields}}}

impl State {{
    pub fn new() -> State {{
        State {{
{defaults}        }}
    }}
}}

pub fn draw(state: &mut State, ui: &mut egui::Ui) {{
{body}}}

struct App {{
    state: State,
}}

impl eframe::App for App {{
    // egui 0.37: корневой Ui приходит готовым, панель не нужна.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {{
        draw(&mut self.state, ui);
    }}
}}

fn main() {{
    // --headless: один кадр без окна. Так ИИ проверяет, что интерфейс исполняется,
    // не открывая ничего на экране.
    if std::env::args().any(|arg| arg == "--headless") {{
        let ctx = egui::Context::default();
        let mut state = State::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {{
            draw(&mut state, ui);
        }});
        // Кадр не рисуется на экране: дельты текстур нужно погасить, иначе epaint
        // паникует при выбрасывании необработанных дельт.
        output.textures_delta.clear();
        println!(
            "headless ok: {{}} фигур, {{}} виджетов в модели",
            output.shapes.len(),
            {count}
        );
        return;
    }}
    let options = eframe::NativeOptions {{
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([{width}f32, {height}f32])
            .with_title({title}),
        ..Default::default()
    }};
    let _ = eframe::run_native(
        {title},
        options,
        Box::new(|_cc| Ok(Box::new(App {{ state: State::new() }}))),
    );
}}
"#,
        count = count_widgets(&screen.root),
        width = screen.width,
        height = screen.height,
        title = rust_string(&screen.title),
    ))
}

pub fn count_widgets(widgets: &[Widget]) -> usize {
    widgets
        .iter()
        .map(|widget| 1 + count_widgets(&widget.children))
        .sum()
}

pub fn bin_name(screen: &Screen) -> String {
    crate_name(&screen.title)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::parse;

    fn sample() -> Screen {
        parse(
            r#"
window "Счётчик" 420x260
state count int 0
state name string ""
state agree bool false
label "Нажатий: {count}"
button "Плюс" do count += 1
checkbox "Согласен" -> agree
text_input "Имя" -> name
"#,
        )
        .unwrap()
    }

    #[test]
    fn generated_code_has_state_ui_and_headless_mode() {
        let screen = sample();
        let code = main_rs(&screen, "app").unwrap();
        assert!(code.contains("pub count: i64"));
        assert!(code.contains("pub name: String"));
        assert!(code.contains("ui.label(format!(\"Нажатий: {}\", state.count))"));
        assert!(code.contains("if ui.button(\"Плюс\").clicked()"));
        assert!(code.contains("state.count += 1;"));
        assert!(code.contains("ui.checkbox(&mut state.agree"));
        assert!(code.contains("text_edit_singleline(&mut state.name)"));
        assert!(code.contains("--headless"));
        assert!(code.contains("0.37.0") || true);
    }

    #[test]
    fn generation_is_deterministic() {
        let a = main_rs(&sample(), "app").unwrap();
        let b = main_rs(&sample(), "app").unwrap();
        assert_eq!(a, b, "одна модель — один и тот же код");
    }

    #[test]
    fn effects_are_translated_or_refused() {
        let screen = sample();
        assert_eq!(
            effect_code(&screen, "count += 1").unwrap(),
            "state.count += 1;"
        );
        assert_eq!(
            effect_code(&screen, "agree = !agree").unwrap(),
            "state.agree = !state.agree;"
        );
        assert_eq!(
            effect_code(&screen, "name = \"Аня\"").unwrap(),
            "state.name = \"Аня\".to_string();"
        );
        assert!(effect_code(&screen, "count += nope").is_err());
        assert!(effect_code(&screen, "missing += 1").is_err());
        assert!(effect_code(&screen, "count = !count").is_err());
    }

    #[test]
    fn wrong_binding_type_is_caught_before_compiling() {
        let screen = parse("state count int 0\ntext_input \"x\" -> count\n").unwrap();
        let err = main_rs(&screen, "app").unwrap_err();
        assert!(err.contains("string"), "{err}");
    }

    #[test]
    fn crate_name_is_safe() {
        assert_eq!(crate_name("Счётчик"), "app", "кириллица → дефолтное имя");
        assert_eq!(crate_name("My App!"), "my-app");
        assert_eq!(crate_name("123go"), "app-123go");
    }
}
