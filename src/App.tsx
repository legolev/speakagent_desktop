import { Routes, Route, Navigate } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import Layout from "./components/Layout";
import Onboarding from "./components/Onboarding";
import HomePage from "./pages/HomePage";
import TranscribePage from "./pages/TranscribePage";
import HistoryPage from "./pages/HistoryPage";
import SettingsPage from "./pages/SettingsPage";
import { isReady } from "./lib/api";
import { useUi } from "./store/ui";

export default function App() {
  const { data: ready } = useQuery({ queryKey: ["ready"], queryFn: isReady });
  const setupOpen = useUi((s) => s.setupOpen);
  const dismissed = useUi((s) => s.dismissedFirstRun);
  const showOnboarding = setupOpen || (ready === false && !dismissed);

  return (
    <>
      {showOnboarding && <Onboarding />}
      <Routes>
        <Route element={<Layout />}>
          <Route path="/" element={<HomePage />} />
          <Route path="/transcribe" element={<TranscribePage />} />
          <Route path="/history" element={<HistoryPage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </>
  );
}
