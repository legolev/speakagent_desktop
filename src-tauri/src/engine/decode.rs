//! Декод аудио/видео → f32 mono 16 кГц.
//! Быстрый путь — symphonia (pure-Rust): mp3/aac/mp4/flac/wav/ogg.
//! Fallback — bundled ffmpeg: webm/opus и всё остальное, что symphonia не берёт.

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

use sherpa_onnx::LinearResampler;
use symphonia::core::audio::{SampleBuffer, SignalSpec};
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
    let n_frames = track.codec_params.n_frames;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("decoder: {e}"))?;

    let limit: Option<usize> = max_secs.map(|s| (s * src_rate as f32) as usize);

    // Ресемплим потоково, по пакету: в памяти только итоговые 16 кГц, без копии всего
    // файла в исходной частоте (1 ч @ 44.1 кГц — это ещё ~635 МБ сверху).
    let mut rs = StreamResampler::new(src_rate, TARGET_SR)?;
    let mut out: Vec<f32> = Vec::new();
    if let Some(n) = n_frames {
        // заранее под всю длину: без удвоений Vec на сотнях мегабайт
        let n = limit.map(|l| (l as u64).min(n)).unwrap_or(n);
        out.reserve((n as f64 * TARGET_SR as f64 / src_rate as f64) as usize + 1024);
    }
    let mut mono: Vec<f32> = Vec::new();
    let mut consumed = 0usize;
    let mut sbuf: Option<(SampleBuffer<f32>, SignalSpec, u64)> = None;
    loop {
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
                let cap = decoded.capacity() as u64;
                // буфер переиспользуем; пересоздаём, только если пакет крупнее или сменился формат
                if !matches!(&sbuf, Some((_, sp, c)) if *sp == spec && *c >= cap) {
                    sbuf = Some((SampleBuffer::<f32>::new(cap, spec), spec, cap));
                }
                let (buf, _, _) = sbuf.as_mut().expect("sample buffer");
                buf.copy_interleaved_ref(decoded);
                let channels = spec.channels.count().max(1);
                mono.clear();
                mono.extend(
                    buf.samples()
                        .chunks(channels)
                        .map(|f| f.iter().sum::<f32>() / channels as f32),
                );
                if let Some(lim) = limit {
                    mono.truncate(lim.saturating_sub(consumed));
                }
                consumed += mono.len();
                rs.push(&mono, &mut out);
                if limit.is_some_and(|lim| consumed >= lim) {
                    break;
                }
            }
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(format!("decode: {e}")),
        }
    }
    rs.finish(&mut out);

    if out.is_empty() {
        return Err("empty stream".into());
    }
    Ok(out)
}

/// Потоковый ресемплер с антиалиасингом: windowed-sinc из sherpa (Kaldi LinearResample,
/// тот же, что sherpa применяет внутри себя). Наивная линейная интерполяция не режет
/// частоты выше 8 кГц — при 44.1/48 → 16 кГц они «заворачиваются» в речевую полосу
/// (замер, GigaAM v3 на 22 кГц записи: WER 3.49% → 3.09%).
struct StreamResampler(Option<LinearResampler>);

impl StreamResampler {
    fn new(from: u32, to: u32) -> Result<Self, String> {
        if from == to {
            return Ok(Self(None));
        }
        if from == 0 || to == 0 {
            return Err(format!("invalid sample rate {from} → {to} Hz"));
        }
        LinearResampler::create(from as i32, to as i32)
            .map(|r| Self(Some(r)))
            .ok_or_else(|| format!("failed to create resampler {from} → {to} Hz"))
    }

    fn push(&mut self, input: &[f32], out: &mut Vec<f32>) {
        match &self.0 {
            Some(r) if !input.is_empty() => out.extend(r.resample(input, false)),
            Some(_) => {}
            None => out.extend_from_slice(input),
        }
    }

    /// Хвост фильтра (последние семплы) — обязательно в конце потока.
    fn finish(&mut self, out: &mut Vec<f32>) {
        if let Some(r) = &self.0 {
            out.extend(r.resample(&[], true));
        }
    }
}

/// Ресемплинг целого буфера (захват микрофона диктовки: 44.1/48 кГц → 16 кГц).
pub fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let mut out = Vec::with_capacity((input.len() as f64 * to as f64 / from as f64) as usize + 64);
    match StreamResampler::new(from, to) {
        Ok(mut rs) => {
            // кусками по ~1 с: без гигантских временных буферов на стороне C API
            for chunk in input.chunks(from as usize) {
                rs.push(chunk, &mut out);
            }
            rs.finish(&mut out);
        }
        Err(_) => out.extend_from_slice(input),
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

    let mut child = cmd
        .stdin(Stdio::null()) // как было у output(): иначе ffmpeg читает stdin (клавиши)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to run ffmpeg ({ffmpeg}): {e}"))?;
    // stderr читаем параллельно: иначе на битом файле (ошибка на каждый кадр) ffmpeg
    // забьёт pipe и повиснет, а мы — вместе с ним.
    let mut err_pipe = child.stderr.take().ok_or("ffmpeg: no stderr")?;
    let err_reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = err_pipe.read_to_string(&mut s);
        s
    });

    // PCM читаем потоком сразу в f32: без второй копии всего файла в байтах.
    let mut stdout = child.stdout.take().ok_or("ffmpeg: no stdout")?;
    let mut samples: Vec<f32> = Vec::new();
    let mut buf = vec![0u8; 1 << 16];
    let mut carry = 0usize; // хвост незаконченного f32 в начале buf
    loop {
        let n = match stdout.read(&mut buf[carry..]) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(format!("ffmpeg read: {e}")),
        };
        let have = carry + n;
        let whole = have - have % 4;
        let (frames, _) = buf[..whole].as_chunks::<4>();
        samples.extend(frames.iter().map(|c| f32::from_le_bytes(*c)));
        buf.copy_within(whole..have, 0);
        carry = have - whole;
    }
    let status = child.wait().map_err(|e| format!("ffmpeg: {e}"))?;
    let stderr = err_reader.join().unwrap_or_default();
    if !status.success() {
        return Err(stderr.trim().to_string());
    }
    if samples.is_empty() {
        return Err("ffmpeg returned an empty stream".into());
    }
    Ok(samples)
}
