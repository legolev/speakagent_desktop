//! Декод аудио/видео → f32 mono 16 кГц.
//! Быстрый путь — symphonia (pure-Rust): mp3/aac/mp4/flac/wav/ogg.
//! Fallback — bundled ffmpeg: webm/opus и всё остальное, что symphonia не берёт.

use std::fs::File;
use std::path::Path;
use std::process::Command;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub const TARGET_SR: u32 = 16000;

pub fn decode_to_16k_mono(path: &str) -> Result<Vec<f32>, String> {
    decode_to_16k_mono_max(path, None)
}

/// Контейнеры, которые обычно несут AAC. AAC-декодер symphonia ИСКАЖАЕТ звук (даёт
/// правдоподобные, но неверные сэмплы → «каша» в распознавании; ffmpeg декодирует верно),
/// поэтому для них сразу идём в ffmpeg. ffmpeg — обязательная инфра-модель (качается на старте).
fn prefers_ffmpeg(path: &str) -> bool {
    matches!(
        Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("mp4" | "m4a" | "m4v" | "aac" | "mov" | "3gp" | "3gpp")
    )
}

/// Декод с опциональным лимитом длины (сек).
/// AAC/mp4 — сначала ffmpeg (symphonia портит AAC), остальное — сначала symphonia.
/// Другой декодер всегда остаётся запасным.
pub fn decode_to_16k_mono_max(path: &str, max_secs: Option<f32>) -> Result<Vec<f32>, String> {
    if prefers_ffmpeg(path) {
        // AAC-контейнер: ffmpeg верно декодирует; symphonia — только если ffmpeg ещё не скачан.
        if let Some(ff) = ffmpeg_bin() {
            return match decode_with_ffmpeg(path, &ff, max_secs) {
                Ok(s) => Ok(s),
                Err(ff_err) => decode_symphonia(path, max_secs).map_err(|sym_err| {
                    format!("failed to decode (ffmpeg: {ff_err}; symphonia: {sym_err})")
                }),
            };
        }
        return decode_symphonia(path, max_secs);
    }
    // mp3/wav/flac/ogg — symphonia надёжен; ffmpeg в запас (webm/opus и прочее).
    match decode_symphonia(path, max_secs) {
        Ok(s) => Ok(s),
        Err(sym_err) => match ffmpeg_bin() {
            Some(ff) => decode_with_ffmpeg(path, &ff, max_secs).map_err(|ff_err| {
                format!("failed to decode (symphonia: {sym_err}; ffmpeg: {ff_err})")
            }),
            None => Err(sym_err),
        },
    }
}

// ─────────────────────────── symphonia (pure-Rust) ───────────────────────────
fn decode_symphonia(path: &str, max_secs: Option<f32>) -> Result<Vec<f32>, String> {
    let file = File::open(path).map_err(|e| format!("open: {e}"))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| format!("probe: {e}"))?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or("no audio track")?;
    let track_id = track.id;
    let src_rate = track.codec_params.sample_rate.ok_or("unknown sample rate")?;
    let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(1).max(1);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("decoder: {e}"))?;

    let limit: Option<usize> = max_secs.map(|s| (s * src_rate as f32) as usize);

    let mut mono: Vec<f32> = Vec::new();
    'outer: loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(_)) => break, // EOF
            Err(e) => return Err(format!("packet: {e}")),
        };
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                let mut sbuf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
                sbuf.copy_interleaved_ref(decoded);
                for frame in sbuf.samples().chunks(channels) {
                    let m: f32 = frame.iter().sum::<f32>() / channels as f32;
                    mono.push(m);
                }
                if let Some(lim) = limit {
                    if mono.len() >= lim {
                        break 'outer;
                    }
                }
            }
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(format!("decode: {e}")),
        }
    }

    if mono.is_empty() {
        return Err("empty stream".into());
    }
    Ok(resample_linear(&mono, src_rate, TARGET_SR))
}

/// Линейный ресемплинг (используется декодером файлов и захватом микрофона диктовки).
pub fn resample_linear(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = to as f64 / from as f64;
    let out_len = ((input.len() as f64) * ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 / ratio;
        let idx = src_pos.floor() as usize;
        let frac = (src_pos - idx as f64) as f32;
        let a = input.get(idx).copied().unwrap_or(0.0);
        let b = input.get(idx + 1).copied().unwrap_or(a);
        out.push(a + (b - a) * frac);
    }
    out
}

/// Быстрая длительность (сек) без полного декода: метаданные symphonia, иначе ffmpeg.
pub fn probe_duration(path: &str) -> Option<f32> {
    probe_symphonia(path).or_else(|| probe_ffmpeg(path))
}

fn probe_symphonia(path: &str) -> Option<f32> {
    let file = File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .ok()?;
    let track = probed
        .format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)?;
    let frames = track.codec_params.n_frames?;
    let rate = track.codec_params.sample_rate?;
    if rate == 0 {
        return None;
    }
    Some(frames as f32 / rate as f32)
}

fn probe_ffmpeg(path: &str) -> Option<f32> {
    let ff = ffmpeg_bin()?;
    let mut cmd = Command::new(ff);
    cmd.args(["-i", path]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().ok()?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    let idx = stderr.find("Duration:")?;
    let ts = stderr[idx + 9..].trim().split(',').next()?.trim().to_string();
    let p: Vec<&str> = ts.split(':').collect();
    if p.len() != 3 {
        return None;
    }
    let h: f32 = p[0].trim().parse().ok()?;
    let m: f32 = p[1].trim().parse().ok()?;
    let s: f32 = p[2].trim().parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

// ─────────────────────────── ffmpeg fallback ───────────────────────────
/// Ищем ffmpeg: менеджер моделей → env → рядом с exe (вверх по дереву) → PATH.
fn ffmpeg_bin() -> Option<String> {
    if let Some(p) = crate::engine::models::ffmpeg() {
        return Some(p);
    }
    if let Ok(p) = std::env::var("SPEAKAGENT_FFMPEG") {
        if Path::new(&p).exists() {
            return Some(p);
        }
    }
    let name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent();
        for _ in 0..6 {
            let Some(d) = dir else { break };
            for cand in [
                d.join(name),
                d.join("bin").join(name),
                d.join("resources").join("bin").join(name),
            ] {
                if cand.exists() {
                    return cand.to_str().map(|s| s.to_string());
                }
            }
            dir = d.parent();
        }
    }
    // macOS: приложения из Finder получают минимальный PATH — пробуем homebrew напрямую
    #[cfg(target_os = "macos")]
    for cand in ["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg"] {
        if Path::new(cand).exists() {
            return Some(cand.to_string());
        }
    }
    Some(name.to_string()) // последняя попытка — из PATH
}

fn decode_with_ffmpeg(path: &str, ffmpeg: &str, max_secs: Option<f32>) -> Result<Vec<f32>, String> {
    let mut cmd = Command::new(ffmpeg);
    cmd.args(["-v", "error", "-i", path, "-ac", "1", "-ar", "16000", "-f", "f32le"]);
    if let Some(s) = max_secs {
        cmd.args(["-t", &format!("{s}")]);
    }
    cmd.arg("pipe:1");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW — без мелькания консоли
    }

    let out = cmd
        .output()
        .map_err(|e| format!("failed to run ffmpeg ({ffmpeg}): {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }

    let bytes = out.stdout;
    let mut samples = Vec::with_capacity(bytes.len() / 4);
    for c in bytes.chunks_exact(4) {
        samples.push(f32::from_le_bytes([c[0], c[1], c[2], c[3]]));
    }
    if samples.is_empty() {
        return Err("ffmpeg returned an empty stream".into());
    }
    Ok(samples)
}
