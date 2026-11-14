; Настройки — светлая тема и форма.
(window "Настройки" 480 400)
(theme light :accent "#388bfd" :font 15 :spacing 10 :round 6)
(state volume int 60)
(state autosave bool true)
(state name string "Аня")
(vbox
  (label "Настройки" :size 24 :bold true)
  (separator)
  (label "громкость: {volume}" :size 12 :color "#57606a")
  (slider "громкость" :bind volume :range (0 100))
  (checkbox "автосохранение" :bind autosave)
  (space 4)
  (text-input "имя" :bind name :grow true)
  (label "привет, {name}" :color "#57606a")
  (separator)
  (hbox
    (button "сбросить" (do (set! volume 50) (set! name "Аня")))
    (button "тишина" (set! volume 0))))
