; Кино — кинематографичный тёмный стиль: таймлайн, скорость, субтитры, отметка «досмотрел».
(window "Кино" 640 520)
(theme dark accent:#e50914 panel:#0a0a0c card:#141418 muted:#8c8c94 font:15 spacing:10 round:8)
(state time int 42)
(state playing bool false)
(state speed int 1)
(state subs bool true)
(state watched int 0)
(vbox
  (hbox
    (label "КИНО" :size 20 :bold true :color "#e50914")
    (label "ночной сеанс" :size 12 :grow true :color "#8c8c94")
    (when subs (label "субтитры вкл" :size 11 :color "#ffb86b")))
  (card
    (label "Медленные реки" :size 22 :bold true :color "#f5f5f7")
    (label "2024 · драма · 128 мин" :size 12 :color "#8c8c94")
    (when playing (label "▶ идёт" :size 12 :color "#39d353"))
    (else (label "⏸ пауза" :size 12 :color "#e5484d"))
    (slider "таймлайн" :bind time :range (0 128))
    (hbox
      (label "{time} мин" :size 12 :color "#f5f5f7")
      (label "из 128" :size 12 :grow true :color "#8c8c94")
      (label "скорость {speed}x" :size 12 :color "#8c8c94")))
  (hbox
    (button "-10" (if (<= time 10) (set! time 0) (set! time (- time 10))))
    (button "играть" (set! playing true))
    (button "пауза" (set! playing false))
    (button "+10" (if (>= time 118) (set! time 128) (set! time (+ time 10)))))
  (hbox
    (button "1x" (set! speed 1))
    (button "2x" (set! speed 2))
    (button "субтитры" (set! subs true))
    (button "без субтитров" (set! subs false)))
  (separator)
  (hbox
    (button "досмотрел" (do (set! time 128) (set! playing false) (set! watched (+ watched 1))))
    (label "отмечено: {watched}" :size 12 :color "#8c8c94"))
  (when (>= watched 1)
    (label "спасибо — оценка отправлена" :size 12 :color "#39d353"))
  (when (>= time 120)
    (label "почти конец" :size 12 :color "#ffb86b")))
