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

fn text_expr_row(text: &str, screen: &Screen, row: bool) -> String {
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
            if closed && row && name == "item" {
                parts.push(current.clone());
                current.clear();
                args.push("item".to_string());
                continue;
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
    let panel = theme
        .panel
        .as_deref()
        .and_then(color_code)
        .map(|code| format!("Some({code})"))
        .unwrap_or_else(|| "None".to_string());
    let card = theme
        .card
        .as_deref()
        .and_then(color_code)
        .map(|code| format!("Some({code})"))
        .unwrap_or_else(|| "None".to_string());
    let muted = theme
        .muted
        .as_deref()
        .and_then(color_code)
        .map(|code| format!("Some({code})"))
        .unwrap_or_else(|| "None".to_string());
    // Подключённый TTF с иконками: семейство «icons», глифы берём из него.
    let icon_font = match &screen.icon_font {
        Some(path) => format!(
            r#"
/// Подключённый шрифт иконок (модель: icon-font "{path}").
fn apply_icon_font(ctx: &egui::Context) {{
    let path = "{path}";
    match std::fs::read(path) {{
        Ok(bytes) => ctx.add_font(egui::FontInsert::new(
            "icons",
            egui::FontData::from_owned(bytes),
            vec![egui::InsertFontFamily {{
                family: egui::FontFamily::Name("icons".into()),
                priority: egui::FontPriority::Highest,
            }}],
        )),
        Err(err) => eprintln!("иконки: не прочитать {{path}}: {{err}}"),
    }}
}}
"#,
            path = path.replace('\\', "\\\\").replace('"', "\\\"")
        ),
        None => String::new(),
    };

    let code = format!(
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
    let panel: Option<egui::Color32> = {panel};
    let card: Option<egui::Color32> = {card};
    let muted: Option<egui::Color32> = {muted};
    let round = egui::CornerRadius::same({});
    let spacing = {};
    let font = {};
    ctx.all_styles_mut(|style| {{
        style.visuals = if dark {{ egui::Visuals::dark() }} else {{ egui::Visuals::light() }};
        // Поле ввода рисуется цветом extreme_bg — он обязан отличаться от фона панели,
        // иначе text_input невидим (проверено скриншотом демки «Задачи»).
        style.visuals.extreme_bg_color = if dark {{
            egui::Color32::from_rgb(24, 28, 36)
        }} else {{
            egui::Color32::from_rgb(248, 249, 251)
        }};
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(
            1.0,
            if dark {{
                egui::Color32::from_rgb(58, 66, 80)
            }} else {{
                egui::Color32::from_rgb(205, 210, 218)
            }},
        );
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
        // Аккуратные состояния кнопок: подсветка в цвет акцента, а не серая.
        style.visuals.widgets.hovered.bg_fill = accent.gamma_multiply(0.28);
        style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, accent);
        style.visuals.widgets.active.bg_fill = accent.gamma_multiply(0.42);
        style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, accent);
        style.visuals.widgets.inactive.bg_fill = if dark {{
            egui::Color32::from_rgb(26, 31, 40)
        }} else {{
            egui::Color32::from_rgb(238, 241, 246)
        }};
        // panel/card из модели: у ретро-стилей светло-серые, у тёмных — почти чёрные
        style.visuals.panel_fill = panel.unwrap_or(if dark {{
            egui::Color32::from_rgb(10, 12, 16)
        }} else {{
            egui::Color32::from_rgb(250, 251, 253)
        }});
        // muted: вторичный текст и линии-разделители
        if let Some(muted) = muted {{
            style.visuals.weak_text_color = Some(muted);
            style.visuals.widgets.noninteractive.bg_stroke =
                egui::Stroke::new(1.0, muted.gamma_multiply(0.7));
        }}
        style.visuals.faint_bg_color = card.unwrap_or(if dark {{
            egui::Color32::from_rgb(22, 26, 33)
        }} else {{
            egui::Color32::from_rgb(238, 241, 246)
        }});
        style.visuals.window_fill = style.visuals.panel_fill;
        // Текст на подсвеченной кнопке должен читаться.
        style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, on_accent);
        style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, on_accent);
        style.spacing.interact_size.y = font * 1.9;
    }});
    {{icon_font_call}}
    {{ui_font_call}}
}}
"#,
        if theme.dark { "Dark" } else { "Light" },
        if theme.dark { "true" } else { "false" },
        theme.round,
        float_lit(theme.spacing),
        float_lit(theme.font),
        panel = panel,
        card = card,
        muted = muted,
    );
    let code = with_icon_font(code, screen.icon_font.is_some());
    let code = with_ui_font(code, screen.ui_font.is_some());
    format!("{icon_font}{}{code}", ui_font_code(screen))
}

/// Функция подключения своего шрифта текста.
fn ui_font_code(screen: &Screen) -> String {
    match &screen.ui_font {
        Some(path) => format!(
            r#"
/// Свой шрифт текста (модель: font-family "{path}").
fn apply_ui_font(ctx: &egui::Context) {{
    // Ставим первым в proportional: встроенный шрифт остаётся запасным,
    // поэтому кириллица и недостающие глифы не превращаются в квадраты.
    let path = "{path}";
    match std::fs::read(path) {{
        Ok(bytes) => ctx.add_font(egui::FontInsert::new(
            "ui",
            egui::FontData::from_owned(bytes),
            vec![egui::InsertFontFamily {{
                family: egui::FontFamily::Proportional,
                priority: egui::FontPriority::Highest,
            }}],
        )),
        Err(err) => eprintln!("шрифт: не прочитать {{path}}: {{err}}"),
    }}
}}
"#,
            path = path.replace('\\', "\\\\").replace('"', "\\\"")
        ),
        None => String::new(),
    }
}

fn with_ui_font(code: String, has_font: bool) -> String {
    if has_font {
        code.replace("{ui_font_call}", "apply_ui_font(ctx);")
    } else {
        code.replace("{ui_font_call}", "")
    }
}

fn with_icon_font(code: String, has_font: bool) -> String {
    if has_font {
        code.replace("{icon_font_call}", "apply_icon_font(ctx);")
    } else {
        code.replace("{icon_font_call}", "")
    }
}

fn card_helpers() -> String {
    r#"
/// Карточка: фон, рамка, скругление и внутренние отступы из темы.
fn card_frame(ui: &egui::Ui, pad: f32) -> egui::Frame {
    let visuals = ui.visuals();
    egui::Frame::new()
        .fill(visuals.faint_bg_color)
        .stroke(egui::Stroke::new(
            1.0,
            visuals.widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(pad)
}
"#
    .to_string()
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

/// Иконка, заданная кодемпоинтом или самим символом приватной зоны:
/// `icon "\uf04b"` или `icon ""`. Встроенный набор обрабатывается отдельно.
fn custom_glyph(name: &str) -> Option<String> {
    if name.contains("\\u") {
        return Some(crate::model::unescape(name));
    }
    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) if (ch as u32) >= 0xE000 => Some(ch.to_string()),
        _ => None,
    }
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
    if widget.style.mono {
        builder.push_str(".monospace()");
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
    /// Мы внутри шаблона строки списка: {item} — это текущая строка.
    row: bool,
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
            let text = styled_text(&text_expr_row(&widget.text, screen, emit.row), widget);
            let hint = match &widget.style.hint {
                Some(tip) => format!(".on_hover_text({})", rust_string(tip)),
                None => String::new(),
            };
            out.push_str(&format!("{pad}ui.label({text}){hint};\n"));
        }
        WidgetKind::Button => {
            // Компактная кнопка-иконка: подписи нет, глиф из подключённого шрифта.
            if widget.text.trim().is_empty() {
                if let Some(glyph) = widget.style.icon.as_deref().and_then(custom_glyph) {
                    let size = widget.style.size.unwrap_or(16.0);
                    out.push_str(&format!(
                        "{pad}if ui.button(egui::RichText::new({}).family(egui::FontFamily::Name(\"icons\".into())).size({})).clicked() {{\n",
                        rust_string(&glyph),
                        float_lit(size)
                    ));
                    if let Some(effect) = &widget.effect {
                        let code = expr::compile(screen, effect)
                            .map_err(|err| format!("{}: {}", err.message, err.hint))?;
                        let action = emit.action(&widget.text, code);
                        out.push_str(&format!("{pad}    {action}(state);\n"));
                    }
                    out.push_str(&format!("{pad}}}\n"));
                    return Ok(out);
                }
            }
            let text = styled_text(&text_expr_row(&widget.text, screen, emit.row), widget);
            if let Some(effect) = &widget.effect {
                // В шаблоне строки «remove items» (без индекса) — это «удалить эту строку».
                if !emit.row {
                    if let Some(rest) = effect.trim().strip_prefix("remove ") {
                        if rest.split_whitespace().count() == 1 {
                            return Err(format!(
                                "«{}» без индекса работает только в шаблоне строки списка",
                                effect.trim()
                            ));
                        }
                    }
                }
                if emit.row {
                    if let Some(rest) = effect.trim().strip_prefix("remove ") {
                        let parts: Vec<&str> = rest.split_whitespace().collect();
                        if parts.len() == 1 {
                            out.push_str(&format!(
                                "{pad}if ui.button({text}).clicked() {{\n{pad}    remove_row = Some(i);\n{pad}}}\n"
                            ));
                            return Ok(out);
                        }
                    }
                }
                let code = expr::compile(screen, effect)
                    .map_err(|err| format!("{}: {}", err.message, err.hint))?;
                let action = emit.action(&widget.text, code);
                if let Some(key) = &widget.style.key {
                    out.push_str(&format!(
                        "{pad}if shortcut(ui, {}) {{\n{pad}    {action}(state);\n{pad}}}\n",
                        rust_string(key)
                    ));
                }
                let hint = match &widget.style.hint {
                    Some(tip) => format!(".on_hover_text({})", rust_string(tip)),
                    None => String::new(),
                };
                out.push_str(&format!(
                    "{pad}if ui.button({text}){hint}.clicked() {{\n{pad}    {action}(state);\n{pad}}}\n"
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
            let text = styled_text(&text_expr_row(&widget.text, screen, emit.row), widget);
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
                "{pad}ui.add(egui::Slider::new(&mut state.{field}, {range}).show_value(false){label});\n"
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
        WidgetKind::When => {
            // Условие компилируется тем же языком, что и обработчики: типы проверяются.
            let condition = expr::compile_condition(screen, &widget.text)
                .map_err(|err| format!("when: {} ({})", err.message, err.hint))?;
            // Ребёнок else — не виджет, а вторая ветка.
            let mut then_part: Vec<Widget> = Vec::new();
            let mut else_part: Option<&Widget> = None;
            for child in &widget.children {
                if child.kind == WidgetKind::Else {
                    else_part = Some(child);
                } else {
                    then_part.push(child.clone());
                }
            }
            let body = emit_widgets(&then_part, screen, depth + 1, emit)?;
            let else_code = match else_part {
                Some(branch) => {
                    let else_body = emit_widgets(&branch.children, screen, depth + 1, emit)?;
                    format!(" else {{\n{else_body}{pad}}}\n")
                }
                None => "\n".to_string(),
            };
            out.push_str(&format!("{pad}if {condition} {{\n{body}{pad}}}{else_code}"));
        }
        // Ветка else рисуется только внутри when; сама по себе — ошибка модели.
        WidgetKind::Else => {
            return Err("«else» без when: ветка должна идти сразу после when".to_string());
        }
        WidgetKind::Card => {
            // Карточка: фон, рамка, скругление и отступы — из общего помощника.
            let pad_px = widget.style.pad.unwrap_or(12.0);
            let body = emit_widgets(&widget.children, screen, depth + 1, emit)?;
            out.push_str(&format!(
                "{pad}card_frame(ui, {}).show(ui, |ui| {{\n{pad}    ui.set_width(ui.available_width());\n{body}{pad}}});\n",
                float_lit(pad_px)
            ));
        }
        WidgetKind::Icon => {
            // Глиф из подключённого шрифта. Без шрифта это был бы «тофу» — говорим прямо.
            if screen.icon_font.is_none() {
                return Err(
                    "иконке нужен шрифт: добавьте строку icon-font \"/путь/к/icons.ttf\""
                        .to_string(),
                );
            }
            let size = widget.style.size.unwrap_or(18.0);
            let mut rich = format!(
                "egui::RichText::new({}).family(egui::FontFamily::Name(\"icons\".into())).size({})",
                rust_string(&widget.text),
                float_lit(size)
            );
            if let Some(color) = &widget.style.color {
                if let Some(code) = color_code(color) {
                    rich.push_str(&format!(".color({code})"));
                }
            }
            out.push_str(&format!("{pad}ui.label({rich});\n"));
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
            // Список строк. Без детей — подпись + «✖»; с детьми — шаблон строки,
            // в котором {item} это текущая строка, а действие «remove items»
            // удаляет именно её.
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
                "{pad}if state.{field}.is_empty() {{\n{pad}    ui.label(egui::RichText::new({}).size(12.0).color(egui::Color32::from_gray(160)));\n{pad}}} else {{\n",
                rust_string(&empty_text)
            ));
            out.push_str(&format!("{pad}    let rows = state.{field}.clone();\n"));
            out.push_str(&format!(
                "{pad}    let mut remove_row: Option<usize> = None;\n"
            ));
            out.push_str(&format!(
                "{pad}    for (i, item) in rows.iter().enumerate() {{\n{pad}        ui.horizontal(|ui| {{\n"
            ));
            if widget.children.is_empty() {
                out.push_str(&format!("{pad}            ui.label(item);\n"));
                out.push_str(&format!(
                    "{pad}            if ui.small_button(\"✖\").clicked() {{\n{pad}                remove_row = Some(i);\n{pad}            }}\n"
                ));
            } else {
                let was_row = emit.row;
                emit.row = true;
                let body = emit_widgets(&widget.children, screen, depth + 3, emit)?;
                emit.row = was_row;
                out.push_str(&body);
            }
            out.push_str(&format!("{pad}        }});\n{pad}    }}\n"));
            out.push_str(&format!(
                "{pad}    if let Some(row) = remove_row {{\n{pad}        if row < state.{field}.len() {{\n{pad}            state.{field}.remove(row);\n{pad}        }}\n{pad}    }}\n"
            ));
            out.push_str(&format!("{pad}}}\n"));
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
        // начальные элементы перечисляются через запятую в одной строке
        FieldType::List => {
            let items: Vec<String> = value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| format!("{}.to_string()", rust_string(item)))
                .collect();
            format!("vec![{}]", items.join(", "))
        }
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
{helpers}{card_helpers}{theme}
// Свои rust-поля (кисти, текстуры, каналы) не обязаны быть Debug/Clone,
// поэтому State не выводит трейты: сравниваем и печатаем только поля модели.
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
{init_block}{store_load}        state
    }}
}}

{fns}{widgets}{actions_code}{store_code}{chrome_fn}pub fn draw(state: &mut State, ui: &mut egui::Ui) {{
    // eframe 0.37 отдаёт голый Ui: фон окна никто не рисует, и светлая тема
    // выглядела чёрной. Красим панель сами — цвет берём из темы.
    // При transparent:yes фон не красим — иначе прозрачность окна не видна.
    if {paint_background} {{
        let background = ui.visuals().panel_fill;
        ui.painter().rect_filled(ui.max_rect(), 0.0, background);
    }}
{chrome_call}{body}}}

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
        {store_before}draw(&mut self.state, ui);
        {store_after}
    }}

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {{
        {clear_color}
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
        {store_save_selftest}
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
            .with_title({title}){viewport},
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
        card_helpers = card_helpers(),
        theme = theme_code(screen),
        actions_code = actions_code,
        actions_table = actions_table,
        actions_count = actions_count,
        width = screen.width,
        height = screen.height,
        title = rust_string(&screen.title),
        viewport = viewport_code(screen)?,
        chrome_fn = chrome_function(screen),
        chrome_call = chrome_call(screen),
        paint_background = if screen.window.transparent {
            "false"
        } else {
            "true"
        },
        clear_color = if screen.window.transparent {
            // Полностью прозрачный буфер: иначе окно закрашивается тёмным фоном,
            // и прозрачность видна только как серое смешение.
            "[0.0, 0.0, 0.0, 0.0]"
        } else {
            "visuals.panel_fill.to_normalized_gamma_f32()"
        },
        store_code = store_code(screen),
        store_save_selftest = if screen.store.is_some() {
            "        store_save(&state);\n        println!(\"состояние записано: {}\", STORE);"
        } else {
            ""
        },
        store_load = if screen.store.is_some() {
            "        store_load(&mut state);\n"
        } else {
            ""
        },
        store_before = if screen.store.is_some() {
            "let before = model_fields(&self.state);\n        "
        } else {
            ""
        },
        store_after = if screen.store.is_some() {
            "if store_changed(&before, &model_fields(&self.state)) {\n            store_save(&self.state);\n        }"
        } else {
            ""
        },
    ))
}

/// Хранилище: значения полей между запусками. Пишем атомарно (временный файл +
/// rename), читаем молча — испорченный файл не должен ронять приложение.
fn store_code(screen: &Screen) -> String {
    let Some(path) = &screen.store else {
        return String::new();
    };
    let path = path.replace('\\', "\\\\").replace('"', "\\\"");
    let mut save = String::new();
    let mut load = String::new();
    for field in &screen.fields {
        let name = &field.name;
        match field.ty {
            crate::model::FieldType::Int | crate::model::FieldType::Float => {
                save.push_str(&format!(
                    "    out.push_str(&format!(\"{name}\\t{{}}\\n\", state.{name}));\n"
                ));
                load.push_str(&format!(
                    "                \"{name}\" => {{\n                    if let Ok(value) = value.parse::<{}>() {{\n                        state.{name} = value;\n                    }}\n                }}\n",
                    if field.ty == crate::model::FieldType::Float {
                        "f32"
                    } else {
                        "i64"
                    }
                ));
            }
            crate::model::FieldType::Bool => {
                save.push_str(&format!(
                    "    out.push_str(&format!(\"{name}\\t{{}}\\n\", state.{name}));\n"
                ));
                load.push_str(&format!(
                    "                \"{name}\" => {{\n                    if let Ok(value) = value.parse::<bool>() {{\n                        state.{name} = value;\n                    }}\n                }}\n"
                ));
            }
            crate::model::FieldType::Str => {
                save.push_str(&format!(
                    "    out.push_str(&format!(\"{name}\\t{{}}\\n\", escape(&state.{name})));\n"
                ));
                load.push_str(&format!(
                    "                \"{name}\" => state.{name} = unescape(value),\n"
                ));
            }
            crate::model::FieldType::List => {
                save.push_str(&format!(
                    "    out.push_str(&format!(\"{name}\\t{{}}\\n\", state.{name}.iter().map(|item| escape(item)).collect::<Vec<_>>().join(\"\\u{{1f}}\")));\n"
                ));
                load.push_str(&format!(
                    "                \"{name}\" => state.{name} = value.split('\\u{{1f}}').map(unescape).filter(|item| !item.is_empty()).collect(),\n"
                ));
            }
        }
    }
    format!(
        r#"
/// Файл состояния приложения.
const STORE: &str = "{path}";

/// Путь с «~» раскрывается в домашний каталог — модель не зависит от машины.
fn store_path() -> String {{
    match STORE.strip_prefix("~/") {{
        Some(rest) => format!("{{}}/{{rest}}", std::env::var("HOME").unwrap_or_default()),
        None => STORE.to_string(),
    }}
}}

fn escape(text: &str) -> String {{
    text.replace('\\', "\\\\").replace('\n', "\\n").replace('\t', "\\t")
}}

fn unescape(text: &str) -> String {{
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {{
        if ch != '\\' {{
            out.push(ch);
            continue;
        }}
        match chars.next() {{
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }}
    }}
    out
}}

fn store_load(state: &mut State) {{
    let Ok(text) = std::fs::read_to_string(store_path()) else {{
        return;
    }};
    for line in text.lines() {{
        let Some((name, value)) = line.split_once('\t') else {{
            continue;
        }};
        match name {{
{load}            _ => {{}}
        }}
    }}
}}

/// Изменилось ли состояние: сравниваем только поля модели. Свои rust-поля
/// (кисти, текстуры, каналы) сравнивать нельзя — они не обязаны быть Eq.
/// Слепок полей модели: дешёвый, сравнимый и без требований к своим типам.
#[derive(PartialEq)]
struct ModelFields {{
{field_decls}}}

fn model_fields(state: &State) -> ModelFields {{
    ModelFields {{
{field_inits}    }}
}}

fn store_changed(before: &ModelFields, after: &ModelFields) -> bool {{
    before != after
}}

fn store_save(state: &State) {{
    let mut out = String::new();
{save}    let path = store_path();
    if let Some(dir) = std::path::Path::new(&path).parent() {{
        if !dir.as_os_str().is_empty() {{
            let _ = std::fs::create_dir_all(dir);
        }}
    }}
    // Пишем во временный файл и переименовываем: оборванная запись не портит данные.
    let temp = format!("{{path}}.tmp");
    if std::fs::write(&temp, out).is_ok() {{
        let _ = std::fs::rename(&temp, &path);
    }}
}}
"#,
        load = load,
        save = save,
        field_decls = screen
            .fields
            .iter()
            .map(|field| format!(
                "    {}: {},\n",
                field.name,
                match field.ty {
                    crate::model::FieldType::Int => "i64",
                    crate::model::FieldType::Float => "f32",
                    crate::model::FieldType::Bool => "bool",
                    crate::model::FieldType::Str => "String",
                    crate::model::FieldType::List => "Vec<String>",
                }
            ))
            .collect::<String>(),
        field_inits = screen
            .fields
            .iter()
            .map(|field| format!("        {}: state.{}.clone(),\n", field.name, field.name))
            .collect::<String>(),
    )
}

/// Опции окна в ViewportBuilder: рамка, размер, всегда сверху, минимум/максимум.
fn viewport_code(screen: &Screen) -> Result<String, String> {
    use crate::model::Chrome;
    let window = &screen.window;
    let mut code = String::new();
    if window.chrome == Chrome::Own {
        code.push_str("\n            .with_decorations(false)");
    }
    if !window.resizable {
        code.push_str("\n            .with_resizable(false)");
    }
    if window.transparent {
        code.push_str("\n            .with_transparent(true)");
    }
    if window.topmost {
        code.push_str("\n            .with_window_level(egui::WindowLevel::AlwaysOnTop)");
    }
    if let Some((w, h)) = window.min {
        code.push_str(&format!(
            "\n            .with_min_inner_size([{w}f32, {h}f32])"
        ));
    }
    if let Some((w, h)) = window.max {
        code.push_str(&format!(
            "\n            .with_max_inner_size([{w}f32, {h}f32])"
        ));
    }
    if let Some(path) = &screen.window_icon {
        // Иконка разбирается своим PNG-декодером на сборке и вшивается пикселями:
        // декодер в само приложение не тащим.
        let bytes = std::fs::read(path).map_err(|err| format!("window-icon «{path}»: {err}"))?;
        let image =
            crate::png::decode(&bytes).map_err(|err| format!("window-icon «{path}»: {err}"))?;
        let (width, height, pixels) = downscale(&image, 48);
        let literal = pixels
            .iter()
            .map(|byte| byte.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        code.push_str(&format!(
            "\n            .with_icon(egui::IconData {{ rgba: vec![{literal}], width: {width}, height: {height} }})"
        ));
    }
    Ok(code)
}

/// Уменьшение «ближайшим соседом»: окно не покажет больше 48×48, а сгенерированный
/// файл не должен раздуваться пикселями.
fn downscale(image: &crate::png::Image, limit: u32) -> (u32, u32, Vec<u8>) {
    if image.width <= limit && image.height <= limit {
        return (image.width, image.height, image.rgba.clone());
    }
    let width = image.width.min(limit);
    let height = image.height.min(limit);
    let mut out = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let source_y = (y as u64 * image.height as u64 / height as u64) as u32;
        for x in 0..width {
            let source_x = (x as u64 * image.width as u64 / width as u64) as u32;
            out.extend_from_slice(&image.pixel(source_x, source_y));
        }
    }
    (width, height, out)
}

/// Своя рамка: заголовок, перетаскивание, крестик. Рисуется без шрифтов —
/// крестик это две линии, поэтому глифы не нужны.
fn chrome_function(screen: &Screen) -> String {
    use crate::model::Chrome;
    if screen.window.chrome != Chrome::Own {
        return String::new();
    }
    format!(
        r#"/// Своя рамка окна: потянуть — переехать, крестик — закрыть.
fn titlebar(ui: &mut egui::Ui, title: &str) {{
    let height = 30.0;
    let rect = egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(ui.available_width(), height));
    let drag = ui.interact(rect, ui.id().with("lazr:titlebar"), egui::Sense::click_and_drag());
    if drag.drag_started() {{
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }}
    let close = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 16.0, rect.center().y),
        egui::vec2(24.0, 24.0),
    );
    let painter = ui.painter().clone();
    let fill = if drag.hovered() {{
        ui.visuals().widgets.hovered.bg_fill
    }} else {{
        ui.visuals().widgets.inactive.bg_fill
    }};
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
    let c = close.center();
    painter.line_segment([c + egui::vec2(-4.0, -4.0), c + egui::vec2(4.0, 4.0)], cross);
    painter.line_segment([c + egui::vec2(4.0, -4.0), c + egui::vec2(-4.0, 4.0)], cross);
    // Двойной клик по полосе — развернуть/вернуть.
    if drag.double_clicked() {{
        let maximized = ui.ctx().input(|input| input.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }}
    if ui.interact(close, ui.id().with("lazr:close"), egui::Sense::click()).clicked() {{
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
    }}
    ui.add_space(height);
}}

"#
    )
}

/// Вызов рамки и «тянуть всё окно», если так сказано в модели.
fn chrome_call(screen: &Screen) -> String {
    use crate::model::Chrome;
    let mut code = String::new();
    if screen.window.chrome == Chrome::Own {
        let title = rust_string(&screen.title);
        code.push_str(&format!("    titlebar(ui, {title});\n"));
    }
    if screen.window.drag_anywhere {
        code.push_str(
            "    if ui.interact(ui.max_rect(), ui.id().with(\"lazr:drag\"), egui::Sense::drag()).drag_started() {\n        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);\n    }\n",
        );
    }
    if screen.window.center {
        code.push_str(&format!(
            "    if !ui.data(|data| data.get_temp::<bool>(ui.id().with(\"lazr:centered\")).unwrap_or(false)) {{\n        if let Some(monitor) = ui.ctx().input(|input| input.viewport().monitor_size) {{\n            ui.ctx().send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2((monitor.x - {}f32) / 2.0, (monitor.y - {}f32) / 2.0)));\n            ui.data_mut(|data| data.insert_temp(egui::Id::new(\"lazr:centered\"), true));\n        }}\n    }}\n",
            float_lit(screen.width as f32),
            float_lit(screen.height as f32)
        ));
    }
    code
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

    #[test]
    fn own_chrome_emits_frameless_window_and_titlebar() {
        let screen = parse(
            r#"window "Т" 420 300 chrome:own topmost:yes min:360x240
label "x""#,
        )
        .expect("окно");
        let source = main_rs(&screen, "t").expect("генерация");
        assert!(
            source.contains(".with_decorations(false)"),
            "рамка не отключена"
        );
        assert!(
            source.contains(".with_window_level(egui::WindowLevel::AlwaysOnTop)"),
            "нет «всегда сверху»"
        );
        assert!(source.contains(".with_min_inner_size([360f32, 240f32])"));
        assert!(
            source.contains("fn titlebar(ui: &mut egui::Ui"),
            "нет своей рамки"
        );
        assert!(
            source.contains("titlebar(ui, \"Т\")"),
            "рамка не вызывается"
        );
    }

    #[test]
    fn state_has_no_required_traits_for_custom_fields() {
        // Свои поля (кисти, текстуры, каналы) не обязаны быть Debug/Clone/Eq,
        // поэтому State не выводит трейты, а хранилище сравнивает поля модели.
        let screen = parse(
            r#"window "Х" 320 220
store "/tmp/x.store"
state count int 0
rust field strokes Vec<Stroke>
label "x""#,
        )
        .expect("модель со своим полем");
        let source = main_rs(&screen, "h").expect("генерация");
        assert!(
            !source.contains("#[derive(Debug, Clone)]\npub struct State"),
            "State не должен требовать Debug/Clone от своих полей"
        );
        assert!(source.contains("struct ModelFields"), "нет слепка полей");
        assert!(source.contains("fn model_fields"), "нет снимка полей");
    }

    #[test]
    fn transparent_window_clears_fully_transparent() {
        // Регрессия: eframe по умолчанию заливает буфер тёмным (12,12,12,180),
        // и прозрачное окно выглядело как серое смешение.
        let screen = parse("window \"Т\" 300 200 transparent:yes\nlabel \"x\"").expect("модель");
        let source = main_rs(&screen, "t").expect("генерация");
        assert!(
            source.contains("with_transparent(true)"),
            "нет прозрачности"
        );
        assert!(
            source.contains("[0.0, 0.0, 0.0, 0.0]"),
            "буфер не прозрачный"
        );
        assert!(source.contains("if false {"), "фон всё ещё закрашивается");

        let opaque = parse("window \"Т\" 300 200\nlabel \"x\"").expect("модель");
        let opaque_source = main_rs(&opaque, "t").expect("генерация");
        assert!(
            opaque_source.contains("if true {"),
            "фон должен закрашиваться"
        );
        assert!(
            opaque_source.contains("visuals.panel_fill.to_normalized_gamma_f32()"),
            "обычное окно должно чиститься цветом панели"
        );
    }

    #[test]
    fn window_icon_is_embedded_as_pixels() {
        // Иконку делаем сами: 3×3 RGBA, чтобы тест не зависел от внешних файлов.
        let dir = std::env::temp_dir().join("lazr-icon-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("icon.png");
        std::fs::write(&path, crate::png::tests::rgba_png(3, 3, [10, 20, 30, 255])).unwrap();
        let screen = parse(&format!(
            "window \"Х\" 320 220\nwindow-icon \"{}\"\nlabel \"x\"",
            path.display()
        ))
        .expect("модель с иконкой");
        let source = main_rs(&screen, "h").expect("генерация");
        assert!(
            source.contains(".with_icon(egui::IconData"),
            "иконка не вшита"
        );
        assert!(
            source.contains("width: 3, height: 3"),
            "размер иконки потерян"
        );
    }

    #[test]
    fn store_emits_atomic_persistence() {
        let screen = parse(
            r#"window "Х" 320 220
store "/tmp/x.store"
state count int 0
state name string "старт"
state items list "a,b"
button "п" do count += 1"#,
        )
        .expect("модель");
        let source = main_rs(&screen, "h").expect("генерация");
        assert!(source.contains("const STORE: &str = \"/tmp/x.store\";"));
        assert!(source.contains("fn store_load"), "нет загрузки");
        assert!(source.contains("fn store_save"), "нет сохранения");
        assert!(
            source.contains("store_save(&state);"),
            "selftest не сохраняет"
        );
        assert!(
            source.contains("store_load(&mut state);"),
            "новый запуск не читает файл"
        );
        // атомарность: временный файл и rename
        assert!(source.contains(".tmp") && source.contains("fs::rename"));
        // списки пишутся разделителем, а не запятой
        assert!(
            source.contains("u{1f}"),
            "список склеен не через разделитель"
        );
    }

    #[test]
    fn ui_font_and_mono_and_hint_reach_the_code() {
        let screen = parse(
            r#"window "Т" 420 300
font-family "/usr/share/fonts/TTF/DejaVuSansMono.ttf"
label "код" {mono, hint:"подсказка"}"#,
        )
        .expect("шрифт и стиль");
        let source = main_rs(&screen, "t").expect("генерация");
        assert!(
            source.contains("fn apply_ui_font"),
            "нет своего шрифта текста"
        );
        assert!(
            source.contains("apply_ui_font(ctx);"),
            "шрифт не применяется"
        );
        assert!(source.contains(".monospace()"), "нет моноширинного текста");
        assert!(source.contains(".on_hover_text("), "нет подсказки");
    }

    #[test]
    fn center_emits_outer_position_once() {
        let screen = parse("window \"Т\" 420 300 center:yes\nlabel \"x\"").expect("окно");
        let source = main_rs(&screen, "t").expect("генерация");
        assert!(
            source.contains("ViewportCommand::OuterPosition"),
            "нет центрирования"
        );
        assert!(
            source.contains("lazr:centered"),
            "центрирование не одноразовое"
        );
    }

    #[test]
    fn system_chrome_has_no_titlebar() {
        let screen = parse("window \"Т\" 420 300\nlabel \"x\"").expect("окно");
        let source = main_rs(&screen, "t").expect("генерация");
        assert!(!source.contains("fn titlebar"));
        assert!(!source.contains("with_decorations(false)"));
    }

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

    #[test]
    fn when_becomes_if_in_code() {
        let screen =
            crate::model::parse("state value int 5\nwhen value > 2\n  label \"видно\"\n").unwrap();
        let code = main_rs(&screen, "app").unwrap();
        assert!(code.contains("if state.value > 2 {"), "{code}");
    }

    #[test]
    fn else_becomes_rust_else() {
        let screen = crate::model::parse(
            "state n int 1\nwhen n > 2\n  label \"да\"\nelse\n  label \"нет\"\n",
        )
        .unwrap();
        let code = main_rs(&screen, "app").unwrap();
        assert!(code.contains("if state.n > 2 {"), "{code}");
        assert!(code.contains("} else {"), "{code}");
    }

    #[test]
    fn list_row_template_and_seed_code() {
        let screen = crate::model::parse(
            "state items list \"молоко,хлеб\"\nlist -> items\n  hbox\n    label \"• {item}\"\n    button \"убрать\" do remove items\n",
        )
        .unwrap();
        let code = main_rs(&screen, "app").unwrap();
        assert!(
            code.contains("vec![\"молоко\".to_string(), \"хлеб\".to_string()]"),
            "{code}"
        );
        assert!(code.contains("for (i, item)"), "{code}");
        assert!(code.contains("format!(\"• {}\", item)"), "{code}");
        assert!(code.contains("remove_row = Some(i)"), "{code}");
    }

    #[test]
    fn row_remove_without_index_is_rejected_outside_template() {
        let screen =
            crate::model::parse("state items list\nbutton \"x\" do remove items\n").unwrap();
        let err = main_rs(&screen, "app").unwrap_err();
        assert!(err.contains("шаблоне строки"), "{err}");
    }

    #[test]
    fn icon_requires_font_and_card_wraps_children() {
        let screen = crate::model::parse("state n int 0\ncard\n  label \"внутри\"\n").unwrap();
        let code = main_rs(&screen, "app").unwrap();
        assert!(code.contains("card_frame(ui,"), "{code}");

        let no_font = crate::model::parse("icon \"\\uf00c\"\n").unwrap();
        let err = main_rs(&no_font, "app").unwrap_err();
        assert!(err.contains("icon-font"), "{err}");
    }
}
