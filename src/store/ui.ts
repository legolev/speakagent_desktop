import { create } from "zustand";

interface UiState {
  setupOpen: boolean; // мастер настройки открыт вручную
  dismissedFirstRun: boolean; // онбординг первого запуска закрыт в этой сессии
  droppedPath: string | null; // файл, брошенный в окно (drag&drop)
  openSetup: () => void;
  closeSetup: () => void;
  setDropped: (path: string) => void;
  clearDropped: () => void;
}

export const useUi = create<UiState>((set) => ({
  setupOpen: false,
  dismissedFirstRun: false,
  droppedPath: null,
  openSetup: () => set({ setupOpen: true }),
  closeSetup: () => set({ setupOpen: false, dismissedFirstRun: true }),
  setDropped: (path) => set({ droppedPath: path }),
  clearDropped: () => set({ droppedPath: null }),
}));
