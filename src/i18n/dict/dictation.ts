const ru = {
  title: "Диктовка",
  intro:
    "Зажмите горячую клавишу, продиктуйте — текст мгновенно распознаётся локально, копируется в буфер обмена и вставляется на курсор в любом приложении.",

  // Статусы.
  recording: "Идёт запись…",
  processing: "Распознаю…",
  ready: "Готов к диктовке",
  holdToRecord: "Зажмите для записи",

  // Настройки.
  settings: "Настройки",
  hotkeyLabel: "Горячая клавиша (push-to-talk) — можно одну клавишу, напр. правый Shift",
  capturing: "Нажмите клавишу…",
  record: "Записать",
  mode: "Режим",
  modeHold: "Зажать и говорить",
  modeToggle: "Нажать старт/стоп",
  autopaste: "Авто-вставка на курсор",
  sound: "Звук старт/стоп",
  mic: "Микрофон",
  defaultDevice: "Устройство по умолчанию",
  asrModel: "Модель распознавания",
  asActive: "Как активная (в «Настройках»)",
  macHelp:
    "На macOS дайте приложению два разрешения (System Settings → Privacy & Security) и перезапустите его: «Мониторинг ввода» (Input Monitoring) — чтобы срабатывала глобальная клавиша даже когда окно неактивно, и «Универсальный доступ» (Accessibility) — для авто-вставки. Без Accessibility текст всё равно копируется в буфер обмена — вставьте вручную (⌘V).",

  // Разрешения macOS.
  permsTitle: "Разрешения macOS",
  permsIntro:
    "Диктовке нужны системные разрешения. После выдачи иногда требуется перезапустить приложение.",
  permInputTitle: "Мониторинг ввода",
  permInputDesc: "чтобы горячая клавиша срабатывала, когда активно другое приложение",
  permAccessTitle: "Универсальный доступ",
  permAccessDesc: "для авто-вставки распознанного текста на курсор",
  permGranted: "выдано",
  permRequest: "Запросить",

  // История.
  historyTitle: "История диктовок",
  searchPlaceholder: "Поиск по тексту…",
  emptyHint: "Здесь появятся ваши быстрые распознавания.",
  clearConfirmTitle: "Очистить историю диктовок?",
  clearConfirmBody: "Все быстрые распознавания будут удалены безвозвратно.",

  // Человекочитаемые подписи клавиш(-ш). ↑↓←→ и Enter — символьные/латинские, одинаковы в обоих языках.
  keyLabels: {
    ShiftLeft: "Левый Shift",
    ShiftRight: "Правый Shift",
    ControlLeft: "Левый Ctrl",
    ControlRight: "Правый Ctrl",
    Alt: "Alt (лев.)",
    AltGr: "Alt (прав.)",
    MetaLeft: "Левый ⌘/Win",
    MetaRight: "Правый ⌘/Win",
    Space: "Пробел",
    Return: "Enter",
    UpArrow: "↑",
    DownArrow: "↓",
    LeftArrow: "←",
    RightArrow: "→",
  },
};

type T = typeof ru;
const en: T = {
  title: "Dictation",
  intro:
    "Hold the hotkey and dictate — the text is recognized locally in an instant, copied to the clipboard, and pasted at the cursor in any app.",

  recording: "Recording…",
  processing: "Recognizing…",
  ready: "Ready to dictate",
  holdToRecord: "Hold to record",

  settings: "Settings",
  hotkeyLabel: "Hotkey (push-to-talk) — a single key works too, e.g. right Shift",
  capturing: "Press a key…",
  record: "Record",
  mode: "Mode",
  modeHold: "Hold and speak",
  modeToggle: "Tap to start/stop",
  autopaste: "Auto-paste at cursor",
  sound: "Start/stop sound",
  mic: "Microphone",
  defaultDevice: "Default device",
  asrModel: "Recognition model",
  asActive: "Same as active (in Settings)",
  macHelp:
    "On macOS, grant the app two permissions (System Settings → Privacy & Security) and restart it: Input Monitoring — so the global hotkey fires even when the window is inactive, and Accessibility — for auto-paste. Without Accessibility the text is still copied to the clipboard — paste it manually (⌘V).",

  permsTitle: "macOS permissions",
  permsIntro:
    "Dictation needs system permissions. After granting them, a restart is sometimes required.",
  permInputTitle: "Input Monitoring",
  permInputDesc: "so the hotkey fires while another app is in the foreground",
  permAccessTitle: "Accessibility",
  permAccessDesc: "for auto-pasting the recognized text at the cursor",
  permGranted: "granted",
  permRequest: "Request",

  historyTitle: "Dictation history",
  searchPlaceholder: "Search text…",
  emptyHint: "Your quick recognitions will show up here.",
  clearConfirmTitle: "Clear dictation history?",
  clearConfirmBody: "All quick recognitions will be permanently deleted.",

  keyLabels: {
    ShiftLeft: "Left Shift",
    ShiftRight: "Right Shift",
    ControlLeft: "Left Ctrl",
    ControlRight: "Right Ctrl",
    Alt: "Alt (left)",
    AltGr: "Alt (right)",
    MetaLeft: "Left ⌘/Win",
    MetaRight: "Right ⌘/Win",
    Space: "Space",
    Return: "Enter",
    UpArrow: "↑",
    DownArrow: "↓",
    LeftArrow: "←",
    RightArrow: "→",
  },
};

export const dictation = { ru, en };
