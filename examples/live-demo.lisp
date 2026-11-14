; Витрина рантайма: этот файл читается и исполняется на ходу, без пересборки.
(window "Живая модель" 560 420)
(theme dark :accent "#388bfd" :font 14 :spacing 8 :round 8)
(state count int 0)
(state limit int 5)
(state name string "Аня")
(state agree bool false)

(vbox
  (label "Нажатий: {count} из {limit}" :size 22 :bold true :color "#69b4ff")
  (hbox
    (button "Плюс" (set! count (+ count 1)))
    (button "Минус" (set! count (- count 1)))
    (button "Сброс" (set! count 0)))
  (slider "порог" :bind limit :range (1 20))
  (checkbox "Согласен" :bind agree)
  (text-input "Имя" :bind name :grow true)
  (label "Привет, {name}" :color "#8b949e")
  (separator)
  (grid 2
    (label "счётчик")
    (label "{count}")))
