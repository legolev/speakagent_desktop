import { create } from "zustand";

interface UiState {
  setupOpen: boolean; // мастер настройки открыт вручную
  dismissedFirstRun: boolean; // онбординг первого запуска закрыт в этой сессии
  droppedPaths: string[]; // файлы, брошенные в окно (drag&drop) — для очереди
  openSetup: () => void;
  closeSetup: () => void;
  setDropped: (paths: string[]) => void;
  clearDropped: () => void;
}

export const useUi = create<UiState>((set) => ({
  setupOpen: false,
  dismissedFirstRun: false,
  droppedPaths: [],
  openSetup: () => set({ setupOpen: true }),
  closeSetup: () => set({ setupOpen: false, dismissedFirstRun: true }),
  setDropped: (paths) => set({ droppedPaths: paths }),
  clearDropped: () => set({ droppedPaths: [] }),
}));
