; Синтезатор: настоящий звук без зависимостей — генерируем WAV сами и играем
; его paplay (или ffplay/aplay). Клавиши рисуются и ловятся одним Rust-виджетом.
(window "Синтезатор" 760 460 :chrome own :min "700x420")
(theme dark accent:#22d3ee panel:#0b0f16 card:#111826 muted:#8ea2b5 font:15 spacing:10 round:12)
(icon-font "/usr/share/fonts/TTF/fa-solid-900.ttf")
(state notes list)
(state played int 0)

(rust-widget piano """
// --- звучание: генерируем WAV в /tmp и играем внешним плеером --------------
fn write_wav(path: &str, freq: f32, dur: f32) {
    use std::f32::consts::TAU;
    let rate = 44100usize;
    let samples = (rate as f32 * dur) as usize;
    let mut pcm = Vec::with_capacity(samples * 2);
    for t in 0..samples {
        let s = t as f32 / rate as f32;
        // огибающая: быстрый щелчок-атака и плавное затухание
        let env = (1.0 - s / dur).powf(1.8) * (1.0 - (-s * 14.0).exp());
        let base = (TAU * freq * s).sin();
        let overtone = (TAU * freq * 2.0 * s).sin() * 0.35;
        let sparkle = (TAU * freq * 3.0 * s).sin() * 0.12;
        let value = env * (base + overtone + sparkle) * 11500.0;
        pcm.extend_from_slice(&(value.clamp(-32000.0, 32000.0) as i16).to_le_bytes());
    }
    let mut wav = Vec::with_capacity(44 + pcm.len());
    wav.extend_from_slice(b"RIFF");
    let body = (36 + pcm.len()) as u32;
    wav.extend_from_slice(&body.to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&(rate as u32).to_le_bytes());
    wav.extend_from_slice(&(rate as u32 * 2u32).to_le_bytes()); // байт/сек
    wav.extend_from_slice(&2u16.to_le_bytes()); // байт на сэмпл
    wav.extend_from_slice(&16u16.to_le_bytes()); // бит
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
    wav.extend_from_slice(&pcm);
    let _ = std::fs::write(path, wav);
}

fn play(freq: f32, dur: f32, path: &str) {
    write_wav(path, freq, dur);
    let mut ok = false;
    for player in ["paplay", "ffplay", "aplay"] {
        let mut command = std::process::Command::new(player);
        if player == "ffplay" {
            command.arg("-nodisp").arg("-autoexit").arg("-loglevel").arg("quiet");
        }
        command.arg(path);
        if let Ok(child) = command
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            std::mem::forget(child); // сэмпл короткий, живёт сам
            ok = true;
            break;
        }
    }
    if !ok {
        eprintln!("ни paplay, ни ffplay, ни aplay не запустились");
    }
}

let note_of = |semitone: i32| 440.0 * (2.0f32).powf((semitone - 9) as f32 / 12.0);
let dur = 0.55f32;

// --- клавиатура: белые и чёрные клавиши одной областью ----------------------
let width = ui.available_width();
let (rect, _hit) = ui.allocate_exact_size(egui::vec2(width, 170.0), egui::Sense::click());
let painter = ui.painter_at(rect);
painter.rect_filled(rect, egui::CornerRadius::same(10), egui::Color32::from_rgb(10, 15, 22));
let whites: [(&str, i32); 8] = [("до", 0), ("ре", 2), ("ми", 4), ("фа", 5), ("соль", 7), ("ля", 9), ("си", 11), ("до2", 12)];
let white_w = rect.width() / 8.0;
for (index, (name, semi)) in whites.iter().enumerate() {
    let key = egui::Rect::from_min_max(
        egui::pos2(rect.left() + white_w * index as f32 + 2.0, rect.top() + 6.0),
        egui::pos2(rect.left() + white_w * (index + 1) as f32 - 2.0, rect.bottom() - 8.0),
    );
    let hit = ui.interact(key, ui.scope_id().with(("key", index)), egui::Sense::click());
    let tone = note_of(*semi);
    painter.rect_filled(key, egui::CornerRadius::same(6), egui::Color32::from_rgb(238, 241, 246));
    if hit.hovered() {
        painter.rect_filled(key, egui::CornerRadius::same(6), egui::Color32::from_rgba_unmultiplied(34, 211, 238, 60));
    }
    if hit.clicked() {
        painter.rect_filled(key, egui::CornerRadius::same(6), egui::Color32::from_rgb(34, 211, 238));
        state.played += 1;
        let path = format!("/tmp/lazr-note-{}.wav", index);
        let freq = tone;
        std::thread::spawn(move || play(freq, dur, &path));
    }
    painter.text(
        egui::pos2(key.center().x, key.bottom() - 14.0),
        egui::Align2::CENTER_CENTER,
        *name,
        egui::FontId::proportional(10.0),
        egui::Color32::from_rgb(40, 60, 80),
    );
}
// чёрные клавиши поверх
let blacks: [(i32, f32); 5] = [(1, 1.0), (3, 2.0), (6, 4.0), (8, 5.0), (10, 6.0)];
for (index, (semi, slot)) in blacks.iter().enumerate() {
    let key = egui::Rect::from_min_max(
        egui::pos2(rect.left() + white_w * slot - 8.0, rect.top() + 6.0),
        egui::pos2(rect.left() + white_w * slot + 8.0, rect.bottom() - 78.0),
    );
    let hit = ui.interact(key, ui.scope_id().with(("black", index)), egui::Sense::click());
    let tone = note_of(*semi);
    painter.rect_filled(key, egui::CornerRadius::same(4), egui::Color32::from_rgb(20, 26, 34));
    painter.rect_stroke(key, egui::CornerRadius::same(4), egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 70)), egui::StrokeKind::Inside);
    if hit.hovered() {
        painter.rect_filled(key, egui::CornerRadius::same(4), egui::Color32::from_rgba_unmultiplied(34, 211, 238, 90));
    }
    if hit.clicked() {
        painter.rect_filled(key, egui::CornerRadius::same(4), egui::Color32::from_rgb(34, 211, 238));
        state.played += 1;
        let path = format!("/tmp/lazr-note-b{}.wav", index);
        let freq = tone;
        std::thread::spawn(move || play(freq, 0.4, &path));
    }
}

// нижняя полоса: кнопка мелодии — отдельное выделение, честный клип
let (strip_rect, strip_hit) = ui.allocate_exact_size(egui::vec2(width, 34.0), egui::Sense::click());
let painter2 = ui.painter_at(strip_rect);
let over = if strip_hit.hovered() { egui::Color32::from_rgb(26, 42, 58) } else { egui::Color32::from_rgb(20, 30, 42) };
painter2.rect_filled(strip_rect, egui::CornerRadius::same(10), over);
painter2.rect_stroke(strip_rect, egui::CornerRadius::same(10), egui::Stroke::new(1.0, egui::Color32::from_rgb(34, 211, 238)), egui::StrokeKind::Inside);
painter2.text(
    egui::pos2(strip_rect.left() + 14.0, strip_rect.center().y),
    egui::Align2::LEFT_CENTER,
    "\u{f04b}  играть элизию — первые восемь нот",
    egui::FontId::proportional(13.0),
    egui::Color32::from_rgb(235, 240, 250),
);
if strip_hit.clicked() {
    // «К Элизе», первые восемь нот: E5 D#5 E5 D#5 E5 B4 D5 C5 A4
    let melody: [(f32, f32); 8] = [
        (note_of(4) * 2.0, 0.22), (note_of(3) * 2.0, 0.22), (note_of(4) * 2.0, 0.22),
        (note_of(3) * 2.0, 0.22), (note_of(4) * 2.0, 0.22), (note_of(7) * 1.0, 0.22),
        (note_of(2) * 1.0, 0.22), (note_of(0) * 1.0, 0.22),
    ];
    state.played += melody.len() as i64;
    std::thread::spawn(move || {
        for (index, (freq, dur)) in melody.iter().enumerate() {
            let path = format!("/tmp/lazr-melody-{}.wav", index);
            write_wav(&path, *freq, *dur + 0.15);
            let mut played = false;
            for player in ["paplay", "ffplay", "aplay"] {
                let mut command = std::process::Command::new(player);
                if player == "ffplay" {
                    command.arg("-nodisp").arg("-autoexit").arg("-loglevel").arg("quiet");
                }
                command.arg(&path);
                if command.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().is_ok() {
                    played = true;
                    break;
                }
            }
            if !played {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis((*dur * 1000.0) as u64));
        }
    });
}
""")

(vbox
  (hbox
    (icon "\uf001" :size 13 :color "#22d3ee")
    (label "синтезатор" :size 13 :mono true :grow true :color "#8ea2b5")
    (label "{played} нажатий" :size 12 :mono true :color "#22d3ee"))
  (custom piano)
  (card
    (vbox
      (hbox
        (label "клавиши" :size 12 :mono true :color "#8ea2b5")
        (label "· нажимаются мышью" :size 12 :mono true :grow true :color "#5b7080"))
      (hbox
        (label "клавиши нажимаются мышью, «элизия» — кнопкой в самом piano" :size 12 :mono true :color "#8ea2b5"))))
  (label "звук идёт через paplay (PulseAudio). Ни одной зависимости: WAV пишется своими руками." :size 11 :mono true :color "#5b7080"))
