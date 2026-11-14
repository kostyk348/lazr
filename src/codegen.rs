//! Генерация настоящего приложения: Cargo.toml + src/main.rs на egui/eframe.
//!
//! Генерация детерминированная: одна модель — один и тот же текст, поэтому её
//! можно класть в git и смотреть diff.

use crate::expr;
use crate::model::{Align, FieldType, RustKind, Screen, Widget, WidgetKind};

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
egui = {{ version = "{EGUI_VERSION}", features = ["monochrome_emoji_fonts"] }}

[[bin]]
name = "{bin_name}"
path = "src/main.rs"

[profile.release]
opt-level = "s"
strip = true
"#
    )
}

/// Кириллица → латиница: иначе русский заголовок даёт пакет «app» у всех.
fn transliterate(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        // to_ascii_lowercase не трогает кириллицу — нужен юникодный вариант.
        let lower = ch.to_lowercase().next().unwrap_or(ch);
        let mapped: &str = match lower {
            'а' => "a",
            'б' => "b",
            'в' => "v",
            'г' => "g",
            'д' => "d",
            'е' | 'ё' => "e",
            'ж' => "zh",
            'з' => "z",
            'и' => "i",
            'й' => "y",
            'к' => "k",
            'л' => "l",
            'м' => "m",
            'н' => "n",
            'о' => "o",
            'п' => "p",
            'р' => "r",
            'с' => "s",
            'т' => "t",
            'у' => "u",
            'ф' => "f",
            'х' => "h",
            'ц' => "c",
            'ч' => "ch",
            'ш' => "sh",
            'щ' => "sch",
            'ъ' | 'ь' => "",
            'ы' => "y",
            'э' => "e",
            'ю' => "yu",
            'я' => "ya",
            other => {
                if other.is_ascii_alphanumeric() {
                    out.push(other);
                } else {
                    out.push('-');
                }
                continue;
            }
        };
        out.push_str(mapped);
    }
    out
}

pub fn crate_name(text: &str) -> String {
    // Cargo принимает только ASCII: кириллицу транслитерируем, остальное — в дефис.
    let cleaned: String = transliterate(text)
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

pub fn bin_name(screen: &Screen) -> String {
    crate_name(&screen.title)
}

/// Литерал f32: у Rust `8` — целое, а `8.0` — число с плавающей точкой.
fn float_lit(value: f32) -> String {
    if value.fract().abs() < f32::EPSILON {
        format!("{:.1}", value)
    } else {
        format!("{value}")
    }
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

fn field_type(screen: &Screen, name: &str) -> Option<FieldType> {
    screen
        .fields
        .iter()
        .find(|f| f.name == name)
        .map(|f| f.ty.clone())
}

fn text_expr(text: &str, screen: &Screen) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut args: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut name = String::new();
            let mut closed = false;
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == '}' {
                    closed = true;
                    break;
                }
                name.push(next);
            }
            if closed {
                if let Some(field) = screen.fields.iter().find(|f| f.name == name) {
                    parts.push(current.clone());
                    current.clear();
                    // у списка в тексте печатаем количество — иначе в format! уходит Vec
                    args.push(match field.ty {
                        FieldType::List => format!("state.{name}.len()"),
                        _ => format!("state.{name}"),
                    });
                    continue;
                }
            }
            current.push('{');
            current.push_str(&name);
            if closed {
                current.push('}');
            }
        } else {
            current.push(ch);
        }
    }
    parts.push(current);
    if args.is_empty() {
        format!("{}.to_string()", rust_string(text))
    } else {
        format!(
            "format!({}, {})",
            rust_string(&parts.join("{}")),
            args.join(", ")
        )
    }
}

/// Цвет: `#rrggbb`, `#rgb` или имя.
fn color_code(color: &str) -> Option<String> {
    let named = match color.to_ascii_lowercase().as_str() {
        "red" => Some((229, 72, 77)),
        "green" => Some((57, 211, 83)),
        "blue" => Some((56, 139, 253)),
        "orange" => Some((232, 132, 60)),
        "yellow" => Some((211, 180, 57)),
        "gray" | "grey" => Some((139, 148, 158)),
        "white" => Some((240, 246, 252)),
        "black" => Some((13, 17, 23)),
        _ => None,
    };
    if let Some((r, g, b)) = named {
        return Some(format!("egui::Color32::from_rgb({r}, {g}, {b})"));
    }
    let hex = color.trim_start_matches('#');
    let (r, g, b) = match hex.len() {
        6 => (
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
        ),
        3 => {
            let expand = |c: char| u8::from_str_radix(&format!("{c}{c}"), 16).ok();
            let mut chars = hex.chars();
            (
                expand(chars.next()?)?,
                expand(chars.next()?)?,
                expand(chars.next()?)?,
            )
        }
        _ => return None,
    };
    Some(format!("egui::Color32::from_rgb({r}, {g}, {b})"))
}

/// Иконки: имена → глифы, которые есть в шрифтах egui по умолчанию.
pub fn icon_glyph(name: &str) -> Option<&'static str> {
    // Только глифы, которые есть в шрифтах egui (emoji-icon-font + NotoEmoji):
    // проверено кадром — стрелки и геометрические фигуры там дают «тофу».
    const ICONS: &[(&str, &str)] = &[
        ("save", "💾"),
        ("open", "📂"),
        ("new", "➕"),
        ("delete", "🗑"),
        ("undo", "↻"),
        ("redo", "⏩"),
        ("pencil", "✏"),
        ("brush", "🖌"),
        ("cut", "✂"),
        ("copy", "⧉"),
        ("paste", "📋"),
        ("search", "🔍"),
        ("settings", "⚙"),
        ("play", "⏵"),
        ("pause", "⏸"),
        ("stop", "⏹"),
        ("prev", "⏮"),
        ("next", "⏭"),
        ("check", "✔"),
        ("close", "✖"),
        ("star", "★"),
        ("heart", "♥"),
        ("warn", "⚠"),
        ("info", "ℹ"),
        ("file", "🗐"),
        ("folder", "🗀"),
        ("print", "🖶"),
        ("plus", "➕"),
        ("minus", "➖"),
        ("text", "A"),
    ];
    ICONS
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, glyph)| *glyph)
}

/// Таблица клавиш: имена из модели → egui::Key.
fn key_table() -> String {
    let mut out = String::new();
    for ch in 'a'..='z' {
        out.push_str(&format!(
            "        (\"{ch}\", egui::Key::{}),\n",
            ch.to_ascii_uppercase()
        ));
    }
    for digit in 0..=9 {
        out.push_str(&format!("        (\"{digit}\", egui::Key::Num{digit}),\n"));
    }
    let named = [
        ("escape", "Escape"),
        ("esc", "Escape"),
        ("enter", "Enter"),
        ("return", "Enter"),
        ("space", "Space"),
        ("tab", "Tab"),
        ("backspace", "Backspace"),
        ("delete", "Delete"),
        ("del", "Delete"),
        ("home", "Home"),
        ("end", "End"),
        ("insert", "Insert"),
        ("page-up", "PageUp"),
        ("page-down", "PageDown"),
        ("up", "ArrowUp"),
        ("down", "ArrowDown"),
        ("left", "ArrowLeft"),
        ("right", "ArrowRight"),
        ("plus", "Plus"),
        ("minus", "Minus"),
        ("equal", "Equals"),
        ("comma", "Comma"),
        ("period", "Period"),
        ("slash", "Slash"),
        ("f1", "F1"),
        ("f2", "F2"),
        ("f3", "F3"),
        ("f4", "F4"),
        ("f5", "F5"),
        ("f6", "F6"),
        ("f7", "F7"),
        ("f8", "F8"),
        ("f9", "F9"),
        ("f10", "F10"),
        ("f11", "F11"),
        ("f12", "F12"),
    ];
    for (name, key) in named {
        out.push_str(&format!("        (\"{name}\", egui::Key::{key}),\n"));
    }
    out
}

/// Функция темы: основа, акцент, шрифт, отступы, скругления.
///
/// В egui 0.37 стиль живёт по темам, поэтому правим сразу все через all_styles_mut.
fn theme_code(screen: &Screen) -> String {
    let theme = &screen.theme;
    let accent = theme
        .accent
        .as_deref()
        .and_then(color_code)
        .unwrap_or_else(|| "egui::Color32::from_rgb(56, 139, 253)".to_string());
    format!(
        r#"
fn apply_theme(ctx: &egui::Context) {{
    ctx.set_theme(egui::ThemePreference::{});
    let accent = {accent};
    // Текст на акценте должен читаться: светлый акцент — тёмный текст и наоборот.
    let on_accent = if accent.r() as u32 + accent.g() as u32 + accent.b() as u32 > 420 {{
        egui::Color32::from_rgb(16, 18, 24)
    }} else {{
        egui::Color32::WHITE
    }};
    let dark = {};
    let round = egui::CornerRadius::same({});
    let spacing = {};
    let font = {};
    ctx.all_styles_mut(|style| {{
        style.visuals = if dark {{ egui::Visuals::dark() }} else {{ egui::Visuals::light() }};
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
        ] {{
            style
                .text_styles
                .insert(text_style, egui::FontId::proportional(size));
        }}
    }});
}}
"#,
        if theme.dark { "Dark" } else { "Light" },
        if theme.dark { "true" } else { "false" },
        theme.round,
        float_lit(theme.spacing),
        float_lit(theme.font)
    )
}

fn shortcut_helpers() -> String {
    format!(
        r#"
/// Горячая клавиша из модели: "Ctrl+S", "F2", "Ctrl+Shift+Z".
fn shortcut(ui: &egui::Ui, spec: &str) -> bool {{
    let mut ctrl = false;
    let mut shift = false;
    let mut alt = false;
    let mut key = String::new();
    for part in spec.split('+') {{
        match part.trim().to_ascii_lowercase().as_str() {{
            "ctrl" | "control" | "cmd" | "command" => ctrl = true,
            "shift" => shift = true,
            "alt" | "option" => alt = true,
            other => key = other.to_string(),
        }}
    }}
    let Some(key) = key_from_name(&key) else {{
        return false;
    }};
    ui.input(|i| {{
        i.key_pressed(key)
            && i.modifiers.ctrl == ctrl
            && i.modifiers.shift == shift
            && i.modifiers.alt == alt
    }})
}}

fn key_from_name(name: &str) -> Option<egui::Key> {{
    const KEYS: &[(&str, egui::Key)] = &[
{}
    ];
    KEYS
        .iter()
        .find(|(label, _)| *label == name)
        .map(|(_, key)| *key)
}}
"#,
        key_table()
    )
}

fn styled_text(text_code: &str, widget: &Widget) -> String {
    let text_code = match widget.style.icon.as_deref().and_then(icon_glyph) {
        Some(glyph) => format!(
            "format!(\"{} {{}}\", {text_code})",
            glyph.replace('{', "{{").replace('}', "}}")
        ),
        None => text_code.to_string(),
    };
    let mut builder = format!("egui::RichText::new({text_code})");
    if let Some(size) = widget.style.size {
        builder.push_str(&format!(".size({})", float_lit(size)));
    }
    if widget.style.bold {
        builder.push_str(".strong()");
    }
    if let Some(color) = &widget.style.color {
        if let Some(code) = color_code(color) {
            builder.push_str(&format!(".color({code})"));
        }
    }
    builder
}

fn with_align(body: &str, align: &Option<Align>, pad: &str) -> String {
    match align {
        Some(Align::Right) => format!(
            "{pad}ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {{\n{body}{pad}}});\n"
        ),
        Some(Align::Center) => format!(
            "{pad}ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {{\n{body}{pad}}});\n"
        ),
        _ => body.to_string(),
    }
}

fn reindent(code: &str, levels: usize) -> String {
    let pad = "    ".repeat(levels);
    code.lines()
        .map(|line| {
            if line.trim().is_empty() {
                String::from("\n")
            } else {
                format!("{pad}{line}\n")
            }
        })
        .collect()
}

/// Состояние генерации: уникальные id, обработчики как отдельные функции.
#[derive(Default)]
struct Emit {
    counter: usize,
    /// (имя кнопки, код действия) в порядке появления
    actions: Vec<(String, String)>,
}

impl Emit {
    fn action(&mut self, name: &str, code: String) -> String {
        let index = self.actions.len();
        self.actions.push((name.to_string(), code));
        format!("action_{index}")
    }
}

fn emit_widgets(
    widgets: &[Widget],
    screen: &Screen,
    depth: usize,
    emit: &mut Emit,
) -> Result<String, String> {
    let pad = "    ".repeat(depth);
    let mut out = String::new();
    for widget in widgets {
        let code = emit_widget(widget, screen, depth, emit)?;
        if let (Some(width), Some(height)) = (widget.style.width, widget.style.height) {
            out.push_str(&format!(
                "{pad}ui.allocate_ui_with_layout(egui::vec2({}, {}), egui::Layout::top_down(egui::Align::Min), |ui| {{\n",
                float_lit(width),
                float_lit(height)
            ));
            out.push_str(&reindent(&code, 1));
            out.push_str(&format!("{pad}}});\n"));
        } else if let Some(width) = widget.style.width {
            out.push_str(&format!("{pad}ui.set_width({});\n", float_lit(width)));
            out.push_str(&code);
        } else {
            out.push_str(&code);
        }
    }
    Ok(out)
}

fn emit_widget(
    widget: &Widget,
    screen: &Screen,
    depth: usize,
    emit: &mut Emit,
) -> Result<String, String> {
    let pad = "    ".repeat(depth);
    let mut out = String::new();
    match &widget.kind {
        WidgetKind::Label => {
            let text = styled_text(&text_expr(&widget.text, screen), widget);
            out.push_str(&format!("{pad}ui.label({text});\n"));
        }
        WidgetKind::Button => {
            let text = styled_text(&text_expr(&widget.text, screen), widget);
            if let Some(effect) = &widget.effect {
                let code = expr::compile(screen, effect)
                    .map_err(|err| format!("{}: {}", err.message, err.hint))?;
                let action = emit.action(&widget.text, code);
                if let Some(key) = &widget.style.key {
                    out.push_str(&format!(
                        "{pad}if shortcut(ui, {}) {{\n{pad}    {action}(state);\n{pad}}}\n",
                        rust_string(key)
                    ));
                }
                out.push_str(&format!(
                    "{pad}if ui.button({text}).clicked() {{\n{pad}    {action}(state);\n{pad}}}\n"
                ));
            } else {
                if let Some(key) = &widget.style.key {
                    // клавиша без действия: только подсказка в подписи
                    out.push_str(&format!(
                        "{pad}ui.button({text}).on_hover_text({});\n",
                        rust_string(key)
                    ));
                } else {
                    out.push_str(&format!("{pad}ui.button({text});\n"));
                }
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
            let text = styled_text(&text_expr(&widget.text, screen), widget);
            out.push_str(&format!("{pad}ui.checkbox(&mut state.{field}, {text});\n"));
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
            if widget.style.grow {
                out.push_str(&format!(
                    "{pad}ui.add_sized(\n{pad}    [ui.available_width(), 0.0],\n{pad}    egui::TextEdit::singleline(&mut state.{field}),\n{pad});\n"
                ));
            } else {
                out.push_str(&format!(
                    "{pad}ui.text_edit_singleline(&mut state.{field});\n"
                ));
            }
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
                _ => format!("{}..={}", float_lit(a as f32), float_lit(b as f32)),
            };
            out.push_str(&format!(
                "{pad}ui.add(egui::Slider::new(&mut state.{field}, {range}){label});\n"
            ));
        }
        WidgetKind::Separator => out.push_str(&format!("{pad}ui.separator();\n")),
        WidgetKind::Space => {
            let height = widget.number.unwrap_or(8.0);
            out.push_str(&format!("{pad}ui.add_space({});\n", float_lit(height)));
        }
        WidgetKind::Vbox => {
            let body = emit_widgets(&widget.children, screen, depth + 1, emit)?;
            let aligned = if widget.style.align.is_some() {
                with_align(&body, &widget.style.align, &"    ".repeat(depth + 1))
            } else {
                body.clone()
            };
            if let Some(pad_px) = widget.style.pad {
                out.push_str(&format!(
                    "{pad}egui::Frame::new().inner_margin({pad_px}).show(ui, |ui| {{\n{aligned}{pad}}});\n"
                ));
            } else if widget.style.align.is_some() {
                out.push_str(&with_align(&body, &widget.style.align, &pad));
            } else {
                out.push_str(&format!("{pad}ui.vertical(|ui| {{\n{body}{pad}}});\n"));
            }
        }
        WidgetKind::Hbox => {
            let body = emit_widgets(&widget.children, screen, depth + 1, emit)?;
            out.push_str(&format!("{pad}ui.horizontal(|ui| {{\n{body}{pad}}});\n"));
        }
        WidgetKind::Grid => {
            let columns = (widget.number.unwrap_or(2.0) as usize).max(1);
            let id = emit.counter;
            emit.counter += 1;
            out.push_str(&format!(
                "{pad}egui::Grid::new(\"grid-{id}\").num_columns({columns}).spacing([8.0, 6.0]).show(ui, |ui| {{\n"
            ));
            let cell_pad = "    ".repeat(depth + 1);
            for (index, child) in widget.children.iter().enumerate() {
                let cell = emit_widget(child, screen, depth + 1, emit)?;
                out.push_str(&cell);
                if (index + 1) % columns == 0 {
                    out.push_str(&format!("{cell_pad}ui.end_row();\n"));
                }
            }
            if widget.children.len() % columns != 0 {
                out.push_str(&format!("{cell_pad}ui.end_row();\n"));
            }
            out.push_str(&format!("{pad}}});\n"));
        }
        WidgetKind::Scroll => {
            let body = emit_widgets(&widget.children, screen, depth + 1, emit)?;
            out.push_str(&format!(
                "{pad}egui::ScrollArea::vertical().show(ui, |ui| {{\n{body}{pad}}});\n"
            ));
        }
        WidgetKind::Place => {
            let (cw, ch) = widget.box_size.unwrap_or((600.0, 400.0));
            let id = emit.counter;
            emit.counter += 1;
            out.push_str(&format!(
                "{pad}// place: абсолютные координаты и якоря\n{pad}let canvas_{id} = egui::Rect::from_min_size(\n{pad}    ui.available_rect_before_wrap().min,\n{pad}    egui::vec2({}, {}),\n{pad});\n",
                float_lit(cw),
                float_lit(ch)
            ));
            for child in &widget.children {
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
                let body = emit_widget(child, screen, depth + 2, emit)?;
                let body = reindent(&body, 2);
                out.push_str(&format!(
                    "{pad}ui.scope_builder(\n{pad}    egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(\n{pad}        canvas_{id}.min + egui::vec2({}, {}),\n{pad}        egui::vec2({}, {}),\n{pad}    )),\n{pad}    |ui| {{\n{body}{pad}    }},\n{pad});\n",
                    float_lit(x),
                    float_lit(y),
                    float_lit(bw),
                    float_lit(bh)
                ));
            }
            out.push_str(&format!(
                "{pad}ui.advance_cursor_after_rect(canvas_{id});\n"
            ));
        }
        WidgetKind::List => {
            // Список строк: подпись + удаление у каждой строки. Клик по «✖» убирает
            // элемент по индексу, поэтому идём по копии — иначе заимствование спорит.
            let field = widget
                .bind
                .clone()
                .ok_or_else(|| "у списка нужен привязанный список: list -> items".to_string())?;
            let Some(found) = screen.fields.iter().find(|f| f.name == field) else {
                return Err(format!("список «{field}» не объявлен в state"));
            };
            if found.ty != FieldType::List {
                return Err(format!(
                    "«{field}» имеет тип {}, а list работает только со списком",
                    found.ty.as_str()
                ));
            }
            let empty_text = if widget.text.is_empty() {
                "пусто".to_string()
            } else {
                widget.text.clone()
            };
            out.push_str(&format!(
                "{pad}if state.{field}.is_empty() {{\n{pad}    ui.label(egui::RichText::new({}).size(12.0).color(egui::Color32::from_gray(130)));\n{pad}}} else {{\n{pad}    let rows = state.{field}.clone();\n{pad}    for (i, item) in rows.iter().enumerate() {{\n{pad}        ui.horizontal(|ui| {{\n{pad}            ui.label(item);\n{pad}            if ui.small_button(\"✖\").clicked() && i < state.{field}.len() {{\n{pad}                state.{field}.remove(i);\n{pad}            }}\n{pad}        }});\n{pad}    }}\n{pad}}}\n",
                rust_string(&empty_text)
            ));
        }
        WidgetKind::Custom(name) => {
            out.push_str(&format!("{pad}widget_{name}(state, ui);\n"));
        }
    }
    Ok(out)
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
        FieldType::Float => float_lit(value.parse::<f32>().unwrap_or(0.0)),
        // список всегда начинается пустым: начальные элементы задаются действиями
        FieldType::List => "Vec::new()".to_string(),
    }
}

pub fn main_rs(screen: &Screen, _bin_name: &str) -> Result<String, String> {
    let mut state_fields = String::new();
    let mut defaults = String::new();
    let mut snapshot = String::new();
    for field in &screen.rust_fields {
        state_fields.push_str(&format!("    pub {}: {},\n", field.name, field.ty));
        defaults.push_str(&format!("            {}: {},\n", field.name, field.default));
    }
    for field in &screen.fields {
        snapshot.push_str(&format!(
            "        (\"{}\", format!(\"{{:?}}\", state.{})),\n",
            field.name, field.name
        ));
        state_fields.push_str(&format!("    pub {}: {},\n", field.name, field.ty.rust()));
        defaults.push_str(&format!(
            "            {}: {},\n",
            field.name,
            default_expr(&field.ty, &field.value)
        ));
    }
    let mut emit = Emit::default();
    let body = emit_widgets(&screen.root, screen, 1, &mut emit)?;
    let actions_code: String = emit
        .actions
        .iter()
        .enumerate()
        .map(|(index, (name, code))| {
            let body = reindent(code, 1);
            format!(
                "#[allow(unused_variables, unused_mut)]\n/// {}: действие кнопки, собрано из модели.\nfn action_{index}(state: &mut State) {{\n{body}}}\n\n",
                name.replace('\n', " ")
            )
        })
        .collect();
    let actions_table: String = emit
        .actions
        .iter()
        .enumerate()
        .map(|(index, (name, _))| format!("        ({}, action_{index}),\n", rust_string(name)))
        .collect();
    let actions_count = emit.actions.len();

    let init_code: String = screen
        .rust
        .iter()
        .filter(|block| block.kind == RustKind::Init)
        .map(|block| block.code.clone())
        .collect::<Vec<_>>()
        .join("\n");
    let init_block = if init_code.trim().is_empty() {
        String::new()
    } else {
        format!("        // rust init\n{init_code}\n")
    };

    let fns: String = screen
        .rust
        .iter()
        .filter(|block| block.kind == RustKind::Fn)
        .map(|block| format!("{}\n\n", block.code))
        .collect();

    let widgets: String = screen
        .rust
        .iter()
        .filter(|block| block.kind == RustKind::Widget)
        .map(|block| {
            format!(
                "#[allow(unused_variables, unused_mut)]\nfn widget_{name}(state: &mut State, ui: &mut egui::Ui) {{\n{code}\n}}\n\n",
                name = block.name,
                code = block.code
            )
        })
        .collect();

    Ok(format!(
        r#"//! Сгенерировано lazr из form.lazr. Правьте модель, а не этот файл.
#![allow(clippy::all)]

use eframe::egui;
#[allow(unused_imports)]
use eframe::egui::NumExt;
{helpers}{theme}
#[derive(Debug, Clone)]
pub struct State {{
{state_fields}}}

fn snapshot(state: &State) -> Vec<(&'static str, String)> {{
    vec![
{snapshot}    ]
}}

impl Default for State {{
    fn default() -> State {{
        State {{
{defaults}        }}
    }}
}}

impl State {{
    #[allow(unused_mut)]
    pub fn new() -> State {{
        let mut state = State::default();
{init_block}        state
    }}
}}

{fns}{widgets}{actions_code}pub fn draw(state: &mut State, ui: &mut egui::Ui) {{
{body}}}

struct App {{
    state: State,
    themed: bool,
}}

impl eframe::App for App {{
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {{
        if !self.themed {{
            apply_theme(ui.ctx());
            self.themed = true;
        }}
        draw(&mut self.state, ui);
    }}
}}

fn main() {{
    if std::env::args().any(|arg| arg == "--selftest") {{
        let actions: [(&str, fn(&mut State)); {actions_count}] = [
{actions_table}        ];
        let mut state = State::new();
        for (name, action) in actions {{
            // Проверяем эффект, а не факт вызова: печатаем, что именно изменилось.
            let before = snapshot(&state);
            action(&mut state);
            let after = snapshot(&state);
            let mut changes: Vec<String> = Vec::new();
            for ((field, old), (_, new)) in before.iter().zip(after.iter()) {{
                if old != new {{
                    changes.push(format!("{{field}}: {{old}} → {{new}}"));
                }}
            }}
            if changes.is_empty() {{
                println!("  действие «{{name}}»: без изменений");
            }} else {{
                println!("  действие «{{name}}»: {{}}", changes.join(", "));
            }}
        }}
        println!("selftest ok: {{}} действий", actions.len());
        return;
    }}
    if std::env::args().any(|arg| arg == "--headless") {{
        let ctx = egui::Context::default();
        apply_theme(&ctx);
        let mut state = State::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {{
            draw(&mut state, ui);
        }});
        // Кадр не рисуется: дельты текстур нужно погасить, иначе epaint паникует.
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
        Box::new(|_cc| Ok(Box::new(App {{ state: State::new(), themed: false }}))),
    );
}}
"#,
        count = count_widgets(&screen.root),
        helpers = shortcut_helpers(),
        theme = theme_code(screen),
        actions_code = actions_code,
        actions_table = actions_table,
        actions_count = actions_count,
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
label "Нажатий: {count}" {size:22, bold, color:#e5484d}
button "Плюс" do count += 1
checkbox "Согласен" -> agree
"#,
        )
        .unwrap_or_else(|errors| {
            let text: Vec<String> = errors.iter().map(|e| e.to_json()).collect();
            panic!("модель не разобралась: {}", text.join(", "));
        })
    }

    #[test]
    fn styles_become_richtext() {
        let screen =
            parse("state count int 0\nlabel \"x\" {size:22, bold, color:#e5484d}\n").unwrap();
        let code = main_rs(&screen, "app").unwrap();
        assert!(code.contains("egui::RichText::new"), "{code}");
        assert!(code.contains(".size(22.0)"), "{code}");
        assert!(code.contains(".strong()"), "{code}");
        assert!(code.contains("Color32::from_rgb(229, 72, 77)"), "{code}");
    }

    #[test]
    fn grid_and_scroll_and_space_are_emitted() {
        let screen = parse(
            "state count int 0\ngrid 2\n  label \"a\"\n  label \"b\"\n  label \"c\"\nscroll\n  space 12\n",
        )
        .unwrap();
        let code = main_rs(&screen, "app").unwrap();
        assert!(
            code.contains("egui::Grid::new(\"grid-0\").num_columns(2)"),
            "{code}"
        );
        assert!(code.contains("ui.end_row();"), "{code}");
        assert!(code.contains("egui::ScrollArea::vertical()"), "{code}");
        assert!(code.contains("ui.add_space(12.0);"), "{code}");
    }

    #[test]
    fn effects_compile_into_the_button_handler() {
        let screen = sample();
        let code = main_rs(&screen, "app").unwrap();
        assert!(code.contains("state.count += 1;"), "{code}");
    }

    #[test]
    fn rust_blocks_are_emitted_as_functions_and_init() {
        let screen = parse(
            "state count int 0\nrust init {\n  state.count = 42;\n}\nrust widget badge {\n  ui.label(\"x\");\n}\ncustom badge\n",
        )
        .unwrap();
        let code = main_rs(&screen, "app").unwrap();
        assert!(
            code.contains("fn widget_badge(state: &mut State, ui: &mut egui::Ui)"),
            "{code}"
        );
        assert!(
            code.contains("state.count = 42;"),
            "init в конструкторе: {code}"
        );
        assert!(code.contains("widget_badge(state, ui);"), "{code}");
    }

    #[test]
    fn generation_is_deterministic() {
        let a = main_rs(&sample(), "app").unwrap();
        let b = main_rs(&sample(), "app").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn wide_text_input_grows() {
        let screen = parse("state name string \"\"\ntext_input \"Имя\" -> name {grow}\n").unwrap();
        let code = main_rs(&screen, "app").unwrap();
        assert!(code.contains("add_sized"), "{code}");
    }

    #[test]
    fn place_positions_children_and_flips_anchors() {
        let screen = parse(
            "place 300x200\n  label \"a\" at 10 20 size 100x24\n  label \"b\" anchor right bottom at 8 8 size 90x30\n",
        )
        .unwrap();
        let code = main_rs(&screen, "app").unwrap();
        assert!(code.contains("egui::UiBuilder::new().max_rect"), "{code}");
        assert!(code.contains("egui::vec2(10.0, 20.0)"), "{code}");
        // 300 - 90 - 8 = 202, 200 - 30 - 8 = 162
        assert!(
            code.contains("egui::vec2(202.0, 162.0)"),
            "якорь не пересчитан: {code}"
        );
        assert!(code.contains("advance_cursor_after_rect"), "{code}");
    }

    #[test]
    fn theme_becomes_apply_theme() {
        let screen = parse("theme light accent:#39d353 font:16 spacing:12 round:10\n").unwrap();
        let code = main_rs(&screen, "app").unwrap();
        assert!(code.contains("fn apply_theme"), "{code}");
        assert!(code.contains("ThemePreference::Light"), "{code}");
        assert!(code.contains("from_rgb(57, 211, 83)"), "{code}");
        assert!(code.contains("CornerRadius::same(10)"), "{code}");
        assert!(code.contains("let font = 16.0;"), "шрифт из темы: {code}");
    }

    #[test]
    fn crate_name_is_safe() {
        assert_eq!(
            crate_name("Счётчик"),
            "schetchik",
            "кириллица транслитерируется"
        );
        assert_eq!(crate_name("Пульт"), "pult");
        assert_eq!(crate_name("Краски"), "kraski");
        assert_eq!(crate_name("My App!"), "my-app");
        assert_eq!(crate_name("123go"), "app-123go");
    }
}
