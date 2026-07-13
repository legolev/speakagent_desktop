import { useEffect } from "react";
import { Routes, Route, Navigate } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import Layout from "./components/Layout";
import Onboarding from "./components/Onboarding";
import HomePage from "./pages/HomePage";
import TranscribePage from "./pages/TranscribePage";
import DictationPage from "./pages/DictationPage";
import McpServerPage from "./pages/McpServerPage";
import SettingsPage from "./pages/SettingsPage";
import AboutPage from "./pages/AboutPage";
import { isReady } from "./lib/api";
import { useUi } from "./store/ui";
import { syncLangFromBackend } from "./i18n";

export default function App() {
  const { data: ready } = useQuery({ queryKey: ["ready"], queryFn: isReady });
  const setupOpen = useUi((s) => s.setupOpen);
  const dismissed = useUi((s) => s.dismissedFirstRun);
  const showOnboarding = setupOpen || (ready === false && !dismissed);

  // Язык интерфейса: источник истины — настройка на бэке; сверяем один раз при старте.
  useEffect(() => {
    void syncLangFromBackend();
  }, []);

  return (
    <>
      {showOnboarding && <Onboarding />}
      <Routes>
        <Route element={<Layout />}>
          <Route path="/" element={<HomePage />} />
          <Route path="/transcribe" element={<TranscribePage />} />
          {/* История объединена с «Расшифровкой» — старый путь ведёт туда же. */}
          <Route path="/history" element={<Navigate to="/transcribe" replace />} />
          <Route path="/dictation" element={<DictationPage />} />
          <Route path="/mcp" element={<McpServerPage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="/about" element={<AboutPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </>
  );
}
