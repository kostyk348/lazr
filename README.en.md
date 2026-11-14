<p align="center">
  <img src="docs/logo.svg" alt="lazr" width="132">
</p>

<h1 align="center">lazr</h1>

<p align="center">
  RAD for Rust that is friendly to AI: screen model → real egui/eframe code → build → verification by running.<br>
  The model is written as line forms <em>or</em> S-expressions. Rust stays the HAL: window, memory, pixels.
</p>

<p align="center">
  <a href="README.md">Русский</a> · <b>English</b>
</p>

<p align="center">
  <a href="https://github.com/kostyk348/lazr/actions"><img src="https://github.com/kostyk348/lazr/actions/workflows/tests.yml/badge.svg" alt="tests"></a>
  <a href="https://kostyk348.github.io/lazr/"><img src="https://img.shields.io/badge/site-lazr-ffb86b" alt="pages"></a>
</p>

---

## Why

Lazarus gives a human a canvas: drag a button, double-click it, write a handler. An agent needs no canvas — it
needs a **semantic** way to change the interface and a **verifiable** result. `lazr` does exactly that:

```
screen model (form.lazr / *.lisp)  →  a real egui/eframe project  →  cargo build  →  verify
```

No mouse dragging, no "generate main.rs for me". Every edit is a named operation on the model; every error is
data with a line number, a code and a hint. Generation is deterministic: one model — one and the same text, so
the generated code lives in git and diffs cleanly.

```bash
lazr new counter --title "Counter" --size 420x260
lazr state counter count int 0
lazr add counter label "Clicks: {count}"
lazr add counter button "Plus" --do "count += 1" --style "icon:play, key:ctrl+plus"
lazr verify counter        # build + a frame without a window + run every action
lazr mcp                   # the same for an agent, over MCP
```

## What the model can do

| What | Line form | S-expression |
|---|---|---|
| window | `window "Paint" 900x720` | `(window "Paint" 900 720)` |
| fields | `state size float 4` | `(state size float 4)` |
| text / buttons | `label "x" {size:22, bold}` | `(label "x" :size 22 :bold true)` |
| action | `button "Plus" do count += 1` | `(button "Plus" (set! count (+ count 1)))` |
| condition | `do if count >= 9 { count = 0 } else { count += 1 }` | `(if (>= count 9) (set! count 0) (set! count (+ count 1)))` |
| layout | `vbox`, `hbox`, `grid 2`, `scroll`, `space 12` | `(vbox …)`, `(grid 2 …)` |
| absolute | `place 800x480` + `at 16 50 size 240x170` | `(place 800 480 (vbox :at 16 50 :box 240 170 …))` |
| anchors | `anchor right bottom at 16 16` | `:anchor "right bottom"` |
| theme | `theme dark accent:#388bfd font:15 spacing:10 round:8` | `(theme dark :accent "#388bfd" :font 15)` |
| style | `{size:22, bold, color:#e5484d, pad:12, align:center, grow}` | `:size 22 :color "#e5484d" :pad 12` |
| icon | `{icon:save}` | `:icon "save"` |
| shortcut | `{key:ctrl+s}` | `:key "ctrl+s"` |
| custom widget | `rust widget badge { … }` + `custom badge` | `(rust-widget badge """…""")` + `(custom badge)` |
| custom field | `rust field strokes Vec<Stroke> Vec::new()` | `(rust-field strokes "Vec<Stroke>" "Vec::new()")` |
| custom function | `rust fn parse_hex { … }` | `(rust-fn parse_hex """…""")` |
| startup code | `rust init { … }` | `(rust-init """…""")` |

Model widgets: `label`, `button`, `checkbox`, `text_input`, `slider`, `separator`, `space`, `vbox`, `hbox`,
`grid`, `scroll`, `place`, `custom`. Icons are a built-in set of safe names (`save`, `open`, `undo`, `pencil`,
`brush`, `play`, `check`, `close`, …) with no external dependencies: glyphs come from egui's bundled fonts.

Actions: `field += N`, `field -= N`, `field = N`, `bool = !bool`, `string = "text"`, plus `if` with arithmetic
and comparisons. Bindings: `checkbox -> bool`, `text_input -> string`, `slider -> number`; types are checked
before compilation.

Why this is AI-friendly:

* **text instead of pixels** — the model diffs in git, and the diff shows what the agent did;
* **an error is data**: `{"line":3,"code":"UNKNOWN_WIDGET","message":"…","hint":"available: …"}`;
* **types are checked before compilation**: `text_input -> count` is rejected at once, not after three minutes of `cargo`;
* **deterministic generation**: one model — the same code;
* **verification without a screen**: `--headless` runs an egui frame, `--selftest` calls every action;
* **zero dependencies for the tool itself**: `lazr` is plain-stdlib Rust; the generated app needs only `egui`/`eframe`.

## Layout: like a form, not by eye

`place` is a fixed-size area where coordinates are absolute, and `anchor` measures them from the right/bottom
edge. This was the missing piece: dashboards, canvases and panels are now placed with numbers.

```lisp
(place 800 480
  (label "Dashboard" at 16 10 size 200x28 :size 22 :bold true)
  (vbox :box 240 170 :at 16 50 (custom card))
  (button "Recount" at 16 436 size 160x32 :icon "refresh" :key "ctrl+r")
  (button "Clear" anchor right bottom at 16 16 size 130x32 :icon "delete"))
```

The theme is one line per model: dark/light base, accent, font size, spacing, corner radius. It compiles into
`fn apply_theme(ctx)` and is applied both in the window and in the headless frame — so the theme is verified,
not just declared.

![Dashboard](docs/dashboard.png)

## Packaging

```console
$ lazr package /tmp/paint --release
binary: /tmp/paint/dist/kraski
icon:   /tmp/paint/dist/icon.png
entry:  /tmp/paint/dist/kraski.desktop
archive:/tmp/paint/dist/kraski-release.tar.gz
```

The package name comes from the model title, transliterated when it is Cyrillic (`Краски` → `kraski`) instead of
becoming a faceless `app`. `dist/` gets the binary, `icon.png` (rendered from `docs/logo.svg` via
`rsvg-convert`), a `.desktop` entry with `Name` from the model and a tar.gz. There is no installer and no
signature — an honest boundary, not a forgotten checkbox.

## Verification — three facts, not a promise

```console
$ lazr verify /tmp/paint
build: ok
headless ok: 53 shapes, 18 widgets in the model
  action "Plus": ok
  action "Reset": ok
selftest ok: 2 actions
result: verified
```

`verify` = compilation + one frame without a window + a call to **every** button action. Custom widgets are
covered by the frame: their code runs inside `headless`, so a panic shows up immediately.

## Demo: Paint

A full Paint, described entirely by the model: eight tools (pencil, brush, eraser, line, rectangle, ellipse,
text, fill), a palette, thickness, undo (Ctrl+Z), save and open, shortcuts 1–8.

The fill is not "paint everything": the canvas is a raster, walls are built from every stroke, and a
breadth-first walk paints only the area inside a closed contour. The file is PPM; its path sits in the "файл"
row and can be edited, `F5` writes it and `F6` reads it back. `Ctrl+S` and `Ctrl+O` (also `F7`) raise the system
`zenity` dialog — a convenience, not the only way.

Verification is a script, not an eyeball: `scripts/check-paint.py`:

```console
$ python3 scripts/check-paint.py
canvas in the window: 11,190
inside (27, 31, 36) · outside (255, 255, 255) → fill OK
saved: /tmp/paint.ppm · 1053375 bytes · 836x420 → OK
cleared True · reading restored the picture True → round-trip OK
open dialog: raised

RESULT:
  contour fill: works
  writing a file: works
  reading a file (round-trip): works
  system open dialog: works
```

The adversarial run `scripts/adversarial-paint.py` (attacks on the controls): undo on an empty history, undo
after undo, clear, 12 rapid clicks, resize — every attack `works`, no panics.

![Paint](docs/paint.png)

The source is `examples/paint.lisp`: about 300 lines, of which ~250 are the canvas Rust code inside
`rust-item`; the rest is DSL forms.

```lisp
(window "Paint" 900 720)
(state tool string "brush")
(state size float 4)
(rust-field strokes "Vec<Stroke>" "Vec::new()")
(vbox :pad 12
  (hbox (label "Paint" :size 20 :bold true) (custom toolbar))
  (custom palette)
  (hbox (custom actions) (slider "thickness" :bind size :range (1 40)))
  (custom canvas)
  (custom status))
```

## `lazr live` — the model is read while the app runs

```console
$ lazr live examples/live-demo.lazr
```

One runtime is built (a template, not generated per screen); it is then launched with the path to a model and
re-reads the file whenever it changes: edit the model, the window changes, no rebuild. This is the first step
towards "Rust is the HAL, everything else is a DSL".

Supported today: window, states, `label`, `button`, `checkbox`, `text_input`, `slider`, `space`, `separator`,
`vbox`, `hbox`, `grid`, `scroll`, `{size, bold, color}` styles, `{field}` interpolation.

**Honest status:** hot reload is verified — an external edit changes the window (new rows and a green heading
appeared without a rebuild). Button actions in the runtime are **not confirmed**: the counter did not change in
the runs, so they cannot be called working. The generator (`lazr build`) does run actions correctly; the gap is
in the runtime, and that is the next step.

## Install

```console
$ cargo install --path .          # or: cargo build --release && cp target/release/lazr ~/.local/bin/
$ lazr new hello --title "Hello" --size 420x260
$ lazr state hello count int 0
$ lazr add hello label "Clicks: {count}"
$ lazr add hello button "Plus" --do "count += 1" --style "icon:play, key:ctrl+plus"
$ lazr verify hello
```

S-expressions:

```console
$ lazr lisp examples/paint.lisp                    # model as text
$ lazr lisp examples/paint.lisp --to /tmp/paint    # model + project + generated code
```

A shared `CARGO_TARGET_DIR` (`LAZR_TARGET_DIR`, defaults to `~/.cache/lazr/target`) saves minutes: the agent
does not recompile egui on every step.

## MCP

`lazr mcp` — JSON-RPC over stdin/stdout, eight tools:

| tool | what it does |
|---|---|
| `form_new` | create a project (model + app) |
| `form_state` | declare a state field |
| `form_add` | add a widget (kind, text, binding, action, container) |
| `form_show` | show the model as text |
| `form_widgets` | list widgets with paths `/0/1` |
| `form_generate` | regenerate the Rust code |
| `form_build` | build (`cargo build --offline`) and return the log tail |
| `form_headless` | run one frame and return the shape count |

Replies are always JSON; errors are `isError: true` with a code and a hint so the agent can fix itself.

## How this differs from what already exists

| Tool | What it does | What is missing |
|---|---|---|
| **Slint** (1.17+) | built-in MCP: inspects the live window tree, clicks, takes screenshots, headless backend | that is **inspection and control**, not authorship: the screen is still written by hand in `.slint` |
| **Ruxel Studio** (VS Code, alpha) | two-way code/canvas editing (`syn`/`quote`) for iced/egui/slint/dioxus | lives in the editor, no MCP, no verification by running |
| **iced** (`comet`, `cargo-hot`) | debugger and hot reload | for humans, not for agents |
| **Lazarus/Delphi** | components, events, `.lfm` as a form model | not Rust, no MCP, verification only by a human |

`lazr` fills exactly the **authorship** gap: a model + operations + deterministic generation + verification by
build, frame and action run.

## What is still missing (honestly)

* **layout**: `vbox`/`hbox`/`grid`/`scroll`/`place`, absolute points, sizes and right/bottom anchors exist;
  percentages of the container, Qt-style edge constraints and reflow on window resize do not;
* **styles**: size, weight, color, padding, alignment and a theme (font, accent, spacing, radius) exist; custom
  font files, shadows and background images do not;
* **custom widgets**: a custom widget is a block of Rust code; there is no DSL for its internals;
* **handlers**: conditions and arithmetic exist, function calls and lists do not — anything beyond that moves
  into `rust widget`, and then it is no longer the model;
* **model history**: git exists, a built-in undo/branching of the model does not;
* **packaging**: binary, icon, `.desktop` and tar.gz exist; an installer and a signature do not — a signature
  needs keys, not a checkbox;
* **runtime Lisp**: S-expressions are the author's language today, not an interpreter inside the app; a real
  Lisp machine (editing forms without recompiling, hot reload) is the next step.

## Tests

```bash
cargo test        # 37 tests: model, layout, theme, action language, S-expressions, generation
cargo run -- demo # end-to-end: model → code → build → headless
```

License: MIT.
