; Витрина рантайма: этот файл читается на ходу, без пересборки.
(window "Живая модель" 560 420)
(state count int 0)
(state limit int 5)
(state name string "Аня")
(state agree bool false)
(vbox
  (label "ПРАВКА ИЗ .LISP: {count} из {limit}" :size 24 :bold true :color "#39d353")
  (hbox
    (button "Плюс" do "count += 1")
    (button "Минус" do "count -= 1")
    (button "Сброс" do "count = 0"))
  (slider "порог" :bind limit :range (1 20))
  (checkbox "Согласен" :bind agree)
  (text-input "Имя" :bind name :grow true)
  (label "Привет, {name}" :color "#8b949e"))
