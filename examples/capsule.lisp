; Планировщик — «капсульный» интерфейс: мягкая тень, пилюли статуса, карточки.
(window "Планировщик" 900 780 :chrome own :min "780x600")
(icon-font "/usr/share/fonts/TTF/fa-solid-900.ttf")
(font-family "/usr/share/fonts/TTF/DejaVuSans.ttf")
(style-sheet "capsule.css")
(state tasks list "Свести требования к поиску,Проверить BM25 на кириллице,Написать тесты парсера,Вынести планировщик в отдельный экран")
(state done int 2)
(state total int 4)
(state filter string "все")

(rust-widget header """
let width = ui.available_width();
let (rect, _hit) = ui.allocate_exact_size(egui::vec2(width, 84.0), egui::Sense::hover());
let painter = ui.painter_at(rect);
// мягкая «тень»: несколько слоёв с убывающей прозрачностью
for step in 0..6 {
    let grow = step as f32 * 2.0;
    let alpha = 26u8.saturating_sub(step as u8 * 4) as u8;
    painter.rect_filled(
        egui::Rect::from_min_max(
            rect.left_top() + egui::vec2(0.0, 8.0 + grow),
            rect.right_bottom() + egui::vec2(0.0, grow),
        ),
        egui::CornerRadius::same(18),
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, alpha),
    );
}
// «градиент» полосами: от акцента к панели
let body = rect.shrink2(egui::vec2(0.0, 4.0));
for line in 0..28 {
    let t = line as f32 / 27.0;
    let mix = |a: f32, b: f32| (a + (b - a) * t) as u8;
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(body.left(), body.top() + 28.0 * 0.0 + (body.height() / 28.0) * line as f32),
            egui::pos2(body.right(), body.top() + (body.height() / 28.0) * (line + 1) as f32),
        ),
        egui::CornerRadius::ZERO,
        egui::Color32::from_rgb(mix(38.0, 21.0), mix(45.0, 22.0), mix(74.0, 27.0)),
    );
}
painter.rect_stroke(
    body,
    egui::CornerRadius::same(18),
    egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(124, 156, 255, 70)),
    egui::StrokeKind::Inside,
);
painter.text(
    egui::pos2(body.left() + 22.0, body.top() + 30.0),
    egui::Align2::LEFT_CENTER,
    "КВАРТАЛЬНЫЙ ПЛАН",
    egui::FontId::proportional(11.0),
    egui::Color32::from_rgb(160, 172, 255),
);
painter.text(
    egui::pos2(body.left() + 22.0, body.top() + 58.0),
    egui::Align2::LEFT_CENTER,
    "Поиск по базе знаний",
    egui::FontId::proportional(26.0),
    egui::Color32::from_rgb(235, 238, 250),
);
""")

(vbox
  (custom header)
  (hbox
    (card
      (vbox
        (label "ЗАДАЧ" :class "meta")
        (label "{total}" :class "strong")
        (label "в этом спринте" :class "meta")))
    (card
      (vbox
        (label "ГОТОВО" :class "meta")
        (label "{done}" :class "strong" :color "#7c9cff")
        (label "из {total}" :class "meta")))
    (card
      (vbox
        (label "ФОКУС" :class "meta")
        (label "поиск" :size 20 :bold true)
        (label "BM25 · tf-idf" :class "meta"))))
  (separator)
  (hbox
    (label "{done}/{total} готово" :class "pill")
    (label "{filter}" :class "meta")
    (label "" :size 6 :grow true)
    (button "все" (set! filter "все") :size 11)
    (button "открытые" (set! filter "открытые") :size 11)
    (button "+ задача" (append tasks "Новая задача") :icon "\uf067"))
  (list :bind tasks :empty "задач нет — добавьте кнопкой «+»"
    (hbox
      (label "✓" :size 12 :color "#7c9cff")
      (label "{item}" :class "row" :hint "клик по строке откроет страницу")
      (label "сегодня" :class "pill")
      (button "" (remove tasks) :icon "\uf1f8" :size 11)))
  (separator)
  (hbox
    (label "обновлено 2 дня назад" :class "meta" :grow true)
    (label "{filter}" :class "meta" :color "#7c9cff")
    (icon "\uf013" :size 12 :color "#9aa3b8")))
