---
name: design-styles
description: Use when a lazr screen needs a look: 42 named UI styles with ready theme lines.
---

# Стили интерфейса для lazr

Модель умеет менять вид одной строкой. Здесь — 42 стиля, сгруппированных по эпохе и настроению:
у каждого есть **готовая строка `theme`**, которую можно вставить в модель как есть.

```
theme dark accent:#69b4ff panel:#0b0f14 card:#151b24 muted:#7d8798 font:15 spacing:10 round:10
```

Ключи темы: `dark|light`, `accent`, `panel` (фон окна), `card` (фон карточки), `muted` (вторичный текст
и линии), `font` (кегль), `spacing`, `round`. Работает в генераторе и в `lazr live`.

## Как выбирать

1. **Сначала стиль, потом виджеты.** Стиль задаёт `round`, `spacing` и палитру — от них зависит, будут ли
   `card` и кнопки выглядеть как задумано.
2. **Проверяй кадром.** `lazr live FILE` — правишь строку темы и видишь результат без пересборки.
3. **Чего lazr пока не умеет**: градиенты, размытие, тени, текстуры, свои шрифты интерфейса (кроме шрифта
   иконок). Если стиль держится на градиенте (Frutiger Aero, Aurora) — бери его палитру и форму, а градиент
   заменяй плоским фоном `panel`; это честная адаптация, а не имитация пиксель-в-пиксель.

## Группа 1. Глянец и объём (2001–2012)

| Стиль | Что узнаваемо | Тема |
|---|---|---|
| **Frutiger Aero** | глянец, вода, зелень, белые блики, воздух | `theme light accent:#00a2e8 panel:#e8f4fb card:#ffffff muted:#5f7d8c font:15 spacing:10 round:14` |
| **Frutiger Metro** | город, силуэты, плоские фигуры, бирюза и оранж | `theme light accent:#f26d21 panel:#f4f4f2 card:#ffffff muted:#6b6b6b font:15 spacing:10 round:8` |
| **Aqua (macOS 10.0)** | полосы, гель, синие капсулы, тонкие тени | `theme light accent:#1f7cf0 panel:#eceff3 card:#ffffff muted:#6e7787 font:14 spacing:9 round:12` |
| **Vista Aero Glass** | тёмное стекло, яркая рамка, блики | `theme dark accent:#59b7ff panel:#0f1b26 card:#16242f muted:#7e93a3 font:14 spacing:9 round:8` |
| **Web 2.0 gloss** | «стеклянные» кнопки, отражения, сочные цвета | `theme light accent:#2b8de0 panel:#f7f9fb card:#ffffff muted:#6a7683 font:14 spacing:9 round:10` |
| **iOS 6 skeuomorphic** | текстуры, тёмная сталь, синий глянец | `theme dark accent:#0a84ff panel:#2b2f36 card:#3a4048 muted:#9aa2ad font:15 spacing:10 round:9` |
| **Neumorphism** | мягкие выпуклости, всё одного цвета | `theme light accent:#7c8aa1 panel:#e3e7ee card:#e9edf3 muted:#7b8494 font:15 spacing:12 round:12` |
| **Glassmorphism** | цветной фон + полупрозрачные панели | `theme dark accent:#a371f7 panel:#141021 card:#1d1730 muted:#8d84a8 font:15 spacing:12 round:16` |

## Группа 2. Плоское и корпоративное (2014–2024)

| Стиль | Что узнаваемо | Тема |
|---|---|---|
| **Material 3** | большие радиусы, тональные поверхности, фиолетовый акцент | `theme light accent:#6750a4 panel:#fef7ff card:#f3edf7 muted:#79747e font:15 spacing:12 round:16` |
| **Fluent 2 (Windows 11)** | мягкая тень, светлый акрил, синий | `theme light accent:#0f6cbd panel:#f3f3f3 card:#ffffff muted:#616161 font:14 spacing:10 round:8` |
| **Carbon (IBM)** | строгая сетка, плотные строки, синий IBM | `theme light accent:#0f62fe panel:#ffffff card:#f4f4f4 muted:#525252 font:14 spacing:8 round:2` |
| **Ant Design** | синий акцент, воздушные формы | `theme light accent:#1677ff panel:#ffffff card:#fafafa muted:#8c8c8c font:14 spacing:10 round:6` |
| **Atlassian** | плотная рабочая панель, хлорный синий | `theme light accent:#0052cc panel:#fafbfc card:#ffffff muted:#6b778c font:14 spacing:9 round:4` |
| **Swiss / International** | белое поле, чёрный текст, красный акцент, без украшений | `theme light accent:#e2231a panel:#ffffff card:#f5f5f5 muted:#555555 font:15 spacing:12 round:0` |
| **Минимализм** | максимум воздуха, один акцент | `theme light accent:#111111 panel:#fdfdfd card:#f4f4f4 muted:#8a8a8a font:16 spacing:16 round:8` |
| **Дизайн-система Stripe** | воздух, глубокий фиолетовый, аккуратные тени | `theme light accent:#635bff panel:#ffffff card:#f6f8fa muted:#6e7787 font:15 spacing:12 round:10` |

## Группа 3. Грубость и печать

| Стиль | Что узнаваемо | Тема |
|---|---|---|
| **Web Brutalism** | острые углы, толстые рамки, чёрный текст на белом | `theme light accent:#000000 panel:#ffffff card:#ffffff muted:#333333 font:16 spacing:14 round:0` |
| **Neobrutalism** | толстая чёрная обводка, кислотные заливки, сдвиг | `theme light accent:#ff5c00 panel:#fff8e7 card:#ffe66d muted:#1a1a1a font:16 spacing:12 round:2` |
| **Memphis** | яркие пятна, геометрия 80-х, беспорядок | `theme light accent:#ff2d95 panel:#fff5f8 card:#ffffff muted:#6a3b52 font:15 spacing:12 round:14` |
| **Editorial / журнал** | крупные заголовки, серифный ритм, много воздуха | `theme light accent:#8b1e1e panel:#fbf8f3 card:#ffffff muted:#6b6257 font:16 spacing:14 round:2` |
| **Zine / DIY** | ксерокс, неровность, чёрно-белое | `theme light accent:#000000 panel:#f2f2f2 card:#ffffff muted:#4a4a4a font:15 spacing:11 round:0` |
| **Bauhaus** | первичные цвета, круг/квадрат/треугольник | `theme light accent:#d62b1f panel:#f5f1e8 card:#ffffff muted:#4a4a4a font:15 spacing:12 round:0` |
| **Art Deco** | золото, тёмный фон, геометрия | `theme dark accent:#c9a227 panel:#12100c card:#1c1913 muted:#8f8264 font:15 spacing:12 round:2` |

## Группа 4. Тёмные палитры

| Стиль | Что узнаваемо | Тема |
|---|---|---|
| **Nord** | холодные серо-синие, приглушённость | `theme dark accent:#88c0d0 panel:#2e3440 card:#3b4252 muted:#7b8ba3 font:15 spacing:10 round:8` |
| **Dracula** | фиолетово-розовый на графите | `theme dark accent:#bd93f9 panel:#282a36 card:#343746 muted:#8b93b0 font:15 spacing:10 round:10` |
| **Solarized Dark** | бирюза и песок на глубоком синем | `theme dark accent:#268bd2 panel:#002b36 card:#073642 muted:#839496 font:15 spacing:10 round:6` |
| **Gruvbox** | тёплая охра, ретро-контраст | `theme dark accent:#fabd2f panel:#282828 card:#3c3836 muted:#a89984 font:15 spacing:10 round:6` |
| **Tokyo Night** | глубокий синий, неон-лаванда | `theme dark accent:#7aa2f7 panel:#1a1b26 card:#24283b muted:#7b85a8 font:15 spacing:10 round:10` |
| **Catppuccin Mocha** | пастель на тёмном, мягкие акценты | `theme dark accent:#cba6f7 panel:#1e1e2e card:#313244 muted:#9399b2 font:15 spacing:11 round:12` |
| **Cyberpunk** | чёрный + неон-розовый/циан | `theme dark accent:#ff2e88 panel:#0a0a0f card:#15121c muted:#6f6a86 font:15 spacing:11 round:4` |
| **OLED black** | чёрный фон, минимум цвета | `theme dark accent:#39d353 panel:#000000 card:#0b0b0b muted:#666666 font:15 spacing:11 round:10` |

## Группа 5. Ретро-компьютеры

| Стиль | Что узнаваемо | Тема |
|---|---|---|
| **Windows 95/98** | серые панели, острые углы, синий заголовок | `theme light accent:#000080 panel:#d4d0c8 card:#c0c0c0 muted:#404040 font:13 spacing:6 round:0` |
| **Windows 3.1** | ещё грубее, белые поля, синяя рамка | `theme light accent:#0000a8 panel:#c0c0c0 card:#ffffff muted:#3a3a3a font:13 spacing:6 round:0` |
| **Mac System 7** | чёрно-белое, полосатые заголовки, 1px | `theme light accent:#000000 panel:#dddddd card:#ffffff muted:#555555 font:13 spacing:8 round:0` |
| **NeXTSTEP** | тёмно-серый, объёмные кнопки, строгий | `theme dark accent:#8c8c8c panel:#4a4a4a card:#5c5c5c muted:#b0b0b0 font:14 spacing:9 round:3` |
| **BeOS** | жёлтые акценты, аккуратная геометрия | `theme light accent:#f2b31c panel:#eceff1 card:#ffffff muted:#5f6b73 font:14 spacing:9 round:5` |
| **Amiga Workbench** | синий фон, белое и оранжевое | `theme dark accent:#ff8c1a panel:#0b3b8c card:#1b4fa8 muted:#9db6e0 font:14 spacing:9 round:0` |
| **DOS / Turbo Pascal** | синий экран, жёлтый текст, моноширинность | `theme dark accent:#ffcc00 panel:#0000a8 card:#000080 muted:#8fa8e0 font:14 spacing:7 round:0` |
| **CRT green** | зелёное свечение на почти чёрном | `theme dark accent:#33ff66 panel:#031a0a card:#062712 muted:#3f8f5a font:15 spacing:9 round:2` |
| **Amber CRT** | янтарные символы, терминал | `theme dark accent:#ffb000 panel:#191000 card:#241800 muted:#9c6f1f font:15 spacing:9 round:2` |

## Группа 6. Настроение и цвет

| Стиль | Что узнаваемо | Тема |
|---|---|---|
| **Aurora** | полярное сияние: бирюза → фиолет | `theme dark accent:#38f9d7 panel:#0b1020 card:#141a30 muted:#7f8ab0 font:15 spacing:12 round:16` |
| **Pastel soft** | пастель, низкий контраст, мягкие формы | `theme light accent:#f4a7b9 panel:#fdf6f7 card:#ffffff muted:#8a7f86 font:15 spacing:12 round:14` |
| **Claymorphism** | «глиняные» пухлые блоки | `theme light accent:#7b61ff panel:#eef0ff card:#f7f5ff muted:#7c7a99 font:15 spacing:12 round:18` |
| **Vaporwave** | розовый/циан, сетка, ностальгия | `theme dark accent:#ff71ce panel:#1b0033 card:#2b0052 muted:#9b7bb8 font:15 spacing:12 round:8` |
| **Synthwave** | закат, неон, сетка | `theme dark accent:#ff2e97 panel:#12002b card:#1d0044 muted:#8a6bb0 font:15 spacing:11 round:10` |
| **Y2K chrome** | серебро, блики, «будущее 2000» | `theme light accent:#3d5afe panel:#dfe3ea card:#ffffff muted:#6f7683 font:15 spacing:10 round:12` |
| **Terminal mono** | чёрное на белом или белое на чёрном, без украшений | `theme dark accent:#ffffff panel:#111111 card:#1a1a1a muted:#888888 font:14 spacing:9 round:0` |

## Рецепт под стиль

Что обычно докручивают, чтобы стиль «читался»:

- **Ретро (Win95, System 7, DOS)**: `round:0`, `spacing` 6–8, кегль 13–14, панели-«окошки» через `card`,
  кнопки одинаковой ширины, никаких иконок из FA — только текст.
- **Глянец (Aqua, Vista, Web 2.0)**: `round` 10–14, `panel` светлее `card`, акцент насыщенный, иконки из
  внешнего TTF.
- **Плоское (Material, Fluent)**: `round` 8–16, `spacing` 10–12, кегль 14–15, много `card`.
- **Брутализм и печать**: `round:0–2`, толстые разделители (`separator`), крупный кегль, мало цвета.
- **Тёмные палитры**: `panel` темнее `card`, `muted` заметно светлее фона, акцент — единственный яркий цвет.

## Проверка

```bash
lazr live examples/player.lisp      # правишь строку theme — окно меняется сразу
lazr build DIR && lazr verify DIR   # сборка + кадр + действия
```

Стиль — это по-прежнему данные: одна строка в модели, ноль правок в Rust.
