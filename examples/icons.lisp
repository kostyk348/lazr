; Иконки из внешнего шрифта (Font Awesome) — кодемпоинты, а не картинки в репозитории.
(icon-font "/usr/share/fonts/TTF/fa-solid-900.ttf")
(window "Иконки" 520 420)
(theme dark :accent "#69b4ff" :font 15 :spacing 10 :round 10)
(state done int 2)
(vbox
  (label "Иконки из внешнего шрифта" :size 20 :bold true)
  (label "глифы берутся из TTF, а не из egui" :size 12 :color "#8b949e")
  (card
    (hbox
      (icon "\uf00c" :size 20 :color "#39d353")
      (label "готово" :size 15 :grow true)
      (label "{done}" :size 15 :color "#39d353"))
    (hbox
      (icon "\uf017" :size 20 :color "#ffb86b")
      (label "в работе" :size 15 :grow true)
      (label "3" :size 15 :color "#ffb86b"))
    (hbox
      (icon "\uf071" :size 20 :color "#e5484d")
      (label "просрочено" :size 15 :grow true)
      (label "1" :size 15 :color "#e5484d")))
  (card
    (hbox
      (button "+" (set! done (+ done 1)))
      (icon "\uf067" :size 16 :color "#69b4ff")
      (icon "\uf1f8" :size 16 :color "#8b949e")
      (icon "\uf021" :size 16 :color "#8b949e")
      (label "глифы Font Awesome в строке" :size 12 :color "#8b949e")))
  (label "нажмите «+», чтобы изменить счётчик" :size 12 :color "#8b949e"))
