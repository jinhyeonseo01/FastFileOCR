import type { Language } from "./i18n";
export type Settings = {
  modelId: string;
  modelOptions: Record<string, Record<string, number | string | boolean>>;
  mode: string;
  instructions: string;
  device: string;
  maxTokens: number;
  useLayout: boolean;
};
export type ModelDescriptor = {
  id: string;
  name: string;
  descriptionKey: string;
  modes: string[];
  devices: string[];
  supportsLayout: boolean;
  fields: {
    key: string;
    labelKey: string;
    kind: string;
    choices: (number | string)[];
    default: number | string | boolean;
    unitKey?: string;
    min?: number;
    max?: number;
    step?: number;
  }[];
};
export type Preferences = {
  schemaVersion: number;
  language: Language;
  checkUpdates: boolean;
  readonly githubRepository: string;
  projectRoot?: string;
  scan: Settings;
};
export type Block = {
  kind: string;
  text: string;
  markdown: string;
  level?: number;
  rows?: string[][];
};
export type Region = {
  id: string;
  order: number;
  label: string;
  bbox: [number, number, number, number];
  confidence: number;
  rawText: string;
  markdown: string;
  ocrMode: string;
  status: string;
  warning?: string;
};
export type Page = {
  id: string;
  name: string;
  sourcePage: number;
  width: number;
  height: number;
  status: string;
  rawText: string;
  markdown: string;
  blocks: Block[];
  error?: string;
  warning?: string;
  elapsedMs: number;
  recognizedWith?: Settings;
  regions: Region[];
};
export type DownloadProgress = {
  kind: "model" | "runtime";
  status: string;
  file: string;
  downloaded: number;
  total: number;
  bytesPerSecond: number;
  error?: string;
};
export type UpdateProgress = {
  status: string;
  version: string;
  currentVersion: string;
  downloaded: number;
  total: number;
  error?: string;
};
export type Snapshot = {
  project: { id: string; name: string; settings: Settings; pages: Page[] };
  directory: string;
  busy: boolean;
  message: string;
  resourcesReady: boolean;
  download: DownloadProgress;
  preferences: Preferences;
  dataRoot: string;
  models: ModelDescriptor[];
  update: UpdateProgress;
};
