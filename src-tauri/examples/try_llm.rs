//! Быстрый тест «Итогов встречи» (тот же код, что в приложении), без GUI.
//!   cargo run --example try_llm -- download                  # движок + активная LLM-модель
//!   cargo run --example try_llm -- models                    # каталог LLM + активная
//!   cargo run --example try_llm -- set <id>                  # выбрать активную LLM-модель
//!   cargo run --example try_llm -- gen <transcript.txt> [summary|business|interview|todo]

use std::sync::atomic::AtomicBool;
use std::time::Instant;

use speakagent_lib::engine::{llm, models};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("models");

    match cmd {
        "download" => {
            for id in models::missing_llm() {
                println!("скачиваю {id}…");
                models::download(&id, |done, total| {
                    if total > 0 {
                        print!("\r  {:.0}%   ", done as f64 / total as f64 * 100.0);
                        let _ = std::io::Write::flush(&mut std::io::stdout());
                    }
                })
                .expect("download");
                println!();
            }
            println!("✓ готово: {}", if llm::is_ready() { "итоги доступны" } else { "чего-то не хватает" });
        }
        "models" => {
            let active = models::active_llm_id();
            for m in models::list_llm() {
                println!(
                    "  {:<16} {:<34} {:>5} МБ  {} {}",
                    m.id,
                    m.name,
                    m.size_mb,
                    if m.installed { "✓" } else { "—" },
                    if m.id == active { "← активная" } else { "" }
                );
            }
            println!("движок llama-server: {}", if models::llama_server().is_some() { "✓" } else { "—" });
        }
        "set" => {
            let id = args.get(2).expect("set <id>");
            models::set_active_llm(id).expect("set_active_llm");
            println!("активная LLM: {id}");
        }
        "gen" => {
            let path = args.get(2).expect("gen <transcript.txt> [kind]");
            let kind = llm::ResultKind::parse(args.get(3).map(|s| s.as_str()).unwrap_or("summary"))
                .expect("kind: summary|business|interview|todo");
            let transcript = std::fs::read_to_string(path).expect("read transcript");
            println!(
                "модель {} · {} символов расшифровки",
                models::active_llm_id(),
                transcript.chars().count()
            );
            let cancel = AtomicBool::new(false);
            let t = Instant::now();
            let mut stage = "";
            let out = llm::generate(kind, &transcript, None, &cancel, |p| {
                if p.stage != stage || p.stage == "reading" {
                    stage = p.stage;
                    match p.stage {
                        "starting" => println!("запускаю сервер…"),
                        "reading" => println!("читаю запись: {}/{}", p.done, p.total),
                        "writing" => println!("пишу…"),
                        _ => {}
                    }
                }
            })
            .expect("generate");
            println!(
                "\n--- РЕЗУЛЬТАТ (за {:.1}с, дайджест: {}) ---\n{}",
                t.elapsed().as_secs_f32(),
                if out.digest.is_some() { "построен" } else { "не понадобился" },
                out.text
            );
            // повторный артефакт по кэшированному дайджесту — так делает приложение
            if let Some(d) = out.digest.as_deref() {
                let t = Instant::now();
                let second = llm::generate(llm::ResultKind::Todo, &transcript, Some(d), &cancel, |_| {})
                    .expect("generate todo from digest");
                println!(
                    "\n--- ЗАДАЧИ ПО ДАЙДЖЕСТУ (за {:.1}с) ---\n{}",
                    t.elapsed().as_secs_f32(),
                    second.text
                );
            }
            llm::shutdown();
        }
        other => eprintln!("неизвестная команда: {other}"),
    }
}
