; Смета — арифметика в действиях и живой пересчёт.
(window "Смета" 420 340)
(theme dark :accent "#39d353" :font 15 :spacing 10 :round 10)
(state total int 1000)
(state step int 250)
(vbox
  (label "смета" :size 12 :color "#8b949e")
  (label "{total} ₽" :size 30 :bold true :color "#39d353")
  (separator)
  (hbox
    (button "+ шаг" (set! total (+ total step)))
    (button "− шаг" (set! total (- total step))))
  (hbox
    (button "×2" (set! total (* total 2)))
    (button "÷2" (set! total (/ total 2)))
    (button "в ноль" (set! total 0)))
  (slider "шаг" :bind step :range (50 1000))
  (label "шаг: {step}" :size 12 :color "#8b949e"))
