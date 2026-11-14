; Викторина — счёт и счётчик попыток.
(window "Викторина" 460 340)
(theme dark :accent "#a371f7" :font 15 :spacing 10 :round 12)
(state score int 0)
(state tries int 0)
(vbox
  (label "Викторина" :size 22 :bold true :color "#a371f7")
  (label "верно {score} из {tries}" :size 13 :color "#8b949e")
  (separator)
  (label "Столица Франции?" :size 16)
  (hbox
    (button "Париж" (do (set! score (+ score 1)) (set! tries (+ tries 1))))
    (button "Лион" (set! tries (+ tries 1))))
  (label "Сколько будет 7 × 8?" :size 16)
  (hbox
    (button "56" (do (set! score (+ score 1)) (set! tries (+ tries 1))))
    (button "54" (set! tries (+ tries 1))))
  (separator)
  (button "сначала" (do (set! score 0) (set! tries 0))))
