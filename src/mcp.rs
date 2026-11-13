//! MCP-сервер: те же операции, что в CLI, но машинно.
//!
//! Смысл: ИИ не пишет `main.rs` и не двигает пиксели. Он вызывает семантические
//! операции над моделью, получает структурный ответ (включая ошибки с номером
//! строки и подсказкой) и проверяет результат сборкой и headless-кадром.

use std::io::{self, BufRead, Write};

use crate::model::{self, FieldType, Widget, WidgetKind};
use crate::project::Project;

pub fn serve() -> i32 {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(request) = parse_request(line) else {
            continue;
        };
        let response = match request.method.as_str() {
            "initialize" => Some(format!(
                r#"{{"jsonrpc":"2.0","id":{},"result":{{"protocolVersion":"2025-06-18","capabilities":{{"tools":{{}}}},"serverInfo":{{"name":"lazr","version":"{}"}}}}}}"#,
                request.id,
                env!("CARGO_PKG_VERSION")
            )),
            "notifications/initialized" | "notifications/cancelled" => None,
            "tools/list" => Some(format!(
                r#"{{"jsonrpc":"2.0","id":{},"result":{{"tools":{}}}}}"#,
                request.id,
                tools_json()
            )),
            "tools/call" => Some(format!(
                r#"{{"jsonrpc":"2.0","id":{},"result":{}}}"#,
                request.id,
                call_tool(&request.params)
            )),
            "ping" => Some(format!(
                r#"{{"jsonrpc":"2.0","id":{},"result":{{}}}}"#,
                request.id
            )),
            other => Some(format!(
                r#"{{"jsonrpc":"2.0","id":{},"error":{{"code":-32601,"message":"метод {} не поддержан"}}}}"#,
                request.id, other
            )),
        };
        if let Some(text) = response {
            if writeln!(stdout, "{text}").is_err() {
                break;
            }
            let _ = stdout.flush();
        }
    }
    0
}

struct Request {
    id: String,
    method: String,
    params: String,
}

fn parse_request(line: &str) -> Result<Request, ()> {
    let id = json_value(line, "id").unwrap_or_else(|| "null".to_string());
    let method = json_string_value(line, "method").ok_or(())?;
    let params = json_raw_value(line, "params").unwrap_or_else(|| "{}".to_string());
    Ok(Request { id, method, params })
}

fn tools_json() -> String {
    let tool = |name: &str, desc: &str, schema: &str| {
        format!(
            r#"{{"name":"{name}","description":{},"inputSchema":{schema}}}"#,
            model::json_str(desc)
        )
    };
    let obj = |props: &str, required: &str| {
        format!(r#"{{"type":"object","properties":{{{props}}},"required":[{required}]}}"#)
    };
    let str_prop = |name: &str, desc: &str| {
        format!(
            r#""{name}":{{"type":"string","description":{}}}"#,
            model::json_str(desc)
        )
    };
    let tools = vec![
        tool(
            "form_new",
            "Создать проект экрана: form.lazr + сгенерированное приложение",
            &obj(
                &format!(
                    "{},{},{}",
                    str_prop("dir", "каталог проекта"),
                    str_prop("title", "заголовок окна"),
                    str_prop("size", "размер, например 480x320")
                ),
                r#""dir","title""#,
            ),
        ),
        tool(
            "form_state",
            "Объявить поле состояния (int, float, bool, string)",
            &obj(
                &format!(
                    "{},{},{},{}",
                    str_prop("dir", "каталог проекта"),
                    str_prop("name", "имя поля"),
                    str_prop("type", "int | float | bool | string"),
                    str_prop("value", "начальное значение (необязательно)")
                ),
                r#""dir","name","type""#,
            ),
        ),
        tool(
            "form_add",
            "Добавить виджет: label, button, checkbox, text_input, slider, separator, vbox, hbox",
            &obj(
                &format!(
                    "{},{},{},{},{},{}",
                    str_prop("dir", "каталог проекта"),
                    str_prop("kind", "тип виджета"),
                    str_prop("text", "подпись/текст"),
                    str_prop("bind", "поле состояния для checkbox/text_input/slider"),
                    str_prop("effect", "действие: count += 1, agree = !agree"),
                    str_prop("container", "vbox | hbox, куда положить")
                ),
                r#""dir","kind""#,
            ),
        ),
        tool(
            "form_show",
            "Показать модель как текст",
            &obj(&str_prop("dir", "каталог проекта"), r#""dir""#),
        ),
        tool(
            "form_widgets",
            "Список виджетов модели с путями",
            &obj(&str_prop("dir", "каталог проекта"), r#""dir""#),
        ),
        tool(
            "form_generate",
            "Перегенерировать Rust-код из модели",
            &obj(&str_prop("dir", "каталог проекта"), r#""dir""#),
        ),
        tool(
            "form_build",
            "Собрать приложение cargo build --offline",
            &obj(&str_prop("dir", "каталог проекта"), r#""dir""#),
        ),
        tool(
            "form_headless",
            "Исполнить интерфейс один кадр без окна (проверка)",
            &obj(&str_prop("dir", "каталог проекта"), r#""dir""#),
        ),
    ];
    format!("[{}]", tools.join(","))
}

fn call_tool(params: &str) -> String {
    let name = json_string_value(params, "name").unwrap_or_default();
    let args = json_raw_value(params, "arguments").unwrap_or_else(|| "{}".to_string());
    let dir = json_string_value(&args, "dir").unwrap_or_else(|| ".".to_string());
    let text_arg = |key: &str| json_string_value(&args, key);

    let outcome: Result<String, String> = (|| {
        // form_new создаёт проект: читать форму ещё нечего, поэтому он идёт первым.
        if name == "form_new" {
            let title = text_arg("title").unwrap_or_else(|| "Окно".to_string());
            let (w, h) = parse_size(text_arg("size").as_deref());
            let fresh = Project::create(&dir, &title, w, h)?;
            fresh.generate()?;
            return Ok(format!("{{\"ok\":true,\"dir\":{}}}", model::json_str(&dir)));
        }
        let mut project = Project::load(&dir)?;
        match name.as_str() {
            "form_show" => Ok(project.screen_text()),
            "form_widgets" => Ok(project.widgets_json()),
            "form_generate" => {
                project.generate()?;
                Ok(format!(
                    "{{\"ok\":true,\"generated\":{}}}",
                    model::json_str(&project.main_path())
                ))
            }
            "form_build" => {
                let log = project.build()?;
                Ok(format!(
                    "{{\"ok\":true,\"log\":{}}}",
                    model::json_str(&tail(&log, 12))
                ))
            }
            "form_headless" => {
                let out = project.headless()?;
                Ok(format!(
                    "{{\"ok\":true,\"output\":{}}}",
                    model::json_str(&out)
                ))
            }
            "form_state" => {
                let name = text_arg("name").ok_or("нужно name")?;
                let ty = text_arg("type").ok_or("нужно type")?;
                let ty = FieldType::parse(&ty).ok_or("тип: int, float, bool, string")?;
                let value = text_arg("value").unwrap_or_default();
                project.add_state(&name, ty, &value)?;
                project.save()?;
                project.generate()?;
                Ok("{\"ok\":true}".to_string())
            }
            "form_add" => {
                let kind = text_arg("kind").ok_or("нужно kind")?;
                let kind = WidgetKind::parse_kind(&kind).ok_or_else(|| {
                    format!(
                        "виджет неизвестен; доступны: {}",
                        WidgetKind::all().join(", ")
                    )
                })?;
                let text = text_arg("text").unwrap_or_default();
                let bind = text_arg("bind");
                let effect = text_arg("effect");
                let container = text_arg("container");
                project.add_widget(
                    kind,
                    &text,
                    bind.as_deref(),
                    effect.as_deref(),
                    container.as_deref(),
                )?;
                project.save()?;
                project.generate()?;
                Ok(format!("{{\"ok\":true,\"widgets\":{}}}", project.count()))
            }
            other => Err(format!("инструмент {other} неизвестен")),
        }
    })();

    match outcome {
        Ok(text) => format!(
            r#"{{"content":[{{"type":"text","text":{}}}],"isError":false}}"#,
            model::json_str(&text)
        ),
        Err(err) => format!(
            r#"{{"content":[{{"type":"text","text":{}}}],"isError":true}}"#,
            model::json_str(&format!(
                "{{\"ok\":false,\"error\":{}}}",
                model::json_str(&err)
            ))
        ),
    }
}

fn tail(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

fn parse_size(size: Option<&str>) -> (u32, u32) {
    size.and_then(|value| value.split_once('x'))
        .and_then(|(w, h)| Some((w.trim().parse().ok()?, h.trim().parse().ok()?)))
        .unwrap_or((480, 320))
}

// --- минимальный разбор JSON без зависимостей -------------------------------------

pub fn json_value(text: &str, key: &str) -> Option<String> {
    json_raw_value(text, key)
}

pub fn json_raw_value(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let start = text.find(&needle)? + needle.len();
    let rest = text[start..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let mut depth = 0i32;
    let mut quoted = false;
    let mut escaped = false;
    for (index, ch) in rest.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if quoted => escaped = true,
            '"' => {
                if quoted && depth == 0 {
                    return Some(rest[..=index].to_string());
                }
                quoted = !quoted;
            }
            '{' | '[' if !quoted => depth += 1,
            '}' | ']' if !quoted => {
                if depth == 0 {
                    return Some(rest[..index].trim_end_matches(',').to_string());
                }
                depth -= 1;
            }
            ',' if !quoted && depth == 0 => return Some(rest[..index].to_string()),
            _ => {}
        }
    }
    Some(rest.trim_end_matches(',').to_string())
}

pub fn json_string_value(text: &str, key: &str) -> Option<String> {
    let raw = json_raw_value(text, key)?;
    let trimmed = raw.trim();
    if !trimmed.starts_with('"') {
        return None;
    }
    let inner = trimmed.trim_matches('"');
    Some(unescape(inner))
}

fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                if let Ok(code) = u32::from_str_radix(&hex, 16) {
                    if let Some(c) = char::from_u32(code) {
                        out.push(c);
                    }
                }
            }
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

#[allow(dead_code)]
fn unused(_: &Widget) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_extraction_handles_nested_arguments() {
        let line = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"form_add","arguments":{"dir":"/tmp/x","kind":"label","text":"Привет"}}}"#;
        let request = parse_request(line).unwrap();
        assert_eq!(request.id, "7");
        assert_eq!(request.method, "tools/call");
        assert_eq!(
            json_string_value(&request.params, "name").as_deref(),
            Some("form_add")
        );
        let args = json_raw_value(&request.params, "arguments").unwrap();
        assert_eq!(json_string_value(&args, "dir").as_deref(), Some("/tmp/x"));
        assert_eq!(json_string_value(&args, "text").as_deref(), Some("Привет"));
    }

    #[test]
    fn form_new_works_on_an_empty_directory() {
        let dir = std::env::temp_dir().join(format!("lazr-mcp-new-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let params = format!(
            r#"{{"name":"form_new","arguments":{{"dir":{},"title":"MCP","size":"320x200"}}}}"#,
            model::json_str(&dir.display().to_string())
        );
        let response = call_tool(&params);
        assert!(!response.contains("\"isError\":true"), "{response}");
        assert!(dir.join("form.lazr").exists(), "модель создана");
        assert!(dir.join("app/src/main.rs").exists(), "код сгенерирован");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tools_list_mentions_every_operation() {
        let list = tools_json();
        for name in [
            "form_new",
            "form_state",
            "form_add",
            "form_show",
            "form_widgets",
            "form_generate",
            "form_build",
            "form_headless",
        ] {
            assert!(list.contains(name), "нет инструмента {name}");
        }
    }
}
