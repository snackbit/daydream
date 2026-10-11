import type { AiModelRegistry, AppSettings } from '@daydream/shared';

function changed<T extends object>(base: T, edited: T, saved: T): T {
  const next = { ...saved };
  for (const key in edited) {
    if (edited[key] !== base[key]) next[key] = edited[key];
  }
  return next;
}

function registryEdit(
  base: AiModelRegistry | undefined,
  edited: AiModelRegistry | undefined,
  saved: AiModelRegistry | undefined,
) {
  if (!base || !edited || !saved) return edited !== base ? edited : saved;
  return {
    ...saved,
    connections: [
      ...edited.connections
        .filter(
          (connection) =>
            !base.connections.some((item) => item.id === connection.id) ||
            saved.connections.some((item) => item.id === connection.id),
        )
        .map((connection) => {
          const before = base.connections.find(
            (item) => item.id === connection.id,
          );
          const persisted = saved.connections.find(
            (item) => item.id === connection.id,
          );
          return before && persisted
            ? changed(before, connection, persisted)
            : connection;
        }),
      ...saved.connections.filter(
        (connection) =>
          !base.connections.some((item) => item.id === connection.id) &&
          !edited.connections.some((item) => item.id === connection.id),
      ),
    ],
    models: [
      ...edited.models
        .filter(
          (model) =>
            !base.models.some((item) => item.id === model.id) ||
            saved.models.some((item) => item.id === model.id),
        )
        .map((model) => {
          const before = base.models.find((item) => item.id === model.id);
          const persisted = saved.models.find((item) => item.id === model.id);
          return before && persisted
            ? changed(before, model, persisted)
            : model;
        }),
      ...saved.models.filter(
        (model) =>
          !base.models.some((item) => item.id === model.id) &&
          !edited.models.some((item) => item.id === model.id),
      ),
    ],
    bindings: changed(base.bindings, edited.bindings, saved.bindings),
  };
}

/** Rebase edits onto native normalization, including newly encrypted credential refs. */
export function applySettingsEdit(
  base: AppSettings,
  edited: AppSettings,
  saved: AppSettings,
): AppSettings {
  const registry = registryEdit(
    base.ai.modelRegistry,
    edited.ai.modelRegistry,
    saved.ai.modelRegistry,
  );
  return {
    ...changed(base, edited, saved),
    record: changed(base.record, edited.record, saved.record),
    storage: changed(base.storage, edited.storage, saved.storage),
    ai: {
      ...changed(base.ai, edited.ai, saved.ai),
      ...(registry ? { modelRegistry: registry } : {}),
      transcription: changed(
        base.ai.transcription,
        edited.ai.transcription,
        saved.ai.transcription,
      ),
      vision: changed(base.ai.vision, edited.ai.vision, saved.ai.vision),
      ocr: changed(base.ai.ocr, edited.ai.ocr, saved.ai.ocr),
      processing: changed(
        base.ai.processing,
        edited.ai.processing,
        saved.ai.processing,
      ),
      activity: {
        ...saved.ai.activity,
        windows: changed(
          base.ai.activity.windows,
          edited.ai.activity.windows,
          saved.ai.activity.windows,
        ),
        keyboard: changed(
          base.ai.activity.keyboard,
          edited.ai.activity.keyboard,
          saved.ai.activity.keyboard,
        ),
        mouse: changed(
          base.ai.activity.mouse,
          edited.ai.activity.mouse,
          saved.ai.activity.mouse,
        ),
        frameDeltas: changed(
          base.ai.activity.frameDeltas,
          edited.ai.activity.frameDeltas,
          saved.ai.activity.frameDeltas,
        ),
      },
      recall: {
        ...changed(base.ai.recall, edited.ai.recall, saved.ai.recall),
        generation: changed(
          base.ai.recall.generation,
          edited.ai.recall.generation,
          saved.ai.recall.generation,
        ),
        embedding: changed(
          base.ai.recall.embedding,
          edited.ai.recall.embedding,
          saved.ai.recall.embedding,
        ),
      },
    },
  };
}
