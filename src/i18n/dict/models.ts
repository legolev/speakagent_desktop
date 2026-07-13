// Отображаемые имена/языки моделей по стабильному `id` из каталога Rust
// (engine/models.rs). UI берёт перевод отсюда с фолбэком на строку из Rust:
//   t.models[m.id]?.name ?? m.name   /   t.models[m.id]?.lang ?? m.lang
// Так каталог остаётся источником самих моделей, а витрина — двуязычная.
type ModelText = { name: string; lang: string };

const ru: Record<string, ModelText> = {
  gigaam: { name: "Русский (GigaAM)", lang: "русский" },
  parakeet: { name: "Мультиязычный (Parakeet)", lang: "25 языков" },
  "whisper-small": { name: "Whisper small", lang: "98 языков" },
  "whisper-turbo": { name: "Whisper large-v3 turbo", lang: "99 языков" },
  "llm-qwen3-4b": { name: "Стандартная — лучшее качество", lang: "русский+" },
  "llm-qwen3-17b": { name: "Быстрая — для слабых компьютеров", lang: "русский+" },
};

const en: Record<string, ModelText> = {
  gigaam: { name: "Russian (GigaAM)", lang: "Russian" },
  parakeet: { name: "Multilingual (Parakeet)", lang: "25 languages" },
  "whisper-small": { name: "Whisper small", lang: "98 languages" },
  "whisper-turbo": { name: "Whisper large-v3 turbo", lang: "99 languages" },
  "llm-qwen3-4b": { name: "Standard — best quality", lang: "Russian+" },
  "llm-qwen3-17b": { name: "Fast — for low-end computers", lang: "Russian+" },
};

export const models = { ru, en };
