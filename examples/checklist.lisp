; Чек-лист — шаблон строки: у каждой строки своя подпись и своя кнопка «убрать».
(window "Чек-лист" 480 420)
(theme dark :accent "#a371f7" :font 15 :spacing 8 :round 8)
(state items list)
(state draft string "")
(state added int 0)
(vbox
  (label "Чек-лист" :size 24 :bold true)
  (label "в списке: {items} · добавлено: {added}" :size 12 :color "#8b949e")
  (hbox
    (text-input "что взять" :bind draft :grow true)
    (button "Добавить" (do (append items draft) (set! draft "") (set! added (+ added 1))) :icon check))
  (separator)
  (list :bind items :empty "пока пусто — добавьте первую строку"
    (hbox
      (label "• {item}" :size 15 :grow true)
      (button "убрать" (remove items) :size 12 :icon delete))
    )
  (separator)
  (when (> added 2)
    (label "записей больше двух" :size 12 :color "#ffb86b"))
  (else
    (label "можно ещё" :size 12 :color "#8b949e")))
