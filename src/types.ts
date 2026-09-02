export type Settings = { mode: 'document'|'text'|'table'|'formula'|'comic'; instructions: string; device: 'auto'|'cpu'|'vulkan'; maxTokens: number; useLayout: boolean };
export type Block = { kind: string; text: string; markdown: string; level?: number; rows?: string[][] };
export type Region = {id:string;order:number;label:string;bbox:[number,number,number,number];confidence:number;rawText:string;markdown:string;ocrMode:string;status:string;warning?:string};
export type Page = { id: string; name: string; sourcePage: number; width: number; height: number; status: string; rawText: string; markdown: string; blocks: Block[]; error?: string; warning?: string; elapsedMs: number; recognizedWith?: Settings; regions: Region[] };
export type DownloadProgress = {status:string;file:string;downloaded:number;total:number;bytesPerSecond:number;error?:string};
export type Snapshot = { project: { id: string; name: string; settings: Settings; pages: Page[] }; directory: string; busy: boolean; message: string; resourcesReady: boolean; download: DownloadProgress };
