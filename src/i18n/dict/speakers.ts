const ru = {
  mergeTitle: "Нажмите, чтобы объединить этого спикера с другим",
  mergeWith: "Объединить с…",
  reassignTitle: "Переназначить эту реплику другому спикеру",
  moveTo: "Переместить к…",
};

type T = typeof ru;

const en: T = {
  mergeTitle: "Click to merge this speaker into another",
  mergeWith: "Merge into…",
  reassignTitle: "Reassign this line to another speaker",
  moveTo: "Move to…",
};

export const speakers = { ru, en };
