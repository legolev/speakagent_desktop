// Лёгкий самописный i18n-слой (RU/EN). Один источник истины — настройка `ui_lang`
// (SQLite на бэке); localStorage используется как мгновенное зеркало для первой
// отрисовки без «мигания». Язык влияет и на UI-строки, и на локаль дат/чисел, и —
// через ту же настройку на бэке — на язык вывода «Итогов встречи».
import { create } from "zustand";
import { setUiLanguage, uiLanguage, type UiLang } from "../lib/api";
import { common } from "./dict/common";
import { models } from "./dict/models";
import { nav } from "./dict/nav";
import { statusBar } from "./dict/statusBar";
import { progress } from "./dict/progress";
import { home } from "./dict/home";
import { transcribe } from "./dict/transcribe";
import { dictation } from "./dict/dictation";
import { about } from "./dict/about";
import { mcp } from "./dict/mcp";
import { settings } from "./dict/settings";
import { onboarding } from "./dict/onboarding";
import { meeting } from "./dict/meeting";
import { resultView } from "./dict/resultView";
import { beautify } from "./dict/beautify";
import { aiProvider } from "./dict/aiProvider";
import { modelSelector } from "./dict/modelSelector";
import { diagnostics } from "./dict/diagnostics";
import { overlay } from "./dict/overlay";
import { exportMenu } from "./dict/exportMenu";
import { dragDrop } from "./dict/dragDrop";
import { perf } from "./dict/perf";

// Словарь собирается из самодостаточных фрагментов (каждый несёт ru+en и сам
// проверяет их паритет). `ru` — источник структуры типа, `en` обязан ей соответствовать.
const ru = {
  common: common.ru,
  models: models.ru,
  nav: nav.ru,
  statusBar: statusBar.ru,
  progress: progress.ru,
  home: home.ru,
  transcribe: transcribe.ru,
  dictation: dictation.ru,
  about: about.ru,
  mcp: mcp.ru,
  settings: settings.ru,
  onboarding: onboarding.ru,
  meeting: meeting.ru,
  resultView: resultView.ru,
  beautify: beautify.ru,
  aiProvider: aiProvider.ru,
  modelSelector: modelSelector.ru,
  diagnostics: diagnostics.ru,
  overlay: overlay.ru,
  exportMenu: exportMenu.ru,
  dragDrop: dragDrop.ru,
  perf: perf.ru,
};

const en = {
  common: common.en,
  models: models.en,
  nav: nav.en,
  statusBar: statusBar.en,
  progress: progress.en,
  home: home.en,
  transcribe: transcribe.en,
  dictation: dictation.en,
  about: about.en,
  mcp: mcp.en,
  settings: settings.en,
  onboarding: onboarding.en,
  meeting: meeting.en,
  resultView: resultView.en,
  beautify: beautify.en,
  aiProvider: aiProvider.en,
  modelSelector: modelSelector.en,
  diagnostics: diagnostics.en,
  overlay: overlay.en,
  exportMenu: exportMenu.en,
  dragDrop: dragDrop.en,
  perf: perf.en,
};

export type Lang = UiLang; // "ru" | "en"
export type Dict = typeof ru;

const dictionaries: Record<Lang, Dict> = { ru, en };
const STORAGE_KEY = "ui_lang";

function detectInitial(): Lang {
  try {
    const s = localStorage.getItem(STORAGE_KEY);
    if (s === "ru" || s === "en") return s;
  } catch {
    /* ignore */
  }
  return "ru";
}

// Модульное зеркало текущего языка — для доступа из не-React утилит
// (format.ts, perf.ts, diarize.ts, api.ts, exporters.ts).
let _lang: Lang = detectInitial();

/** Текущий язык (императивно, вне React). */
export const getLang = (): Lang => _lang;

/** Словарь текущего языка (императивно, для не-React утилит). */
export const tr = (): Dict => dictionaries[_lang];

interface LangState {
  lang: Lang;
  setLang: (l: Lang) => void;
}

export const useLang = create<LangState>((set) => ({
  lang: _lang,
  setLang: (l) => {
    _lang = l;
    try {
      localStorage.setItem(STORAGE_KEY, l);
    } catch {
      /* ignore */
    }
    set({ lang: l });
    // Персист в бэкенд (это же меняет язык вывода LLM). Ошибку глушим — UI уже переключён.
    void setUiLanguage(l).catch(() => {});
  },
}));

/** Хук словаря текущего языка — компонент ре-рендерится при смене языка. */
export function useT(): Dict {
  const lang = useLang((s) => s.lang);
  return dictionaries[lang];
}

/**
 * Сверка с бэкендом на старте: `ui_lang` в SQLite — источник истины. Вызывать один
 * раз при монтировании App. localStorage уже дал корректную первую отрисовку; здесь
 * лишь примиряем расхождение (например, после чистой установки или сброса localStorage).
 */
export async function syncLangFromBackend(): Promise<void> {
  try {
    const backend = await uiLanguage();
    if ((backend === "ru" || backend === "en") && backend !== _lang) {
      _lang = backend;
      try {
        localStorage.setItem(STORAGE_KEY, backend);
      } catch {
        /* ignore */
      }
      useLang.setState({ lang: backend });
    }
  } catch {
    /* ignore */
  }
}
