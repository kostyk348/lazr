; Краски — полноценный Paint на модели lazr.
;
; Всё, что видно, — формы DSL. Rust остаётся ровно там, где он и нужен: пиксели
; холста, палитра и сохранение файла. Инструменты, цвет и толщина — поля модели.
(window "Краски" 900 720)

(state tool string "кисть")
(state color string "#1b1f24")
(state size float 4)
(state text string "Привет")
(state file string "paint.sav")

(rust-field strokes "Vec<Stroke>" "Vec::new()")
(rust-field undo "Vec<Vec<Stroke>>" "Vec::new()")
(rust-field bg "egui::Color32" "egui::Color32::WHITE")

(rust-item stroke """
#[derive(Clone)]
pub struct Stroke {
    pub kind: u8,
    pub color: egui::Color32,
    pub width: f32,
    pub points: Vec<egui::Pos2>,
    pub text: String,
}

impl Stroke {
    pub fn new(kind: u8, color: egui::Color32, width: f32, at: egui::Pos2) -> Stroke {
        Stroke { kind, color, width, points: vec![at], text: String::new() }
    }

    pub fn draw(&self, painter: &egui::Painter) {
        let stroke = egui::Stroke::new(self.width, self.color);
        match self.kind {
            0 | 1 | 2 => {
                if self.points.len() < 2 {
                    painter.circle_filled(self.points[0], (self.width / 2.0).max(0.5), self.color);
                } else {
                    painter.add(egui::Shape::line(self.points.clone(), stroke));
                }
            }
            3 => {
                if let (Some(a), Some(b)) = (self.points.first(), self.points.last()) {
                    painter.line_segment([*a, *b], stroke);
                }
            }
            4 => {
                if let (Some(a), Some(b)) = (self.points.first(), self.points.last()) {
                    let rect = egui::Rect::from_two_pos(*a, *b);
                    painter.add(egui::Shape::closed_line(
                        vec![rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()],
                        stroke,
                    ));
                }
            }
            5 => {
                if let (Some(a), Some(b)) = (self.points.first(), self.points.last()) {
                    let rect = egui::Rect::from_two_pos(*a, *b);
                    painter.add(egui::Shape::ellipse_stroke(rect.center(), rect.size() / 2.0, stroke));
                }
            }
            6 => {
                if let Some(at) = self.points.first() {
                    painter.text(*at, egui::Align2::LEFT_TOP, &self.text, egui::FontId::proportional(self.width), self.color);
                }
            }
            _ => {}
        }
    }

    pub fn to_line(&self) -> String {
        let c = self.color;
        let mut out = format!(
            "{};{};{};{};{};{}",
            self.kind, c.r(), c.g(), c.b(), self.width, self.text
        );
        for p in &self.points {
            out.push_str(&format!(";{:.1},{:.1}", p.x, p.y));
        }
        out
    }

    pub fn from_line(line: &str) -> Option<Stroke> {
        let parts: Vec<&str> = line.split(';').collect();
        if parts.len() < 6 {
            return None;
        }
        let mut stroke = Stroke {
            kind: parts[0].parse().ok()?,
            color: egui::Color32::from_rgb(
                parts[1].parse().unwrap_or(0),
                parts[2].parse().unwrap_or(0),
                parts[3].parse().unwrap_or(0),
            ),
            width: parts[4].parse().unwrap_or(2.0),
            points: Vec::new(),
            text: parts[5].to_string(),
        };
        for p in &parts[6..] {
            let (x, y) = p.split_once(',')?;
            stroke.points.push(egui::pos2(x.parse().ok()?, y.parse().ok()?));
        }
        Some(stroke)
    }
}

pub fn save_strokes(strokes: &[Stroke], path: &str) -> std::io::Result<()> {
    let mut out = String::new();
    for stroke in strokes {
        out.push_str(&stroke.to_line());
        out.push('\n');
    }
    std::fs::write(path, out)
}

pub fn load_strokes(path: &str) -> Vec<Stroke> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines().filter_map(Stroke::from_line).collect()
}

pub fn hex_color(text: &str) -> egui::Color32 {
    let hex = text.trim().trim_start_matches('#');
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0);
    if hex.len() == 6 {
        egui::Color32::from_rgb(byte(0), byte(2), byte(4))
    } else {
        egui::Color32::from_rgb(27, 31, 36)
    }
}
""")

(rust-widget toolbar """
let tools = [
    ("карандаш", "✏"),
    ("кисть", "🖌"),
    ("ластик", "◻"),
    ("линия", "╱"),
    ("прямоугольник", "▭"),
    ("овал", "◯"),
    ("текст", "A"),
    ("заливка", "▣"),
];
ui.horizontal(|ui| {
    for (index, (name, icon)) in tools.iter().enumerate() {
        let key = format!("{}", index + 1);
        if shortcut(ui, &key) {
            state.tool = name.to_string();
        }
        if ui
            .selectable_label(state.tool == *name, format!("{} {}", icon, name))
            .on_hover_text(format!("клавиша {}", key))
            .clicked()
        {
            state.tool = name.to_string();
        }
    }
});
""")

(rust-widget palette """
let colors = [
    ("#1b1f24", "чёрный"),
    ("#e5484d", "красный"),
    ("#f2751f", "оранжевый"),
    ("#e3b341", "жёлтый"),
    ("#39d353", "зелёный"),
    ("#388bfd", "синий"),
    ("#a371f7", "сиреневый"),
    ("#8b949e", "серый"),
];
ui.horizontal(|ui| {
    for (hex, name) in colors {
        let picked = state.color.eq_ignore_ascii_case(hex);
        let text = if picked { format!("• {}", name) } else { name.to_string() };
        if ui
            .button(egui::RichText::new(text).color(hex_color(hex)))
            .on_hover_text(hex)
            .clicked()
        {
            state.color = hex.to_string();
        }
    }
});
""")

(rust-widget actions """
if shortcut(ui, "ctrl+z") {
    if let Some(previous) = state.undo.pop() {
        state.strokes = previous;
    }
}
if shortcut(ui, "ctrl+s") {
    let path = state.file.clone();
    if save_strokes(&state.strokes, &path).is_ok() {
        state.text = format!("сохранено: {}", path);
    }
}
if shortcut(ui, "ctrl+n") {
    state.undo.push(state.strokes.clone());
    state.strokes.clear();
    state.bg = egui::Color32::WHITE;
}
if ui.button("↩ Отменить").on_hover_text("Ctrl+Z").clicked() {
    if let Some(previous) = state.undo.pop() {
        state.strokes = previous;
    }
}
if ui.button("💾 Сохранить").on_hover_text("Ctrl+S").clicked() {
    let path = state.file.clone();
    if save_strokes(&state.strokes, &path).is_ok() {
        state.text = format!("сохранено: {}", path);
    }
}
if ui.button("📂 Открыть").clicked() {
    let path = state.file.clone();
    state.undo.push(state.strokes.clone());
    state.strokes = load_strokes(&path);
}
if ui.button("🗑 Очистить").on_hover_text("Ctrl+N").clicked() {
    state.undo.push(state.strokes.clone());
    state.strokes.clear();
    state.bg = egui::Color32::WHITE;
}
""")

(rust-widget canvas """
let avail = ui.available_size();
let size = egui::vec2(avail.x.clamp(320.0, 860.0), avail.y.clamp(220.0, 470.0));
let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
let painter = ui.painter_at(rect);
painter.rect_filled(rect, 4.0, state.bg);
painter.rect_stroke(
    rect,
    4.0,
    egui::Stroke::new(1.0, egui::Color32::from_rgb(190, 195, 205)),
    egui::StrokeKind::Inside,
);

let tool = state.tool.clone();
let color = hex_color(&state.color);
let width = state.size;
let kind = match tool.as_str() {
    "карандаш" => 0,
    "кисть" => 1,
    "ластик" => 2,
    "линия" => 3,
    "прямоугольник" => 4,
    "овал" => 5,
    "текст" => 6,
    _ => 7,
};
let ink = if kind == 2 { state.bg } else { color };

let snapshot = |state: &mut State| {
    state.undo.push(state.strokes.clone());
    if state.undo.len() > 40 {
        state.undo.remove(0);
    }
};

if response.drag_started() {
    if let Some(pos) = response.interact_pointer_pos() {
        if kind <= 5 {
            snapshot(state);
            state.strokes.push(Stroke::new(kind, ink, width, pos));
        }
    }
}
if response.dragged() {
    if let Some(pos) = response.interact_pointer_pos() {
        let last = state.strokes.len().saturating_sub(1);
        if let Some(stroke) = state.strokes.get_mut(last) {
            if kind <= 2 {
                stroke.points.push(pos);
            } else if stroke.points.len() == 1 {
                stroke.points.push(pos);
            } else {
                stroke.points[1] = pos;
            }
        }
    }
}
if response.drag_stopped() {
    let last = state.strokes.len().saturating_sub(1);
    let degenerate = state
        .strokes
        .get(last)
        .map(|stroke| stroke.kind >= 3 && stroke.kind <= 5 && stroke.points.len() < 2)
        .unwrap_or(false);
    if degenerate {
        state.strokes.pop();
    }
}
if response.clicked() {
    if let Some(pos) = response.interact_pointer_pos() {
        match kind {
            6 => {
                snapshot(state);
                let mut stroke = Stroke::new(6, color, (state.size * 4.0).max(14.0), pos);
                stroke.text = state.text.clone();
                state.strokes.push(stroke);
            }
            7 => {
                snapshot(state);
                state.bg = color;
            }
            _ => {}
        }
    }
}

for stroke in &state.strokes {
    stroke.draw(&painter);
}
if state.strokes.is_empty() {
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "рисуй мышкой: инструменты наверху",
        egui::FontId::proportional(16.0),
        egui::Color32::from_rgb(150, 155, 165),
    );
}
""")

(rust-widget status """
ui.label(
    egui::RichText::new(format!(
        "{} · {} · толщина {:.0} · штрихов {} · отмен {} · 1–8 инструменты, Ctrl+Z отмена",
        state.tool, state.color, state.size, state.strokes.len(), state.undo.len()
    ))
    .size(12.0)
    .color(egui::Color32::from_rgb(120, 126, 138)),
);
""")

(vbox :pad 12
  (hbox
    (label "Краски" :size 20 :bold true)
    (space 14)
    (custom toolbar))
  (space 8)
  (custom palette)
  (space 8)
  (hbox
    (custom actions)
    (space 18)
    (slider "толщина" :bind size :range (1 40))
    (space 14)
    (text-input "надпись" :bind text :grow true))
  (space 8)
  (custom canvas)
  (space 6)
  (custom status))
