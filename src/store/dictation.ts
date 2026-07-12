import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import {
  listDictations,
  deleteDictation,
  clearDictations,
  type StoredDictation,
} from "../lib/api";

interface DictationState {
  entries: StoredDictation[];
  recording: boolean;
  processing: boolean;
  hydrated: boolean;
  error: string | null;
  hydrate: () => Promise<void>;
  remove: (id: string) => void;
  clear: () => void;
  clearError: () => void;
}

export const useDictation = create<DictationState>((set, get) => ({
  entries: [],
  recording: false,
  processing: false,
  hydrated: false,
  error: null,
  async hydrate() {
    if (get().hydrated) return;
    try {
      const rows = await listDictations();
      set({ entries: rows, hydrated: true });
    } catch {
      set({ hydrated: true });
    }
  },
  remove(id) {
    set((s) => ({ entries: s.entries.filter((e) => e.id !== id) }));
    deleteDictation(id).catch(() => {});
  },
  clear() {
    set({ entries: [] });
    clearDictations().catch(() => {});
  },
  clearError() {
    set({ error: null });
  },
}));

// ── Живые события с бэкенда (глобальный хоткей срабатывает вне окна) ──
void listen<StoredDictation>("dictation:new", (e) => {
  useDictation.setState((s) => ({ entries: [e.payload, ...s.entries] }));
});
void listen<{ recording: boolean; processing: boolean }>("dictation:state", (e) => {
  useDictation.setState({
    recording: e.payload.recording,
    processing: e.payload.processing,
  });
});
void listen<string>("dictation:error", (e) => {
  useDictation.setState({ error: e.payload });
});
