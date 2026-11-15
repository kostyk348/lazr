; Терминал — CRT green: журнал команд, ввод, счётчик; шаблон строки и сиды списка.
(window "Терминал" 560 480)
(theme dark accent:#33ff66 panel:#031a0a card:#062712 muted:#3f8f5a font:14 spacing:9 round:2)
(state log list "boot ok,kernel 6.1,ready")
(state cmd string "uname -a")
(state runs int 0)
(vbox
  (hbox
    (label "term@lazr" :size 16 :bold true :color "#33ff66")
    (label "~" :size 16 :grow true :color "#3f8f5a")
    (label "команд: {runs}" :size 12 :color "#3f8f5a"))
  (separator)
  (list :bind log :empty "журнал пуст"
    (hbox
      (label "> {item}" :size 13 :grow true :color "#33ff66")
      (button "x" (remove log) :size 11)))
  (separator)
  (hbox
    (text-input "команда" :bind cmd :grow true)
    (button "выполнить" (do (append log cmd) (set! cmd "") (set! runs (+ runs 1)))))
  (hbox
    (button "очистить" (clear log))
    (button "сброс счётчика" (set! runs 0))
    (label "журнал: {log}" :size 12 :grow true :color "#3f8f5a")))
