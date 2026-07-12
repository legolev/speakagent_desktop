import React from "react";
import ReactDOM from "react-dom/client";
import { HashRouter } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import { useJobs } from "./store/jobs";
import { ensureCore } from "./lib/api";
import "./index.css";

const queryClient = new QueryClient();

// При старте: подгрузить историю и молча докачать инфраструктурные модели (диаризация, ffmpeg).
void useJobs.getState().hydrate();
void ensureCore().catch(() => {});

// Убираем «браузерное» контекстное меню по правому клику (кроме полей ввода).
document.addEventListener("contextmenu", (e) => {
  const el = e.target as HTMLElement;
  if (!el.closest("input, textarea")) e.preventDefault();
});

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <HashRouter>
        <App />
      </HashRouter>
    </QueryClientProvider>
  </React.StrictMode>,
);
