; Задачи — список строк: append / remove / clear, счётчики и поле ввода.
(window "Задачи" 480 560)
(theme dark :accent "#388bfd" :font 14 :spacing 8 :round 8)
(state items list)
(state draft string "")
(state done int 0)
(vbox
  (label "Задачи" :size 24 :bold true)
  (label "сделано: {done} · в списке: {items}" :size 12 :color "#8b949e")
  (hbox
    (text-input "что сделать" :bind draft :grow true)
    (button "Добавить" (do (append items draft) (set! draft ""))))
  (list :bind items "пусто — добавьте первую задачу")
  (hbox
    (button "Готово" (set! done (+ done 1)))
    (button "Сброс" (set! done 0))
    (button "Очистить" (do (clear items) (set! done 0)))))
