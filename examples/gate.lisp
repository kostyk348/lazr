; Доступ — условная видимость и сравнение строк: панели появляются по условию.
(window "Доступ" 440 400)
(theme dark :accent "#39d353" :font 15 :spacing 10 :round 10)
(state code string "")
(state tries int 0)
(vbox
  (label "Доступ" :size 24 :bold true)
  (label "попыток: {tries}" :size 12 :color "#8b949e")
  (text-input "введите код" :bind code :grow true)
  (hbox
    (button "Проверить" (set! tries (+ tries 1)) :icon check :key "ctrl+enter")
    (button "Сброс" (do (set! code "") (set! tries 0))))
  (separator)
  (when (== code "42")
    (label "открыто" :size 22 :bold true :color "#39d353"))
  (when (!= code "42")
    (label "закрыто" :size 18 :color "#e5484d"))
  (when (>= tries 3)
    (label "подсказка: 42" :size 12 :color "#ffb86b")))
