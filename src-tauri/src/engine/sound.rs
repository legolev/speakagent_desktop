//! Короткие тихие звуковые сигналы старт/стоп для диктовки (rodio, pure-Rust).
//! WAV вшиты в бинарь (`include_bytes!`), проигрываются в отдельном потоке, чтобы
//! не блокировать вызывающего. Ошибки воспроизведения молча игнорируются — звук
//! необязателен и не должен ронять флоу диктовки.

use std::io::Cursor;

use rodio::{Decoder, OutputStream, Sink};

static START_WAV: &[u8] = include_bytes!("sounds/start.wav");
static STOP_WAV: &[u8] = include_bytes!("sounds/stop.wav");

fn play(bytes: &'static [u8]) {
    std::thread::spawn(move || {
        // OutputStream нужно держать живым до конца проигрывания.
        let Ok((_stream, handle)) = OutputStream::try_default() else {
            return;
        };
        let Ok(sink) = Sink::try_new(&handle) else {
            return;
        };
        if let Ok(src) = Decoder::new(Cursor::new(bytes)) {
            sink.append(src);
            sink.sleep_until_end();
        }
    });
}

/// Сигнал начала записи.
pub fn play_start_cue() {
    play(START_WAV);
}

/// Сигнал окончания записи.
pub fn play_stop_cue() {
    play(STOP_WAV);
}
