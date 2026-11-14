; Пульт — демо абсолютной раскладки: place, якоря, тема.
;
; Здесь нет ни одного отступа «на глазок»: каждая карточка стоит в точке,
; сетка пульта задана числами, как в Lazarus-форме. Тема — одна строка.
(window "Пульт" 820 560)

(theme dark accent "#388bfd" font 14 spacing 8 round 8)

(state demand int 17)
(state capacity int 2)
(state refreshed int 0)

(rust-widget card_demand "
let ratio = (state.demand as f32 / state.capacity.max(1) as f32).min(9.9);
ui.label(egui::RichText::new(\"потребность / ёмкость\").size(12.0).color(egui::Color32::from_rgb(139, 148, 158)));
ui.label(egui::RichText::new(format!(\"{:.1}×\", ratio)).size(30.0).strong().color(egui::Color32::from_rgb(229, 72, 77)));
ui.label(egui::RichText::new(format!(\"{} задач на {} воркера\", state.demand, state.capacity)).size(12.0));
")

(place 800 480
  (label "Пульт" at 16 10 size 200x28 :size 22 :bold true)
  (label "обновлено: {refreshed} раз" at 600 16 size 190x20 :size 12 :color "#8b949e")

  (vbox :box 240 170 :at 16 50
    (custom card_demand))

  (vbox :box 240 170 :at 272 50
    (label "ёмкость" :size 12 :color "#8b949e")
    (label "{capacity}" :size 30 :bold true :color "#39d353")
    (label "воркеров в строю" :size 12)
    (space 6)
    (slider "план" :bind capacity :range (1 12)))

  (vbox :box 240 170 :at 528 50
    (label "потребность" :size 12 :color "#8b949e")
    (label "{demand}" :size 30 :bold true :color "#e8833c")
    (label "задач в очереди" :size 12)
    (space 6)
    (hbox
      (button "+1" do demand += 1)
      (button "-1" do demand -= 1)))

  (vbox :box 752 120 :at 16 236
    (label "журнал" :size 12 :color "#8b949e")
    (separator)
    (label "потребность {demand} · ёмкость {capacity}" :size 13)
    (label "пересчёт: {refreshed}" :size 13 :color "#8b949e"))

  (button "Пересчитать" at 16 436 size 160x32 :do "refreshed += 1" :icon "refresh" :key "ctrl+r")
  (button "Очистить" anchor right bottom at 16 16 size 130x32 :do "refreshed = 0" :icon "delete")
  (label "F5 — пересчёт" anchor right bottom at 16 54 size 200x20 :size 11 :color "#8b949e"))
