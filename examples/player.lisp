; Плеер — компактный, как Winamp: экран-визуализатор, транспорт, громкость, очередь.
(icon-font "/usr/share/fonts/TTF/fa-solid-900.ttf")
(window "Плеер" 480 520)
(theme light accent:#0084c7 panel:#e6f2f9 card:#ffffff muted:#2f4c5c font:14 spacing:8 round:12)
(state playing bool true)
(state track int 1)
(state pos int 35)
(state volume int 70)
(state queue list)

(rust-widget spectrum """
let size = egui::vec2(ui.available_width(), 110.0);
let (rect, _hit) = ui.allocate_exact_size(size, egui::Sense::hover());
let painter = ui.painter_at(rect);
painter.rect_filled(rect, egui::CornerRadius::same(10), egui::Color32::from_rgb(7, 20, 28));
let bars = 36usize;
let t = ui.input(|i| i.time) as f32;
let step = rect.width() / bars as f32;
for i in 0..bars {
    let phase = i as f32 * 0.42;
    let wave = 0.5 + 0.5 * (t * 2.4 + phase).sin();
    let wobble = 0.55 + 0.45 * (t * 0.8 + phase * 1.9).cos().abs();
    let level = if state.playing { wave * wobble } else { 0.06 };
    let h = (level * rect.height() * 0.92).max(3.0);
    let x = rect.left() + i as f32 * step;
    let color = if level > 0.72 {
        egui::Color32::from_rgb(255, 184, 107)
    } else {
        egui::Color32::from_rgb(0, 190, 255)
    };
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(x + 2.0, rect.bottom() - h - 4.0),
            egui::pos2(x + step - 2.0, rect.bottom() - 4.0),
        ),
        egui::CornerRadius::same(2),
        color,
    );
}
if !state.playing {
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "пауза",
        egui::FontId::proportional(13.0),
        egui::Color32::from_rgb(140, 190, 210),
    );
}
""")

(vbox
  (hbox
    (label "LAZR" :size 12 :bold true :color "#0b5c85")
    (label "player" :size 12 :grow true :color "#5f7d8c")
    (icon "\uf001" :size 11 :color "#00a2e8")
    (label "трек {track}/5" :size 11 :color "#5f7d8c"))
  (custom spectrum)
  (hbox
    (when (== track 1) (label "Полёт над водой" :size 17 :bold true :grow true :color "#0b5c85"))
    (when (== track 2) (label "Стеклянный дождь" :size 17 :bold true :grow true :color "#0b5c85"))
    (when (== track 3) (label "Зелёный шум" :size 17 :bold true :grow true :color "#0b5c85"))
    (when (== track 4) (label "Воздух" :size 17 :bold true :grow true :color "#0b5c85"))
    (when (== track 5) (label "Летний луг" :size 17 :bold true :grow true :color "#0b5c85"))
    (when playing (icon "\uf001" :size 12 :color "#0084c7"))
    (else (icon "\uf04c" :size 12 :color "#2f4c5c")))
  (hbox
    (label "0:{pos}" :size 11 :color "#2f4c5c")
    (slider "позиция" :bind pos :range (0 100))
    (label "4:10" :size 11 :color "#2f4c5c"))
  (hbox
    (button "" (if (<= track 1) (set! track 5) (set! track (- track 1))) :icon "\uf048")
    (button "" (set! pos 0) :icon "\uf04a")
    (when playing (button "" (set! playing false) :icon "\uf04c"))
    (else (button "" (set! playing true) :icon "\uf04b"))
    (button "" (if (>= track 5) (set! track 1) (set! track (+ track 1))) :icon "\uf051"))
  (hbox
    (icon "\uf028" :size 12 :color "#0084c7")
    (slider "громкость" :bind volume :range (0 100))
    (label "{volume}" :size 11 :color "#2f4c5c"))
  (separator)
  (hbox
    (label "очередь · {queue}" :size 11 :grow true :color "#2f4c5c")
    (button "" (append queue track) :icon "\uf067")
    (button "" (clear queue) :icon "\uf1f8"))
  (list :bind queue :empty "пусто — добавьте трек кнопкой «+»"
    (hbox
      (label "• трек {item}" :size 12 :grow true :color "#0b5c85")
      (button "x" (remove queue) :size 10))))
