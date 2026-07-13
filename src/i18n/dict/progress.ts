const ru = {
  decoding: "Готовлю аудио…",
  diarizing: "Определяю говорящих…",
  transcribing: "Распознаю речь…",
  punctuating: "Расставляю знаки препинания…",
  done: "Готово",
};
type T = typeof ru;
const en: T = {
  decoding: "Preparing audio…",
  diarizing: "Identifying speakers…",
  transcribing: "Transcribing…",
  punctuating: "Adding punctuation…",
  done: "Done",
};
export const progress = { ru, en };
