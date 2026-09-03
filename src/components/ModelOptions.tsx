import type { ModelDescriptor, Settings } from "../types";
import type { Translator } from "../i18n";
export function ModelOptions({
  model,
  settings,
  onChange,
  disabled,
  t,
}: {
  model: ModelDescriptor;
  settings: Settings;
  onChange: (settings: Settings) => void;
  disabled: boolean;
  t: Translator;
}) {
  const saved = settings.modelOptions?.[model.id] ?? {};
  return (
    <>
      {model.fields.map((field) => {
        const value =
          saved[field.key] ??
          (field.key === "maxTokens" ? settings.maxTokens : field.default);
        const change = (next: string | number | boolean) =>
          onChange({
            ...settings,
            modelOptions: {
              ...settings.modelOptions,
              [model.id]: { ...saved, [field.key]: next },
            },
          });
        if (field.kind === "boolean")
          return (
            <label key={field.key} className="check-field">
              <input
                type="checkbox"
                checked={Boolean(value)}
                disabled={disabled}
                onChange={(e) => change(e.target.checked)}
              />
              {t(field.labelKey)}
            </label>
          );
        return (
          <label className="field" key={field.key}>
            {t(field.labelKey)}
            {field.kind === "select" ? (
              <select
                value={String(value)}
                disabled={disabled}
                onChange={(e) =>
                  change(
                    typeof field.default === "number"
                      ? Number(e.target.value)
                      : e.target.value,
                  )
                }
              >
                {field.choices.map((option) => (
                  <option key={String(option)} value={String(option)}>
                    {field.unitKey
                      ? t(field.unitKey, { count: String(option) })
                      : String(option)}
                  </option>
                ))}
              </select>
            ) : field.kind === "number" ? (
              <input
                type="number"
                value={Number(value)}
                min={field.min ?? undefined}
                max={field.max ?? undefined}
                step={field.step ?? undefined}
                disabled={disabled}
                onChange={(e) => change(Number(e.target.value))}
              />
            ) : (
              <input
                type="text"
                value={String(value)}
                disabled={disabled}
                onChange={(e) => change(e.target.value)}
              />
            )}
          </label>
        );
      })}
    </>
  );
}
