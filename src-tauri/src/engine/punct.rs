//! Оффлайн-восстановление пунктуации и регистра для русского (RUPunct) через tract.
//! Чистый Rust: НЕ линкует onnxruntime (у sherpa свой, статический) → без второй DLL и cmake.
//! Модель — маленький BERT (token-classification, 33 метки: регистр × знак). Применяется к
//! ПЛОСКОМУ списку слов ASR; таймкоды/спикеры в модель не попадают.

use std::collections::HashMap;

use tokenizers::Tokenizer;
use tract_onnx::prelude::*;

type Runnable = RunnableModel<TypedFact, Box<dyn TypedOp>, TypedModel>;

const WIN: usize = 160; // слов в окне (с запасом под лимит позиций модели)
const MARGIN: usize = 24; // контекст по краям окна, который не эмитим (кроме краёв файла)
const LOWER_O: usize = 31; // метка «нижний регистр, без знака» — дефолт

pub struct Punctuator {
    model: Runnable,
    tok: Tokenizer,
    in_names: Vec<String>,
    id2label: Vec<String>, // индекс = id метки
}

impl Punctuator {
    /// Грузит модель + токенизатор + карту меток (config.json рядом с onnx).
    /// None → пунктуация недоступна (пайплайн просто пропускает шаг).
    pub fn load(onnx: &str, tokenizer: &str) -> Option<Self> {
        let tok = Tokenizer::from_file(tokenizer).ok()?;
        let cfg_path = std::path::Path::new(onnx).parent()?.join("config.json");
        let id2label = load_labels(&cfg_path)?;
        let raw = tract_onnx::onnx().model_for_path(onnx).ok()?;
        let in_names: Vec<String> = raw
            .input_outlets()
            .ok()?
            .iter()
            .map(|o| raw.node(o.node).name.clone())
            .collect();
        let model = raw.into_optimized().ok()?.into_runnable().ok()?;
        Some(Self {
            model,
            tok,
            in_names,
            id2label,
        })
    }

    /// Плоский список слов (нижний регистр, без знаков) → те же слова с регистром и знаками.
    /// Длина и порядок сохраняются 1:1 (можно zip-нуть обратно на слова с таймкодами).
    pub fn restore(&self, words: &[String]) -> Vec<String> {
        let n = words.len();
        if n == 0 {
            return Vec::new();
        }

        // скользящее окно: эмитим только «ядро» окна, чтобы у слов был контекст с обеих сторон
        let mut labels: Vec<usize> = vec![LOWER_O; n];
        let stride = WIN.saturating_sub(2 * MARGIN).max(1);
        let mut start = 0usize;
        loop {
            let end = (start + WIN).min(n);
            let win_labels = self.infer(&words[start..end]);
            let emit_lo = if start == 0 { 0 } else { MARGIN };
            let emit_hi = if end == n {
                end - start
            } else {
                (end - start).saturating_sub(MARGIN)
            };
            for j in emit_lo..emit_hi {
                labels[start + j] = win_labels.get(j).copied().unwrap_or(LOWER_O);
            }
            if end == n {
                break;
            }
            start += stride;
        }

        // применяем метки + подстраховка: заглавная в начале и после конечного знака
        let mut out: Vec<String> = Vec::with_capacity(n);
        let mut cap_next = true;
        for (i, w) in words.iter().enumerate() {
            let lbl = self
                .id2label
                .get(labels[i])
                .map(|s| s.as_str())
                .unwrap_or("LOWER_O");
            let mut s = apply(w, lbl);
            if cap_next {
                s = capitalize_first(&s);
            }
            cap_next = ends_sentence(&s);
            out.push(s);
        }
        out
    }

    /// Одно окно слов → метка для каждого слова (по первому сабтокену слова).
    fn infer(&self, words: &[String]) -> Vec<usize> {
        let fallback = || vec![LOWER_O; words.len()];
        let text = words.join(" ");
        let enc = match self.tok.encode(text.as_str(), true) {
            Ok(e) => e,
            Err(_) => return fallback(),
        };
        let ids: Vec<i64> = enc.get_ids().iter().map(|&x| x as i64).collect();
        let mask: Vec<i64> = enc.get_attention_mask().iter().map(|&x| x as i64).collect();
        let seq = ids.len();
        if seq == 0 {
            return fallback();
        }
        let types: Vec<i64> = vec![0i64; seq];

        let mut by_name: HashMap<&str, Tensor> = HashMap::new();
        by_name.insert("input_ids", tensor(&ids, seq));
        by_name.insert("attention_mask", tensor(&mask, seq));
        by_name.insert("token_type_ids", tensor(&types, seq));
        let inputs: Option<TVec<TValue>> = self
            .in_names
            .iter()
            .map(|nm| by_name.remove(nm.as_str()).map(|t| t.into()))
            .collect();
        let inputs = match inputs {
            Some(v) => v,
            None => return fallback(),
        };

        let out = match self.model.run(inputs) {
            Ok(o) => o,
            Err(_) => return fallback(),
        };
        let logits = match out[0].to_array_view::<f32>() {
            Ok(l) => l,
            Err(_) => return fallback(),
        };
        if logits.ndim() != 3 {
            return fallback();
        }
        let nlab = logits.shape()[2];

        let mut word_label = vec![LOWER_O; words.len()];
        let mut seen = vec![false; words.len()];
        for (i, wid) in enc.get_word_ids().iter().enumerate() {
            if let Some(w) = wid {
                let w = *w as usize;
                if w < words.len() && !seen[w] {
                    seen[w] = true;
                    let mut best = 0usize;
                    let mut bv = f32::MIN;
                    for k in 0..nlab {
                        let v = logits[[0, i, k]];
                        if v > bv {
                            bv = v;
                            best = k;
                        }
                    }
                    word_label[w] = best;
                }
            }
        }
        word_label
    }
}

fn tensor(data: &[i64], seq: usize) -> Tensor {
    Tensor::from_shape(&[1, seq], data).expect("tensor shape [1, seq]")
}

fn load_labels(cfg_path: &std::path::Path) -> Option<Vec<String>> {
    let cfg: serde_json::Value = serde_json::from_slice(&std::fs::read(cfg_path).ok()?).ok()?;
    let obj = cfg.get("id2label")?.as_object()?;
    let mut v = vec![String::new(); obj.len()];
    for (k, val) in obj {
        let idx: usize = k.parse().ok()?;
        if idx < v.len() {
            v[idx] = val.as_str()?.to_string();
        }
    }
    Some(v)
}

/// Знак препинания по хвосту метки RUPunct.
fn mark(m: &str) -> &'static str {
    match m {
        "PERIOD" => ".",
        "COMMA" => ",",
        "QUESTION" => "?",
        "TIRE" => " —",
        "VOSKL" => "!",
        "DVOETOCHIE" => ":",
        "PERIODCOMMA" => ";",
        "DEFIS" => "-",
        "QUESTIONVOSKL" => "?!",
        "MNOGOTOCHIE" => "…",
        _ => "", // "O" — без знака
    }
}

/// Применяет метку RUPunct к слову: регистр (UPPER/UPPER_TOTAL/LOWER) + завершающий знак.
fn apply(word: &str, label: &str) -> String {
    let (case, m) = if let Some(r) = label.strip_prefix("UPPER_TOTAL_") {
        (2u8, r)
    } else if let Some(r) = label.strip_prefix("UPPER_") {
        (1, r)
    } else if let Some(r) = label.strip_prefix("LOWER_") {
        (0, r)
    } else {
        (0, label)
    };
    let cased = match case {
        2 => word.to_uppercase(),
        1 => capitalize_first(word),
        _ => word.to_string(),
    };
    format!("{}{}", cased, mark(m))
}

fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn ends_sentence(s: &str) -> bool {
    matches!(s.chars().last(), Some('.' | '?' | '!' | '…'))
}
