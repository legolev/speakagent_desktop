//! Диаризация «кто говорит» через sherpa-onnx (pyannote-segmentation + эмбеддер CAM++).
//! Порог кластеризации задаётся снаружи (см. lib.rs). Если число спикеров известно —
//! передаём его как num_clusters (тогда порог не используется, счёт точный).
//!
//! Пост-обработка сегментов — порт проверенного алгоритма нашего облачного
//! диаризационного конвейера: collar → назначение слова по
//! МАКСИМАЛЬНОМУ перекрытию → сглаживание «островков» A-B-A → склейка соседних
//! реплик одного спикера → авто-один-спикер. Это чинит «десятки лишних людей» и
//! рваные реплики, которые даёт сырой выход кластеризации.

use sherpa_onnx::{
    FastClusteringConfig, OfflineSpeakerDiarization, OfflineSpeakerDiarizationConfig,
    OfflineSpeakerSegmentationModelConfig, OfflineSpeakerSegmentationPyannoteModelConfig,
    SpeakerEmbeddingExtractorConfig,
};

use crate::engine::asr::Word;

// ── Константы пост-обработки (дефолты из облачного конвейера диаризации) ──
const COLLAR_SEC: f32 = 0.2; // расширение границ сегмента с обеих сторон
const MERGE_GAP_SEC: f32 = 1.0; // склейка соседних реплик одного спикера, если пауза ≤
const MIN_TURN_SEC: f32 = 1.2; // «островок» короче — поглощается соседями
const SINGLE_RATIO: f32 = 0.90; // если лидер покрывает ≥ этой доли речи — все в одного

#[derive(Clone)]
pub struct Segment {
    pub start: f32,
    pub end: f32,
    pub speaker: i32,
}

/// Возвращает речевые сегменты с метками спикеров (0-based), отсортированные по времени.
/// `num_speakers = 0` → автоопределение по порогу.
pub fn diarize(
    seg_model: &str,
    emb_model: &str,
    samples: &[f32],
    num_threads: i32,
    cluster_threshold: f32,
    num_speakers: i32,
) -> Result<Vec<Segment>, String> {
    let mut config = OfflineSpeakerDiarizationConfig::default();
    config.segmentation = OfflineSpeakerSegmentationModelConfig {
        pyannote: OfflineSpeakerSegmentationPyannoteModelConfig {
            model: Some(seg_model.to_string()),
        },
        num_threads,
        ..Default::default()
    };
    config.embedding = SpeakerEmbeddingExtractorConfig {
        model: Some(emb_model.to_string()),
        num_threads,
        ..Default::default()
    };
    config.clustering = FastClusteringConfig {
        num_clusters: if num_speakers > 0 { num_speakers } else { -1 },
        threshold: cluster_threshold,
    };

    let diar = OfflineSpeakerDiarization::create(&config)
        .ok_or("failed to create the diarizer (check segmentation/embedding models)")?;
    let result = diar
        .process(samples)
        .ok_or("diarization produced no result")?;

    Ok(result
        .sort_by_start_time()
        .into_iter()
        .map(|s| Segment {
            start: s.start,
            end: s.end,
            speaker: s.speaker,
        })
        .collect())
}

fn fmt_ts(sec: f32) -> String {
    let s = sec.max(0.0) as u32;
    format!("{}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
}

/// Реплика: непрерывный кусок речи одного спикера.
struct Turn {
    speaker: i32,
    start: f32,
    end: f32,
    text: String,
}

/// Привязка слов к спикерам + пост-обработка → продуктовый формат
/// `SpeakerN [H:MM:SS]: текст` (по строке на реплику).
pub fn words_to_replicas(words: &[Word], segs: &[Segment]) -> String {
    if words.is_empty() {
        return String::new();
    }

    // 1) collar + сортировка сегментов диаризации
    let mut d: Vec<Segment> = segs
        .iter()
        .map(|s| Segment {
            start: (s.start - COLLAR_SEC).max(0.0),
            end: s.end + COLLAR_SEC,
            speaker: s.speaker,
        })
        .collect();
    d.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));

    // авто-один-спикер: если у лидера ≥ SINGLE_RATIO всего времени речи
    let force_single = force_single(&d);

    // 2) назначаем каждому слову спикера по МАКСИМАЛЬНОМУ перекрытию и сразу
    //    группируем подряд идущие слова одного спикера в реплики
    let mut turns: Vec<Turn> = Vec::new();
    for w in words {
        let spk = if force_single { 0 } else { assign_speaker(w, &d) };
        match turns.last_mut() {
            Some(last) if last.speaker == spk => {
                last.text.push(' ');
                last.text.push_str(&w.text);
                last.end = w.end.max(last.end);
            }
            _ => turns.push(Turn {
                speaker: spk,
                start: w.start,
                end: w.end.max(w.start),
                text: w.text.clone(),
            }),
        }
    }

    // 3) сглаживаем «островки» A-B-A короче MIN_TURN_SEC
    smooth_islands(&mut turns);
    // 4) склеиваем соседние реплики одного спикера в пределах MERGE_GAP_SEC
    let merged = merge_adjacent(turns);
    // 5) перенумеровываем спикеров по первому появлению и рендерим
    render(&merged)
}

fn force_single(d: &[Segment]) -> bool {
    let mut total = 0f32;
    let mut by: std::collections::HashMap<i32, f32> = std::collections::HashMap::new();
    for s in d {
        let dur = (s.end - s.start).max(0.0);
        total += dur;
        *by.entry(s.speaker).or_insert(0.0) += dur;
    }
    let leader = by.values().cloned().fold(0.0f32, f32::max);
    total > 0.0 && leader / total >= SINGLE_RATIO
}

/// Спикер слова: максимальное перекрытие [w.start, w.end] с сегментом; иначе ближайший.
fn assign_speaker(w: &Word, d: &[Segment]) -> i32 {
    let (ws, we) = (w.start, w.end.max(w.start));
    let mut best_spk = -1;
    let mut best_ov = 0f32;
    for s in d {
        let ov = (we.min(s.end) - ws.max(s.start)).max(0.0);
        if ov > best_ov {
            best_ov = ov;
            best_spk = s.speaker;
        }
    }
    if best_spk >= 0 {
        return best_spk;
    }
    // нет перекрытия — берём ближайший по времени сегмент
    let mid = (ws + we) * 0.5;
    let mut best = 0;
    let mut best_d = f32::INFINITY;
    for s in d {
        let dist = if mid < s.start {
            s.start - mid
        } else if mid > s.end {
            mid - s.end
        } else {
            0.0
        };
        if dist < best_d {
            best_d = dist;
            best = s.speaker;
        }
    }
    best
}

/// A-B-A: короткую реплику B между двумя репликами A перекрашиваем в A и сливаем.
fn smooth_islands(turns: &mut Vec<Turn>) {
    if turns.len() < 3 {
        return;
    }
    let mut i = 1;
    while i + 1 < turns.len() {
        let dur = turns[i].end - turns[i].start;
        let island = dur < MIN_TURN_SEC
            && turns[i - 1].speaker == turns[i + 1].speaker
            && turns[i - 1].speaker != turns[i].speaker;
        if island {
            let cur = turns.remove(i);
            turns[i - 1].end = cur.end;
            turns[i - 1].text.push(' ');
            turns[i - 1].text.push_str(&cur.text);
            // возможно, сразу склеим с правым соседом того же спикера
            if i < turns.len()
                && turns[i].speaker == turns[i - 1].speaker
                && (turns[i].start - turns[i - 1].end) <= MERGE_GAP_SEC
            {
                let nxt = turns.remove(i);
                turns[i - 1].end = nxt.end;
                turns[i - 1].text.push(' ');
                turns[i - 1].text.push_str(&nxt.text);
            }
            // не увеличиваем i — перепроверяем позицию
            continue;
        }
        i += 1;
    }
}

fn merge_adjacent(turns: Vec<Turn>) -> Vec<Turn> {
    let mut out: Vec<Turn> = Vec::new();
    for t in turns {
        match out.last_mut() {
            Some(last)
                if last.speaker == t.speaker && (t.start - last.end) <= MERGE_GAP_SEC =>
            {
                last.end = t.end;
                last.text.push(' ');
                last.text.push_str(&t.text);
            }
            _ => out.push(t),
        }
    }
    out
}

fn render(turns: &[Turn]) -> String {
    let mut map: std::collections::HashMap<i32, usize> = std::collections::HashMap::new();
    let mut next = 0usize;
    let mut lines: Vec<String> = Vec::new();
    for t in turns {
        let text = t.text.trim();
        if text.is_empty() {
            continue;
        }
        let idx = *map.entry(t.speaker).or_insert_with(|| {
            let v = next;
            next += 1;
            v
        });
        lines.push(format!("Speaker{} [{}]: {}", idx + 1, fmt_ts(t.start), text));
    }
    lines.join("\n")
}
