; Термостат — числа и пороги: панели появляются, когда условие выполняется.
(window "Термостат" 440 360)
(theme dark :accent "#ffb86b" :font 15 :spacing 10 :round 10)
(state temp int 18)
(state limit int 25)
(vbox
  (label "Термостат" :size 24 :bold true)
  (label "сейчас {temp}°, порог {limit}°" :size 12 :color "#8b949e")
  (slider "температура" :bind temp :range (0 40))
  (slider "порог" :bind limit :range (10 35))
  (separator)
  ; ветка else: одно условие вместо двух противоположных панелей
  (when (>= temp limit)
    (label "жарко" :size 22 :bold true :color "#e5484d")
    (else
      (label "норма" :size 22 :bold true :color "#39d353")))
  (when (>= temp 35)
    (label "проверьте окно" :size 12 :color "#ffb86b"))
  (button "сбросить" (do (set! temp 18) (set! limit 25))))
