import en from "../locate/en.json";
import ko from "../locate/ko.json";
import ja from "../locate/ja.json";
export type Language = "en" | "ko" | "ja";
export type Translator = (
  key: string,
  values?: Record<string, string | number>,
) => string;
const catalogs: Record<Language, Record<string, string>> = { en, ko, ja };
export const languageNames = { en: "English", ko: "한국어", ja: "日本語" };
export function translator(language: Language): Translator {
  return (key, values = {}) =>
    (catalogs[language]?.[key] ?? en[key as keyof typeof en] ?? key).replace(
      /\{(\w+)\}/g,
      (match, k) => String(values[k] ?? match),
    );
}
export function localizeMessage(
  value: string | undefined,
  t: Translator,
): string {
  if (!value) return "";
  return value.replace(
    /@i18n\(([\w.]+),([\w-]*)\)/g,
    (_match, key, encoded) => {
      let args: string[] = [];
      try {
        if (encoded)
          args = JSON.parse(
            new TextDecoder().decode(
              Uint8Array.from(
                atob(encoded.replace(/-/g, "+").replace(/_/g, "/")),
                (c) => c.charCodeAt(0),
              ),
            ),
          );
      } catch {}
      return localizeMessage(
        t(key, Object.fromEntries(args.map((v, i) => [i, v]))),
        t,
      );
    },
  );
}
