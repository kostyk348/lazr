; Winamp-класс: настоящий плеер на mpv IPC. Пауза, seek, громкость и позиция
; управляются почитай как в VLC — сокет, JSON-команды, std::os::unix::net.
(window "Winamp" 760 520 :chrome own :min "700x480")
(icon-font "/usr/share/fonts/TTF/fa-solid-900.ttf")
(font-family "/usr/share/fonts/TTF/DejaVuSansMono.ttf")
(theme dark accent:#22d3ee panel:#070b12 card:#0e1520 muted:#8ea2b5 font:14 spacing:8 round:10)
(state playing bool false)
(state pos int 0)
(state volume int 80)
(state playlist list)
(state current string "")
(rust-field last_volume "i64" "80")

(rust-widget deck """
// --- управление mpv по IPC-сокету: std-only --------------------------------
const SOCK: &str = "/tmp/lazr-mpv.sock";

fn mpv_send(command: &str) -> Option<String> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    let mut stream = UnixStream::connect(SOCK).ok()?;
    let json = format!("{{\"command\":[{command}]}}\n");
    stream.write_all(json.as_bytes()).ok()?;
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    Some(line.trim().to_string())
}

fn mpv_position() -> f64 {
    let Some(answer) = mpv_send("\"get_property\", \"playback-time\"") else {
        return 0.0;
    };
    // ответ вида {"data":12.3,"request_id":0,"error":"success"}
    let Some(start) = answer.find("\"data\":") else {
        return 0.0;
    };
    let rest = &answer[start + 7..];
    let end = rest.find(',').unwrap_or(rest.len());
    rest[..end].trim().parse().unwrap_or(0.0)
}

fn mpv_duration() -> f64 {
    let Some(answer) = mpv_send("\"get_property\", \"duration\"") else {
        return 0.0;
    };
    let Some(start) = answer.find("\"data\":") else {
        return 0.0;
    };
    let rest = &answer[start + 7..];
    let end = rest.find(',').unwrap_or(rest.len());
    rest[..end].trim().parse().unwrap_or(0.0)
}

fn mpv_running() -> bool {
    mpv_send("\"get_property\", \"filename\"").is_some()
}

/// Дочерний mpv держим в слоте: иначе после quit он остаётся зомби (никто не сделал wait).
static MPV_CHILD: std::sync::Mutex<Option<std::process::Child>> = std::sync::Mutex::new(None);

fn mpv_quit() {
    let _ = mpv_send("\"quit\"");
    if let Ok(mut slot) = MPV_CHILD.lock() {
        if let Some(mut child) = slot.take() {
            let _ = child.wait();
        }
    }
}

fn mpv_set(name: &str, value: &str) {
    let _ = mpv_send(&format!("\"set_property\", \"{}\", {}", name, value));
}

/// Демо-трек: генерируем 48 секунд мягкого лупа своими руками (как в синтезаторе).
fn ensure_demo_track() -> String {
    let path = "/tmp/lazr-winamp-demo.wav";
    if std::path::Path::new(path).exists() {
        return path.to_string();
    }
    use std::f32::consts::TAU;
    let rate = 22050usize;
    let seconds = 48usize;
    let mut pcm = Vec::with_capacity(seconds * rate * 2);
    for t in 0..seconds * rate {
        let s = t as f32 / rate as f32;
        let beat = (s * 2.0).sin() * 0.5 + 0.5;
        let bass = (TAU * 110.0 * s).sin() * 0.35 * (1.0 - beat * 0.4);
        let pad = (TAU * 220.5 * s).sin() * 0.18 + (TAU * 277.2 * s).sin() * 0.14;
        let shimmer = (TAU * 554.4 * s).sin() * 0.06 * beat;
        let value = (bass + pad + shimmer) * 11000.0;
        pcm.extend_from_slice(&(value.clamp(-30000.0, 30000.0) as i16).to_le_bytes());
    }
    let mut wav = Vec::with_capacity(44 + pcm.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&((36 + pcm.len()) as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&(rate as u32).to_le_bytes());
    wav.extend_from_slice(&((rate as u32) * 2u32).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
    wav.extend_from_slice(&pcm);
    let _ = std::fs::write(path, wav);
    path.to_string()
}

fn mpv_play(path: &str) -> bool {
    mpv_quit();
    let _ = std::fs::remove_file(SOCK);
    let spawned = std::process::Command::new("mpv")
        .arg("--no-video")
        .arg("--log-file=/tmp/lazr-mpv.log2")
        .arg(format!("--input-ipc-server={SOCK}"))
        .arg("--idle=yes")
        .arg("--volume=80")
        .arg(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::fs::OpenOptions::new().create(true).append(true).open("/tmp/lazr-mpv.err").unwrap())
        .spawn();
    let started = spawned.is_ok();
    if let Ok(mut slot) = MPV_CHILD.lock() {
        *slot = spawned.ok();
    }
    if started {
        // сокет появляется за десятки миллисекунд
        std::thread::sleep(std::time::Duration::from_millis(400));
    }
    started
}

// --- оформление: экран-визуализатор (анимация, не спектр) -------------------
let width = ui.available_width();
let (screen_rect, _hit) = ui.allocate_exact_size(egui::vec2(width, 120.0), egui::Sense::hover());
ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
let painter = ui.painter_at(screen_rect);
painter.rect_filled(screen_rect, egui::CornerRadius::same(10), egui::Color32::from_rgb(4, 8, 14));
let bars = 44usize;
let step = screen_rect.width() / bars as f32;
let t = ui.input(|input| input.time) as f32;
for index in 0..bars {
    let phase = index as f32 * 0.37;
    let wave = 0.5 + 0.5 * (t * (2.0 + (index % 5) as f32 * 0.3) + phase).sin();
    let level = if state.playing { wave * 0.9 + 0.06 } else { 0.05 };
    let height = (level * screen_rect.height() * 0.92).max(2.0);
    let x = screen_rect.left() + index as f32 * step;
    let color = if level > 0.62 {
        egui::Color32::from_rgb(124, 156, 255)
    } else {
        egui::Color32::from_rgb(34, 211, 238)
    };
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(x + 1.5, screen_rect.bottom() - height - 4.0),
            egui::pos2(x + step - 1.5, screen_rect.bottom() - 4.0),
        ),
        egui::CornerRadius::same(2),
        color,
    );
}
painter.text(
    egui::pos2(screen_rect.left() + 12.0, screen_rect.top() + 16.0),
    egui::Align2::LEFT_CENTER,
    &state.current,
    egui::FontId::proportional(15.0),
    egui::Color32::from_rgb(230, 236, 246),
);

// --- громкость ползунка сразу уходит в mpv --------------------------------
if state.volume != state.last_volume {
    mpv_set("volume", &state.volume.to_string());
    state.last_volume = state.volume;
}

// --- полоса прогресса: pos / duration прямо из mpv -------------------------
if state.playing && mpv_running() {
    let duration = mpv_duration();
    if duration > 1.0 {
        let ratio = (state.pos as f64 / duration).clamp(0.0, 1.0) as f32;
        let track = egui::Rect::from_min_max(
            egui::pos2(screen_rect.left() + 12.0, screen_rect.bottom() - 12.0),
            egui::pos2(screen_rect.right() - 12.0, screen_rect.bottom() - 5.0),
        );
        painter.rect_filled(track, egui::CornerRadius::same(3), egui::Color32::from_rgb(30, 42, 58));
        if ratio > 0.0 {
            let done = egui::Rect::from_min_max(
                track.left_top(),
                egui::pos2(track.left() + track.width() * ratio, track.bottom()),
            );
            painter.rect_filled(done, egui::CornerRadius::same(3), egui::Color32::from_rgb(34, 211, 238));
        }
        // «0:04 / 0:48» справа над полосой: видно, где мы внутри трека
        let clock = |seconds: f64| format!("{}:{:02}", (seconds as i64) / 60, (seconds as i64) % 60);
        painter.text(
            egui::pos2(screen_rect.right() - 12.0, screen_rect.bottom() - 20.0),
            egui::Align2::RIGHT_CENTER,
            format!("{} / {}", clock(state.pos as f64), clock(duration)),
            egui::FontId::proportional(11.0),
            egui::Color32::from_rgb(154, 173, 191),
        );
    }
}

// --- позиция: спрашиваем mpv раз в полсекунды ------------------------------
if state.playing && mpv_running() {
    let position = mpv_position();
    if (position - state.pos as f64).abs() > 0.4 {
        state.pos = position as i64;
    }
}

// --- транспорт ------------------------------------------------------------
let (transport_rect, _transport) = ui.allocate_exact_size(egui::vec2(width, 44.0), egui::Sense::click());
let tp = ui.painter_at(transport_rect);
let keys: [(&str, &str); 4] = [
    ("\u{f04b}", "играть"),
    ("\u{f04c}", "пауза"),
    ("\u{f04d}", "стоп"),
    ("\u{f04a}", "-10 сек"),
];
let key_w = transport_rect.width() / 4.0;
for (index, (glyph, name)) in keys.iter().enumerate() {
    let rect = egui::Rect::from_min_max(
        egui::pos2(transport_rect.left() + key_w * index as f32 + 2.0, transport_rect.top() + 2.0),
        egui::pos2(transport_rect.left() + key_w * (index + 1) as f32 - 2.0, transport_rect.bottom() - 2.0),
    );
    let hit = ui.interact(rect, ui.scope_id().with(("key", index)), egui::Sense::click());
    let hover = hit.hovered();
    tp.rect_filled(rect, egui::CornerRadius::same(8), if hover {
        egui::Color32::from_rgb(22, 52, 66)
    } else {
        egui::Color32::from_rgb(14, 21, 32)
    });
    tp.rect_stroke(rect, egui::CornerRadius::same(8), egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 60, 76)), egui::StrokeKind::Inside);
    tp.text(
        egui::pos2(rect.center().x - 10.0, rect.center().y),
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::FontId::new(15.0, egui::FontFamily::Name("icons".into())),
        egui::Color32::from_rgb(34, 211, 238),
    );
    tp.text(
        egui::pos2(rect.center().x + 12.0, rect.center().y),
        egui::Align2::CENTER_CENTER,
        name,
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgb(154, 163, 184),
    );
    if !hit.clicked() {
        continue;
    }
    match *name {
        "играть" => {
            if mpv_running() {
                mpv_set("pause", "false");
                state.playing = true;
            } else {
                let demo = ensure_demo_track();
                // без mpv звука не будет — говорим об этом в самом окне, а не молчим
                if mpv_play(&demo) {
                    state.current = "демо-луп (свой WAV)".to_string();
                    state.playing = true;
                } else {
                    state.current = "mpv не найден — поставьте его, звук отдаётся внешнему плееру".to_string();
                    state.playing = false;
                }
            }
        }
        "пауза" => {
            mpv_set("pause", "true");
            state.playing = false;
        }
        "стоп" => {
            mpv_quit();
            state.playing = false;
            state.pos = 0;
        }
        "-10 сек" => {
            let target = (state.pos as f64 - 10.0).max(0.0);
            mpv_set("playback-time", &format!("{}", target));
            state.pos = target as i64;
        }
        _ => {}
    }
}
""")

(vbox
  (custom deck)
  (hbox
    (label "громкость" :size 11 :mono true :color "#8ea2b5")
    (slider :bind volume :range (0 100) :grow true)
    (label "{volume}" :size 12 :mono true :color "#22d3ee"))
  (hbox
    (label "файл" :size 11 :mono true :color "#8ea2b5")
    (button "открыть…" :size 12)
    (label "позиция {pos} сек · mpv IPC · std-only" :size 11 :mono true :grow true :color "#5b7080"))
  (separator)
  (hbox
    (label "плейлист" :size 11 :mono true :grow true :color "#8ea2b5")
    (button "" (append playlist "демо-луп (свой WAV)") :icon "\uf067" :size 11)
    (button "" (clear playlist) :icon "\uf1f8" :size 11))
  (list :bind playlist :empty "пусто — «+» добавит демо-луп"
    (hbox
      (icon "\uf001" :size 11 :color "#22d3ee")
      (label "{item}" :size 13 :grow true :color "#e6edf6")
      (button "" (remove playlist) :icon "\uf1f8" :size 11)))
  (label "управление — mpv по unix-сокету: пауза, seek, громкость. Ни одной зависимости." :size 11 :mono true :color "#5b7080"))
