//! Спайк оффлайн-пунктуации RUPunct через tract (чистый Rust, без onnxruntime).
//! Проверяет главный риск: держит ли tract int8-граф модели.
//!   cargo run --example try_punct -- "текст без пунктуации в нижнем регистре"

use std::collections::HashMap;

use speakagent_lib::engine::models;
use tokenizers::Tokenizer;
use tract_onnx::prelude::*;

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
        _ => "", // "O"
    }
}

/// Применяет метку RUPunct к слову: регистр + завершающий знак.
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
        1 => {
            let mut c = word.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        }
        _ => word.to_string(),
    };
    format!("{}{}", cased, mark(m))
}

fn main() -> TractResult<()> {
    let args: Vec<String> = std::env::args().collect();
    let text = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "шестьдесят тысяч тенге сколько будет стоить".to_string());

    let dir = models::models_dir().join("rupunct");
    let onnx = dir.join("rupunct_small_int8.onnx");
    let tokp = dir.join("tokenizer.json");
    let cfgp = dir.join("config.json");

    // id2label из config.json (авторитетный источник)
    let cfg: serde_json::Value = serde_json::from_slice(&std::fs::read(&cfgp)?)?;
    let id2label: HashMap<usize, String> = cfg["id2label"]
        .as_object()
        .expect("id2label")
        .iter()
        .map(|(k, v)| (k.parse::<usize>().unwrap(), v.as_str().unwrap().to_string()))
        .collect();

    let tok = Tokenizer::from_file(&tokp).expect("tokenizer");
    let enc = tok.encode(text.as_str(), true).expect("encode");
    let ids: Vec<i64> = enc.get_ids().iter().map(|&x| x as i64).collect();
    let mask: Vec<i64> = enc.get_attention_mask().iter().map(|&x| x as i64).collect();
    let types: Vec<i64> = vec![0i64; ids.len()];
    let seq = ids.len();
    println!("сабтокенов: {seq}");

    let t0 = std::time::Instant::now();
    let raw = tract_onnx::onnx().model_for_path(&onnx)?;
    let in_names: Vec<String> = raw
        .input_outlets()?
        .iter()
        .map(|o| raw.node(o.node).name.clone())
        .collect();
    println!("входы модели: {in_names:?}");
    let model = raw.into_optimized()?.into_runnable()?;
    println!("модель загружена за {:.2}с", t0.elapsed().as_secs_f32());

    let mut by_name: HashMap<&str, Tensor> = HashMap::new();
    by_name.insert("input_ids", Tensor::from_shape(&[1, seq], &ids)?);
    by_name.insert("attention_mask", Tensor::from_shape(&[1, seq], &mask)?);
    by_name.insert("token_type_ids", Tensor::from_shape(&[1, seq], &types)?);
    let inputs: TVec<TValue> = in_names
        .iter()
        .map(|n| {
            by_name
                .remove(n.as_str())
                .unwrap_or_else(|| panic!("модель ждёт вход '{n}' — не заполнен"))
                .into()
        })
        .collect();

    let t1 = std::time::Instant::now();
    let out = model.run(inputs)?;
    let logits = out[0].to_array_view::<f32>()?;
    println!("инференс за {:.1}мс, форма logits {:?}", t1.elapsed().as_secs_f32() * 1000.0, logits.shape());
    let nlab = logits.shape()[2];

    // первый сабтокен на слово → argmax-метка
    let word_ids = enc.get_word_ids();
    let mut word_label: HashMap<u32, usize> = HashMap::new();
    for (i, wid) in word_ids.iter().enumerate() {
        if let Some(w) = wid {
            word_label.entry(*w).or_insert_with(|| {
                let mut best = 0usize;
                let mut bv = f32::MIN;
                for k in 0..nlab {
                    let v = logits[[0, i, k]];
                    if v > bv {
                        bv = v;
                        best = k;
                    }
                }
                best
            });
        }
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    let restored: Vec<String> = words
        .iter()
        .enumerate()
        .map(|(wi, w)| {
            let lbl = word_label
                .get(&(wi as u32))
                .and_then(|k| id2label.get(k))
                .map(|s| s.as_str())
                .unwrap_or("LOWER_O");
            apply(w, lbl)
        })
        .collect();

    println!("\nВХОД:  {text}");
    println!("ВЫХОД: {}", restored.join(" "));
    Ok(())
}
