; Кинотеатр: настоящее видео — ffplay (или mpv) в отдельном окне.
; Демо-поток рисует ffmpeg сам; «открыть файл» — системный диалог.
(window "Кинотеатр" 720 420 :chrome own :min "640x380")
(icon-font "/usr/share/fonts/TTF/fa-solid-900.ttf")
(theme dark accent:#e50914 panel:#0a0a0c card:#141418 muted:#9aa3b8 font:15 spacing:10 round:12)
(state history list)
(state plays int 0)

(rust-widget screen """
let width = ui.available_width();
let (rect, _hit) = ui.allocate_exact_size(egui::vec2(width, 180.0), egui::Sense::hover());
let painter = ui.painter_at(rect);
painter.rect_filled(rect, egui::CornerRadius::same(14), egui::Color32::from_rgb(5, 5, 7));
painter.rect_stroke(
    rect,
    egui::CornerRadius::same(14),
    egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(229, 9, 20, 90)),
    egui::StrokeKind::Inside,
);
painter.text(
    egui::pos2(rect.center().x, rect.center().y - 16.0),
    egui::Align2::CENTER_CENTER,
    "\u{f03d}",
    egui::FontId::new(42.0, egui::FontFamily::Name("icons".into())),
    egui::Color32::from_rgb(229, 9, 20),
);
painter.text(
    egui::pos2(rect.center().x, rect.center().y + 22.0),
    egui::Align2::CENTER_CENTER,
    "видео открывается отдельным окном ffplay",
    egui::FontId::proportional(12.0),
    egui::Color32::from_rgb(154, 163, 184),
);

// две полосы: демо-поток и свой файл
let (demo_rect, demo_hit) = ui.allocate_exact_size(egui::vec2(width, 36.0), egui::Sense::click());
let dp = ui.painter_at(demo_rect);
let over = if demo_hit.hovered() { egui::Color32::from_rgb(70, 14, 16) } else { egui::Color32::from_rgb(40, 10, 12) };
dp.rect_filled(demo_rect, egui::CornerRadius::same(10), over);
dp.rect_stroke(demo_rect, egui::CornerRadius::same(10), egui::Stroke::new(1.0, egui::Color32::from_rgb(229, 9, 20)), egui::StrokeKind::Inside);
dp.text(
    egui::pos2(demo_rect.left() + 14.0, demo_rect.center().y),
    egui::Align2::LEFT_CENTER,
    "\u{f04b}  смотреть демо-видео (ffplay, testsrc2)",
    egui::FontId::proportional(13.0),
    egui::Color32::from_rgb(240, 244, 250),
);
if demo_hit.clicked() {
    state.plays += 1;
    state.history.push("демо-поток (testsrc2)".to_string());
    let _ = std::process::Command::new("ffplay")
        .arg("-x").arg("800").arg("-y").arg("450")
        .arg("-f").arg("lavfi")
        .arg("-i").arg("testsrc2=size=800x450:rate=30")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}
let (file_rect, file_hit) = ui.allocate_exact_size(egui::vec2(width, 36.0), egui::Sense::click());
let fp = ui.painter_at(file_rect);
let fover = if file_hit.hovered() { egui::Color32::from_rgb(30, 34, 44) } else { egui::Color32::from_rgb(20, 22, 28) };
fp.rect_filled(file_rect, egui::CornerRadius::same(10), fover);
fp.rect_stroke(file_rect, egui::CornerRadius::same(10), egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 96, 110)), egui::StrokeKind::Inside);
fp.text(
    egui::pos2(file_rect.left() + 14.0, file_rect.center().y),
    egui::Align2::LEFT_CENTER,
    "\u{f07c}  открыть файл… (zenity, играет ffplay)",
    egui::FontId::proportional(13.0),
    egui::Color32::from_rgb(220, 226, 236),
);
if file_hit.clicked() {
    let output = std::process::Command::new("zenity")
        .arg("--file-selection")
        .stdout(std::process::Stdio::piped())
        .output();
    if let Ok(out) = output {
        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !path.is_empty() {
            state.plays += 1;
            state.history.push(path.clone());
            let _ = std::process::Command::new("ffplay")
                .arg("-x").arg("800").arg("-y").arg("450")
                .arg(&path)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
    }
}
""")

(vbox
  (hbox
    (icon "\\uf03d" :size 13 :color "#e50914")
    (label "кинопоказ" :size 13 :mono true :grow true :color "#9aa3b8")
    (label "{plays} запусков" :size 12 :mono true :color "#e50914"))
  (custom screen)
  (separator)
  (hbox
    (label "история просмотров" :size 11 :mono true :grow true :color "#9aa3b8")
    (button "" (clear history) :icon "\\uf1f8" :size 11))
  (list :bind history :empty "пусто — запустите демо или откройте файл"
    (hbox
      (label "\\uf03d" :size 11 :color "#e50914")
      (label "{item}" :size 13 :grow true :color "#eef1f5"))))
