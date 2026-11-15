//! Язык действий: `count += 1`, `if count > 5 { count = 0; }`, `name = "Аня"`.
//!
//! Компилируется в Rust-код и проверяется по типам **до** `cargo build`, чтобы
//! агент получал ошибку сразу и с подсказкой, а не через три минуты сборки.

use crate::model::{FieldType, Screen};

#[derive(Debug, Clone)]
pub struct ExprError {
    pub message: String,
    pub hint: String,
}

impl ExprError {
    fn new(message: impl Into<String>, hint: impl Into<String>) -> ExprError {
        ExprError {
            message: message.into(),
            hint: hint.into(),
        }
    }
}

#[derive(Debug, Clone)]
enum Tok {
    Ident(String),
    Number(f64),
    Text(String),
    Op(String),
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semi,
    Comma,
    Eof,
}

fn lex(text: &str) -> Result<Vec<Tok>, ExprError> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        if ch.is_ascii_digit() {
            let mut number = String::new();
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                number.push(chars[i]);
                i += 1;
            }
            out.push(Tok::Number(number.parse().map_err(|_| {
                ExprError::new(format!("«{number}» не число"), "пишите 1, 2.5 без букв")
            })?));
            continue;
        }
        if ch.is_alphabetic() || ch == '_' {
            let mut ident = String::new();
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                ident.push(chars[i]);
                i += 1;
            }
            out.push(Tok::Ident(ident));
            continue;
        }
        if ch == '"' {
            i += 1;
            let mut text = String::new();
            while i < chars.len() && chars[i] != '"' {
                text.push(chars[i]);
                i += 1;
            }
            i += 1;
            out.push(Tok::Text(text));
            continue;
        }
        let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
        match two.as_str() {
            "+=" | "-=" | "==" | "!=" | ">=" | "<=" | "&&" | "||" => {
                out.push(Tok::Op(two));
                i += 2;
                continue;
            }
            _ => {}
        }
        match ch {
            '+' | '-' | '*' | '/' | '>' | '<' | '=' | '!' => out.push(Tok::Op(ch.to_string())),
            '(' => out.push(Tok::LParen),
            ')' => out.push(Tok::RParen),
            '{' => out.push(Tok::LBrace),
            '}' => out.push(Tok::RBrace),
            ';' => out.push(Tok::Semi),
            ',' => out.push(Tok::Comma),
            other => {
                return Err(ExprError::new(
                    format!("символ «{other}» не понимаю"),
                    "доступны: = += -= + - * / < > == != if else",
                ))
            }
        }
        i += 1;
    }
    out.push(Tok::Eof);
    Ok(out)
}

struct Parser<'a> {
    toks: Vec<Tok>,
    pos: usize,
    screen: &'a Screen,
}

#[derive(Debug, Clone, PartialEq)]
enum Ty {
    Int,
    Float,
    Str,
    Bool,
    /// Список: участвует только в append/remove/clear, в выражениях бесполезен.
    List,
}

impl Ty {
    fn from(field: &FieldType) -> Ty {
        match field {
            FieldType::Int => Ty::Int,
            FieldType::Float => Ty::Float,
            FieldType::Bool => Ty::Bool,
            FieldType::Str => Ty::Str,
            FieldType::List => Ty::List,
        }
    }
    fn numeric(&self) -> bool {
        matches!(self, Ty::Int | Ty::Float)
    }
}

struct Compiled {
    code: String,
    ty: Ty,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos.min(self.toks.len() - 1)]
    }
    fn next(&mut self) -> Tok {
        let tok = self.toks[self.pos.min(self.toks.len() - 1)].clone();
        self.pos += 1;
        tok
    }
    fn eat(&mut self, tok: &Tok) -> bool {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(tok) {
            let same = match (self.peek(), tok) {
                (Tok::Op(a), Tok::Op(b)) => a == b,
                (Tok::Ident(a), Tok::Ident(b)) => a == b,
                _ => true,
            };
            if same {
                self.pos += 1;
                return true;
            }
        }
        false
    }
    fn expect_semi(&mut self) -> Result<(), ExprError> {
        if self.eat(&Tok::Semi) || matches!(self.peek(), Tok::RBrace | Tok::Eof) {
            Ok(())
        } else {
            Err(ExprError::new(
                "ожидалась «;» между действиями",
                "count += 1; name = \"Аня\"",
            ))
        }
    }

    fn field(&mut self, name: &str) -> Result<Ty, ExprError> {
        match self.screen.fields.iter().find(|f| f.name == name) {
            Some(field) => Ok(Ty::from(&field.ty)),
            None => Err(ExprError::new(
                format!("нет поля «{name}»"),
                "поле объявляется строкой state имя тип значение",
            )),
        }
    }

    fn statements(&mut self, depth: usize) -> Result<String, ExprError> {
        let mut out = String::new();
        loop {
            match self.peek() {
                Tok::Eof | Tok::RBrace => break,
                _ => {}
            }
            let stmt = self.statement(depth)?;
            out.push_str(&stmt);
            self.expect_semi()?;
        }
        Ok(out)
    }

    fn statement(&mut self, depth: usize) -> Result<String, ExprError> {
        let pad = "    ".repeat(depth);
        if let Tok::Ident(word) = self.peek().clone() {
            if word == "if" {
                self.next();
                // «if playing { … }» — поле-флаг без сравнения
                if let Tok::Ident(name) = self.peek().clone() {
                    if matches!(self.toks.get(self.pos + 1), Some(Tok::LBrace)) {
                        if let Some(field) = self.screen.fields.iter().find(|f| f.name == name) {
                            if field.ty == FieldType::Bool {
                                self.next();
                                let cond = format!("state.{name}");
                                if !self.eat(&Tok::LBrace) {
                                    return Err(ExprError::new(
                                        "после условия нужна «{»",
                                        "if agree { … } else { … }",
                                    ));
                                }
                                let then_code = self.statements(depth + 1)?;
                                if !self.eat(&Tok::RBrace) {
                                    return Err(ExprError::new(
                                        "условие не закрыто",
                                        "добавьте «}»",
                                    ));
                                }
                                let mut code = format!("{pad}if {cond} {{\n{then_code}{pad}}}");
                                if self.eat(&Tok::Ident("else".to_string())) {
                                    if !self.eat(&Tok::LBrace) {
                                        return Err(ExprError::new(
                                            "после else нужна «{»",
                                            "else { … }",
                                        ));
                                    }
                                    let else_code = self.statements(depth + 1)?;
                                    if !self.eat(&Tok::RBrace) {
                                        return Err(ExprError::new(
                                            "else не закрыт",
                                            "добавьте «}»",
                                        ));
                                    }
                                    code.push_str(&format!(" else {{\n{else_code}{pad}}}"));
                                }
                                return Ok(code);
                            }
                        }
                    }
                }
                let (cond, _ty) = self.comparison()?;
                if !self.eat(&Tok::LBrace) {
                    return Err(ExprError::new(
                        "после условия нужна «{»",
                        "if count > 5 { count = 0; }",
                    ));
                }
                let then_code = self.statements(depth + 1)?;
                if !self.eat(&Tok::RBrace) {
                    return Err(ExprError::new("условие не закрыто", "добавьте «}»"));
                }
                let mut code = format!("{pad}if {cond} {{\n{then_code}{pad}}}");
                if self.eat(&Tok::Ident("else".to_string())) {
                    if !self.eat(&Tok::LBrace) {
                        return Err(ExprError::new("после else нужна «{»", "else { … }"));
                    }
                    let else_code = self.statements(depth + 1)?;
                    if !self.eat(&Tok::RBrace) {
                        return Err(ExprError::new("else не закрыт", "добавьте «}»"));
                    }
                    code.push_str(&format!(" else {{\n{else_code}{pad}}}"));
                }
                return Ok(code);
            }
            // операции над списком: append items "текст", remove items 0, clear items
            if matches!(word.as_str(), "append" | "remove" | "clear") {
                self.next();
                let name = match self.next() {
                    Tok::Ident(name) => name,
                    _ => {
                        return Err(ExprError::new(
                            format!("после {word} нужно имя списка"),
                            "append items \"молоко\"; remove items 0; clear items",
                        ))
                    }
                };
                let ty = self.field(&name)?;
                if ty != Ty::List {
                    return Err(ExprError::new(
                        format!("«{name}» — не список, а {}", self.type_name(&name)),
                        "append/remove/clear работают только с полем типа list",
                    ));
                }
                return match word.as_str() {
                    "clear" => Ok(format!("{pad}state.{name}.clear();\n")),
                    "append" => {
                        let value = self.expression()?;
                        let code = match value.ty {
                            // to_string() годится и литералу, и полю: push ждёт String
                            Ty::Str | Ty::Int | Ty::Float => {
                                format!("({}).to_string()", value.code)
                            }
                            _ => {
                                return Err(ExprError::new(
                                    "в список можно класть строки и числа",
                                    "append items \"молоко\"; append items count",
                                ))
                            }
                        };
                        Ok(format!("{pad}state.{name}.push({code});\n"))
                    }
                    _ => {
                        // remove без индекса — только для шаблона строки списка
                        if matches!(self.peek(), Tok::Eof | Tok::Semi) {
                            return Ok(format!("{pad}remove_row = Some(i);\n"));
                        }
                        let index = self.expression()?;
                        if index.ty != Ty::Int {
                            return Err(ExprError::new(
                                "индекс удаления должен быть целым",
                                "remove items 0",
                            ));
                        }
                        let idx = index.code;
                        Ok(format!(
                            "{pad}if ({idx}) < state.{name}.len() {{\n{pad}    state.{name}.remove({idx});\n{pad}}}\n"
                        ))
                    }
                };
            }
            // присваивание
            self.next();
            let target_ty = self.field(&word)?;
            let op = match self.next() {
                Tok::Op(op) if matches!(op.as_str(), "=" | "+=" | "-=") => op,
                _ => {
                    return Err(ExprError::new(
                        format!("после «{word}» ожидалось =, += или -="),
                        "count += 1",
                    ))
                }
            };
            let value = self.expression()?;
            match (&op[..], &target_ty, &value.ty) {
                ("+=", Ty::Int | Ty::Float, Ty::Int | Ty::Float) => {}
                ("-=", Ty::Int | Ty::Float, Ty::Int | Ty::Float) => {}
                ("+=", Ty::Str, Ty::Str) => {
                    return Ok(format!("{pad}state.{word}.push_str(&{});\n", value.code))
                }
                ("=", Ty::Bool, Ty::Bool) => {}
                ("=", Ty::Str, Ty::Str) => {}
                ("=", Ty::Int | Ty::Float, Ty::Int | Ty::Float) => {}
                (op, target, value_ty) => {
                    return Err(ExprError::new(
                        format!(
                            "«{op}» нельзя применить: поле {word} — {:?}, а значение — {:?}",
                            target, value_ty
                        ),
                        "числа к числам, строки к строкам, bool к bool",
                    ))
                }
            }
            return Ok(format!("{pad}state.{word} {op} {};\n", value.code));
        }
        Err(ExprError::new(
            "действие должно начинаться с поля или if",
            "count += 1; if count > 5 { count = 0; }",
        ))
    }

    /// Имя типа поля для сообщений об ошибках.
    fn type_name(&self, name: &str) -> &'static str {
        self.screen
            .fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| field.ty.as_str())
            .unwrap_or("полем")
    }

    /// Сравнение: `count > 5`, `name == "Аня"`.
    fn comparison(&mut self) -> Result<(String, Ty), ExprError> {
        let left = self.expression()?;
        let op = match self.next() {
            Tok::Op(op) if matches!(op.as_str(), ">" | "<" | ">=" | "<=" | "==" | "!=") => op,
            _ => {
                return Err(ExprError::new(
                    "в условии нужно сравнение",
                    "if count > 5 { … }",
                ))
            }
        };
        let right = self.expression()?;
        if left.ty.numeric() != right.ty.numeric() {
            return Err(ExprError::new(
                "сравнивать можно числа с числами, строки со строками",
                "count > 5, name == \"Аня\"",
            ));
        }
        if matches!(left.ty, Ty::Bool) || matches!(right.ty, Ty::Bool) {
            return Err(ExprError::new(
                "bool в условии сравнивать не нужно",
                "if agree { … }",
            ));
        }
        Ok((format!("{} {} {}", left.code, op, right.code), Ty::Bool))
    }

    fn expression(&mut self) -> Result<Compiled, ExprError> {
        let mut left = self.term()?;
        loop {
            let op = match self.peek() {
                Tok::Op(op) if op == "+" || op == "-" => op.clone(),
                _ => break,
            };
            self.next();
            let right = self.term()?;
            if left.ty == Ty::Str || right.ty == Ty::Str {
                if op != "+" || left.ty != Ty::Str || right.ty != Ty::Str {
                    return Err(ExprError::new(
                        "строки можно только складывать со строками",
                        "name + \"!\"",
                    ));
                }
                left = Compiled {
                    code: format!("format!(\"{{}}{{}}\", {}, {})", left.code, right.code),
                    ty: Ty::Str,
                };
                continue;
            }
            if !left.ty.numeric() || !right.ty.numeric() {
                return Err(ExprError::new("арифметика только для чисел", "count + 1"));
            }
            let ty = if left.ty == Ty::Float || right.ty == Ty::Float {
                Ty::Float
            } else {
                Ty::Int
            };
            left = Compiled {
                code: format!("({} {} {})", left.code, op, right.code),
                ty,
            };
        }
        Ok(left)
    }

    fn term(&mut self) -> Result<Compiled, ExprError> {
        let mut left = self.factor()?;
        loop {
            let op = match self.peek() {
                Tok::Op(op) if op == "*" || op == "/" => op.clone(),
                _ => break,
            };
            self.next();
            let right = self.factor()?;
            if !left.ty.numeric() || !right.ty.numeric() {
                return Err(ExprError::new(
                    "умножать и делить можно только числа",
                    "count * 2",
                ));
            }
            let ty = if left.ty == Ty::Float || right.ty == Ty::Float {
                Ty::Float
            } else {
                Ty::Int
            };
            left = Compiled {
                code: format!("({} {} {})", left.code, op, right.code),
                ty,
            };
        }
        Ok(left)
    }

    fn factor(&mut self) -> Result<Compiled, ExprError> {
        match self.next() {
            Tok::Number(value) => {
                let ty = if (value - value.round()).abs() < f64::EPSILON {
                    Ty::Int
                } else {
                    Ty::Float
                };
                Ok(Compiled {
                    code: format_number(value, &ty),
                    ty,
                })
            }
            Tok::Text(text) => Ok(Compiled {
                code: format!("{}.to_string()", rust_string(&text)),
                ty: Ty::Str,
            }),
            Tok::Ident(name) => {
                if name == "true" || name == "false" {
                    return Ok(Compiled {
                        code: name,
                        ty: Ty::Bool,
                    });
                }
                if name == "!" {
                    let inner = self.factor()?;
                    if inner.ty != Ty::Bool {
                        return Err(ExprError::new("«!» работает только с bool", "!agree"));
                    }
                    return Ok(Compiled {
                        code: format!("(!{})", inner.code),
                        ty: Ty::Bool,
                    });
                }
                let ty = self.field(&name)?;
                Ok(Compiled {
                    code: format!("state.{name}"),
                    ty,
                })
            }
            Tok::Op(op) if op == "!" => {
                let inner = self.factor()?;
                if inner.ty != Ty::Bool {
                    return Err(ExprError::new("«!» работает только с bool", "!agree"));
                }
                Ok(Compiled {
                    code: format!("(!{})", inner.code),
                    ty: Ty::Bool,
                })
            }
            Tok::LParen => {
                let inner = self.expression()?;
                if !self.eat(&Tok::RParen) {
                    return Err(ExprError::new("скобка не закрыта", ")"));
                }
                Ok(inner)
            }
            other => Err(ExprError::new(
                format!("неожиданное значение: {other:?}"),
                "число, строка, поле или (выражение)",
            )),
        }
    }
}

fn format_number(value: f64, ty: &Ty) -> String {
    match ty {
        Ty::Int => format!("{}", value as i64),
        _ => {
            if (value - value.round()).abs() < f64::EPSILON {
                format!("{value:.1}")
            } else {
                format!("{value}")
            }
        }
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

/// Скомпилировать условие для раскладки: `when count > 5`, `when agree`.
pub fn compile_condition(screen: &Screen, source: &str) -> Result<String, ExprError> {
    let text = source.trim();
    if text.is_empty() {
        return Err(ExprError::new("у when нет условия", "when count > 5"));
    }
    let toks = lex(text)?;
    // одно поле-флаг — уже условие: when agree
    if let (Tok::Ident(name), Tok::Eof) = (
        toks.first().cloned().unwrap_or(Tok::Eof),
        toks.get(1).cloned().unwrap_or(Tok::Eof),
    ) {
        if let Some(field) = screen.fields.iter().find(|field| field.name == name) {
            if field.ty == FieldType::Bool {
                return Ok(format!("state.{name}"));
            }
        }
    }
    let mut parser = Parser {
        toks,
        pos: 0,
        screen,
    };
    let (code, ty) = parser.comparison()?;
    if !matches!(parser.peek(), Tok::Eof) {
        return Err(ExprError::new("в условии лишний текст", "when count > 5"));
    }
    if ty != Ty::Bool {
        return Err(ExprError::new(
            "условие должно быть логическим",
            "count > 5",
        ));
    }
    Ok(code)
}

/// Скомпилировать действие в Rust-код.
pub fn compile(screen: &Screen, source: &str) -> Result<String, ExprError> {
    let text = source.trim();
    if text.is_empty() {
        return Err(ExprError::new("пустое действие", "count += 1"));
    }
    let toks = lex(text)?;
    let mut parser = Parser {
        toks,
        pos: 0,
        screen,
    };
    let code = parser.statements(0)?;
    if !matches!(parser.peek(), Tok::Eof) {
        return Err(ExprError::new(
            "после действия остался лишний текст",
            "действия разделяются «;»",
        ));
    }
    Ok(code.trim_end().to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn bare_flag_in_if() {
        let screen = crate::model::Screen {
            title: "t".into(),
            width: 300,
            height: 200,
            fields: vec![crate::model::Field {
                name: "agree".into(),
                ty: crate::model::FieldType::Bool,
                value: "true".into(),
            }],
            rust: Vec::new(),
            rust_fields: Vec::new(),
            theme: crate::model::Theme::default(),
            icon_font: None,
            root: Vec::new(),
        };
        let code =
            crate::expr::compile(&screen, "if agree { agree = false }").expect("условие-флаг");
        assert!(code.contains("if state.agree {"), "{code}");
        assert!(code.contains("state.agree = false"), "{code}");
    }

    use super::*;
    use crate::model::parse;

    fn screen() -> Screen {
        parse("state count int 0\nstate name string \"\"\nstate agree bool false\n").unwrap()
    }

    #[test]
    fn simple_assignments_compile() {
        let s = screen();
        assert_eq!(compile(&s, "count += 1").unwrap(), "state.count += 1;");
        assert_eq!(compile(&s, "count -= 2").unwrap(), "state.count -= 2;");
        assert_eq!(
            compile(&s, "name = \"Аня\"").unwrap(),
            "state.name = \"Аня\".to_string();"
        );
        assert_eq!(
            compile(&s, "agree = !agree").unwrap(),
            "state.agree = (!state.agree);"
        );
    }

    #[test]
    fn if_else_compiles() {
        let s = screen();
        let code = compile(&s, "if count > 5 { count = 0; } else { count += 1; }").unwrap();
        assert!(code.contains("if state.count > 5 {"), "{code}");
        assert!(code.contains("state.count = 0;"), "{code}");
        assert!(code.contains("} else {"), "{code}");
    }

    #[test]
    fn arithmetic_and_parentheses() {
        let s = screen();
        assert_eq!(
            compile(&s, "count = (count + 1) * 2").unwrap(),
            "state.count = ((state.count + 1) * 2);"
        );
    }

    #[test]
    fn two_statements_need_a_semicolon() {
        let s = screen();
        assert!(compile(&s, "count += 1 name = \"x\"").is_err());
        assert!(compile(&s, "count += 1; name = \"x\"").is_ok());
    }

    #[test]
    fn type_mismatches_are_refused() {
        let s = screen();
        assert!(compile(&s, "count = name").is_err(), "число из строки");
        assert!(compile(&s, "name += 1").is_err(), "число к строке");
        assert!(compile(&s, "count += true").is_err());
        assert!(compile(&s, "if count > name { count = 1; }").is_err());
    }

    #[test]
    fn unknown_field_is_named() {
        let s = screen();
        let err = compile(&s, "nope += 1").unwrap_err();
        assert!(err.message.contains("nope"));
        assert!(err.hint.contains("state"));
    }

    #[test]
    fn string_concatenation_uses_format() {
        let s = screen();
        let code = compile(&s, "name = name + \"!\"").unwrap();
        assert!(code.contains("format!(\"{}{}\""), "{code}");
    }
}
