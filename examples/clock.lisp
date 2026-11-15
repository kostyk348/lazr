; Часы — неоновый циферблат, нарисованный своим Rust-виджетом.
(window "Часы" 420 500 :chrome own :topmost true :min "380x460")
(icon-font "/usr/share/fonts/TTF/fa-solid-900.ttf")
(theme dark accent:#22d3ee panel:#070b12 card:#0e1520 muted:#5b7080 font:14 spacing:10 round:14)
(state hour int 21)
(state minute int 42)
(state second int 10)
(state show_seconds bool true)

(rust-widget dial """
let side = ui.available_width().min(300.0);
let (rect, _hit) = ui.allocate_exact_size(egui::vec2(ui.available_width(), side), egui::Sense::hover());
let center = rect.center();
let radius = side * 0.45;
let painter = ui.painter_at(rect);
// корпус
painter.circle_filled(center, radius + 10.0, egui::Color32::from_rgb(14, 21, 32));
painter.circle_stroke(center, radius + 10.0, egui::Stroke::new(1.5, egui::Color32::from_rgb(34, 211, 238)));
painter.circle_filled(center, radius, egui::Color32::from_rgb(7, 11, 18));
// риски
for tick in 0..60 {
    let angle = tick as f32 * std::f32::consts::TAU / 60.0 - std::f32::consts::FRAC_PI_2;
    let long = tick % 5 == 0;
    let outer = radius - 4.0;
    let inner = radius - if long { 16.0 } else { 9.0 };
    let color = if long {
        egui::Color32::from_rgb(34, 211, 238)
    } else {
        egui::Color32::from_rgb(40, 60, 80)
    };
    painter.line_segment(
        [
            center + egui::vec2(angle.cos(), angle.sin()) * inner,
            center + egui::vec2(angle.cos(), angle.sin()) * outer,
        ],
        egui::Stroke::new(if long { 2.0 } else { 1.0 }, color),
    );
}
let hand = |value: f32, total: f32, length: f32, width: f32, color: egui::Color32| {
    let angle = value / total * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
    painter.line_segment(
        [
            center - egui::vec2(angle.cos(), angle.sin()) * (length * 0.15),
            center + egui::vec2(angle.cos(), angle.sin()) * length,
        ],
        egui::Stroke::new(width, color),
    );
};
let minutes = state.minute as f32 + state.second as f32 / 60.0;
let hours = (state.hour % 12) as f32 + minutes / 60.0;
hand(hours, 12.0, radius * 0.5, 6.0, egui::Color32::from_rgb(226, 232, 240));
hand(minutes, 60.0, radius * 0.72, 4.0, egui::Color32::from_rgb(148, 163, 184));
if state.show_seconds {
    hand(state.second as f32, 60.0, radius * 0.82, 1.8, egui::Color32::from_rgb(34, 211, 238));
}
painter.circle_filled(center, 5.0, egui::Color32::from_rgb(34, 211, 238));
""")

(vbox
  (hbox
    (icon "\uf017" :size 13 :color "#22d3ee")
    (label "циферблат" :size 12 :mono true :grow true :color "#5b7080")
    (when show_seconds (label "с секундами" :size 11 :mono true :color "#22d3ee"))
    (else (label "без секунд" :size 11 :mono true :color "#5b7080")))
  (custom dial)
  (card
    (hbox
      (label "{hour}:{minute}" :size 44 :bold true :mono true :grow true :color "#e2e8f0")
      (label ":{second}" :size 22 :mono true :color "#22d3ee")))
  (hbox
    (label "час" :size 11 :mono true :color "#5b7080")
    (slider :bind hour :range (0 23) :grow true)
    (label "{hour}" :size 12 :mono true :color "#e2e8f0"))
  (hbox
    (label "мин" :size 11 :mono true :color "#5b7080")
    (slider :bind minute :range (0 59) :grow true)
    (label "{minute}" :size 12 :mono true :color "#e2e8f0"))
  (hbox
    (label "сек" :size 11 :mono true :color "#5b7080")
    (slider :bind second :range (0 59) :grow true)
    (label "{second}" :size 12 :mono true :color "#e2e8f0"))
  (hbox
    (checkbox "показывать секундную" :bind show_seconds)
    (button "сейчас" (do (set! hour 21) (set! minute 42) (set! second 0)) :icon "\uf017")))
