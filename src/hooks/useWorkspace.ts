import { version } from "../../package.json";
import { useCallback, useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { open, save } from "@tauri-apps/plugin-dialog";
import { localizeMessage, translator, type Language } from "../i18n";
import { selectIds } from "../domain/selection";
import type {
  Settings,
  Snapshot,
  Page,
  Preferences,
  DownloadProgress,
  UpdateProgress,
} from "../types";
const defaults: Settings = {
  modelId: "",
  modelOptions: {},
  mode: "document",
  instructions: "",
  device: "auto",
  maxTokens: 8192,
  useLayout: true,
};
const empty: Snapshot = {
  project: { id: "", name: "", settings: defaults, pages: [] },
  directory: "",
  busy: false,
  message: "",
  resourcesReady: false,
  download: {
    kind: "model",
    status: "idle",
    file: "",
    downloaded: 0,
    total: 1,
    bytesPerSecond: 0,
  },
  preferences: {
    schemaVersion: 1,
    language: "en",
    checkUpdates: true,
    githubRepository: "",
    scan: defaults,
  },
  dataRoot: "",
  models: [],
  update: {
    status: "idle",
    version: "",
    currentVersion: version,
    downloaded: 0,
    total: 0,
  },
};
export function useWorkspace() {
  const native = isTauri();
  const [data, setData] = useState<Snapshot>(empty),
    [selected, setSelected] = useState(""),
    [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [settings, setSettings] = useState<Settings>(defaults),
    [preferences, setPreferences] = useState<Preferences>(empty.preferences);
  const [drafts, setDrafts] = useState<Record<string, string>>({}),
    [image, setImage] = useState(""),
    [tab, setTab] = useState("preview");
  const [settingsSection, setSettingsSection] = useState("general");
  const [modal, setModal] = useState(false),
    [error, setError] = useState(""),
    [toast, setToast] = useState(""),
    [pending, setPending] = useState(false),
    [drag, setDrag] = useState(false);
  const [query, setQuery] = useState(""),
    [scope, setScope] = useState("all"),
    [format, setFormat] = useState("md"),
    [zoom, setZoom] = useState(100);
  const [showRegions, setShowRegions] = useState(true),
    [activeRegion, setActiveRegion] = useState(""),
    [downloadHidden, setDownloadHidden] = useState(false),
    [updateHidden, setUpdateHidden] = useState(false);
  const dataRef = useRef(data);
  dataRef.current = data;
  const draftsRef = useRef(drafts);
  draftsRef.current = drafts;
  const running = useRef(false),
    anchor = useRef(""),
    startupChecked = useRef(false);
  const importRef = useRef<(paths: string[]) => void>(() => {});
  const t = translator(preferences.language),
    l = (value?: string) => localizeMessage(value, t);
  const pages = data.project.pages,
    page = pages.find((p) => p.id === selected) ?? pages[0],
    text = page ? (drafts[page.id] ?? page.markdown) : "";
  const working = data.busy || pending,
    filtered = pages.filter(
      (p) =>
        p.name.toLowerCase().includes(query.toLowerCase()) &&
        (scope === "all" || p.status === scope),
    );
  const scanIds = pages
    .filter((p) => selectedIds.includes(p.id))
    .map((p) => p.id);
  const refresh = useCallback(async () => {
    if (!native) return;
    const next = await invoke<Snapshot>("snapshot");
    const old = dataRef.current;
    if (old.project.id !== next.project.id) {
      setSettings(next.project.settings);
      setDrafts({});
    }
    setData(next);
    setPreferences(next.preferences);
    setSelectedIds((current) =>
      old.project.id !== next.project.id || !old.project.pages.length
        ? next.project.pages.slice(0, 1).map((p) => p.id)
        : current.filter((id) => next.project.pages.some((p) => p.id === id)),
    );
    setSelected((current) =>
      next.project.pages.some((p) => p.id === current)
        ? current
        : (next.project.pages[0]?.id ?? ""),
    );
  }, [native]);
  const flush = useCallback(async () => {
    const entries = Object.entries(draftsRef.current);
    for (const [pageId, markdown] of entries)
      await invoke("edit_page", { pageId, markdown });
    setDrafts((current) => {
      const next = { ...current };
      for (const [id, value] of entries)
        if (next[id] === value) delete next[id];
      return next;
    });
  }, []);
  const action = useCallback(
    async (task: () => Promise<void>, shouldFlush = true) => {
      if (!native) {
        setError("@i18n(desktopRequired,)");
        return;
      }
      if (running.current) return;
      running.current = true;
      setPending(true);
      setError("");
      try {
        if (shouldFlush) await flush();
        await task();
        await refresh();
      } catch (e) {
        setError(String(e));
      } finally {
        running.current = false;
        setPending(false);
      }
    },
    [native, flush, refresh],
  );
  const addPaths = useCallback(
    (paths: string[]) =>
      void action(async () => {
        const errors = await invoke<string[]>("import_paths", { paths });
        if (errors.length) setError(errors.join("\n"));
      }),
    [action],
  );
  importRef.current = addPaths;
  const addFiles = () =>
    void action(async () => {
      const paths = await open({
        multiple: true,
        filters: [
          {
            name: t("fileFilter"),
            extensions: ["pdf", "png", "jpg", "jpeg", "webp", "bmp"],
          },
        ],
      });
      if (paths) {
        const errors = await invoke<string[]>("import_paths", {
          paths: Array.isArray(paths) ? paths : [paths],
        });
        if (errors.length) setError(errors.join("\n"));
      }
    });
  const paste = () =>
    void action(async () => {
      await invoke("paste_image");
    });
  const scan = (ids = scanIds) =>
    void action(async () => {
      await invoke("update_settings", { settings });
      await invoke("start_scan", { pageIds: ids });
    });
  const exportFile = () =>
    void action(async () => {
      const path = await save({
        defaultPath: data.project.name + "." + format,
        filters: [{ name: format.toUpperCase(), extensions: [format] }],
      });
      if (path) {
        await invoke("export_document", { path, format });
        setToast(t("exported"));
      }
    });
  const removePages = (ids = scanIds) => {
    if (working || !ids.length) return;
    void action(async () => {
      const count = await invoke<number>("remove_pages", { pageIds: ids });
      setToast(t("pagesRemoved", { count }));
    });
  };
  const selectPage = (
    p: Page,
    event: React.MouseEvent | React.KeyboardEvent,
  ) => {
    setSelectedIds((current) =>
      selectIds(
        current,
        p.id,
        filtered.map((item) => item.id),
        anchor.current,
        event.shiftKey,
        event.ctrlKey || event.metaKey,
      ),
    );
    if (!event.shiftKey || !anchor.current) anchor.current = p.id;
    setSelected(p.id);
    setZoom(100);
    if (Object.keys(draftsRef.current).length) void action(async () => {});
  };
  const selectAll = () => {
    setSelectedIds(filtered.map((p) => p.id));
    anchor.current = filtered[0]?.id ?? "";
  };
  const selectionKeys = (event: React.KeyboardEvent) => {
    if (
      (event.target as HTMLElement).closest(
        'input,textarea,[contenteditable="true"]',
      )
    )
      return;
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "a") {
      event.preventDefault();
      selectAll();
    }
    if (event.key === "Delete" && !working && scanIds.length) {
      event.preventDefault();
      removePages();
    }
    if (event.key === "Escape") {
      event.preventDefault();
      setSelectedIds([]);
    }
    if (["ArrowDown", "ArrowUp"].includes(event.key) && filtered.length) {
      event.preventDefault();
      const index = filtered.findIndex((p) => p.id === page?.id);
      const next =
        filtered[
          Math.max(
            0,
            Math.min(
              filtered.length - 1,
              index + (event.key === "ArrowDown" ? 1 : -1),
            ),
          )
        ];
      selectPage(next, event);
      document.getElementById("page-" + next.id)?.focus();
    }
  };
  const newProject = () =>
    void action(async () => {
      const parent = await open({
        directory: true,
        title: t("chooseParent"),
        defaultPath: data.dataRoot ? data.dataRoot + "/workspaces" : undefined,
      });
      if (typeof parent === "string")
        await invoke("create_project", { parent, name: t("newDocument") });
    });
  const openProject = () =>
    void action(async () => {
      const directory = await open({
        directory: true,
        title: t("chooseWorkspace"),
      });
      if (typeof directory === "string")
        await invoke("open_project", { directory });
    });
  const command = (name: string, args?: Record<string, unknown>) =>
    void action(async () => {
      await invoke(name, args);
    }, !["cancel_scan", "pause_download", "resume_download", "check_updates", "download_update"].includes(name));
  const openSettings = (section = "general") => {
    setSettingsSection(section);
    setModal(true);
  };
  const saveSettings = () =>
    void action(async () => {
      await invoke("update_settings", { settings });
      await invoke("update_preferences", { preferences });
      setModal(false);
      setToast(t("settingsSaved"));
    });
  const changeLanguage = (language: Language) => {
    const next = { ...preferences, language };
    setPreferences(next);
    if (native)
      void invoke("update_preferences", { preferences: next }).catch((e) =>
        setError(String(e)),
      );
  };
  const checkUpdates = () =>
    void action(async () => {
      await invoke("update_preferences", { preferences });
      await invoke("check_updates");
      setUpdateHidden(false);
    }, false);
  const chooseModel = (modelId: string) => {
    const model = data.models.find((m) => m.id === modelId);
    setSettings({
      ...settings,
      modelId,
      mode: model?.modes.includes(settings.mode)
        ? settings.mode
        : (model?.modes[0] ?? "text"),
      device: model?.devices.includes(settings.device)
        ? settings.device
        : (model?.devices[0] ?? "cpu"),
      useLayout: model?.supportsLayout ? settings.useLayout : false,
    });
  };
  useEffect(() => {
    if (!native) return;
    let active = true;
    void refresh().catch((e) => setError(String(e)));
    const listeners = [
      listen("workspace-changed", () => {
        if (active) void refresh().catch((e) => setError(String(e)));
      }),
      listen<DownloadProgress>("model-download", ({ payload }) => {
        if (active) {
          setData((current) => ({ ...current, download: payload }));
          if (
            ["checking", "downloading", "extracting"].includes(payload.status)
          )
            setDownloadHidden(false);
        }
      }),
      listen<UpdateProgress>("app-update", ({ payload }) => {
        if (active) setData((current) => ({ ...current, update: payload }));
      }),
      getCurrentWebviewWindow().onDragDropEvent(({ payload }) => {
        if (!active) return;
        if (payload.type === "enter") setDrag(true);
        if (payload.type === "leave") setDrag(false);
        if (payload.type === "drop") {
          setDrag(false);
          if (!dataRef.current.busy) importRef.current(payload.paths);
        }
      }),
    ];
    return () => {
      active = false;
      listeners.forEach((p) => void p.then((unlisten) => unlisten()));
    };
  }, [native, refresh]);
  useEffect(() => {
    if (!native || !data.project.id || startupChecked.current) return;
    startupChecked.current = true;
    if (data.preferences.checkUpdates && data.preferences.githubRepository)
      void invoke("check_updates").catch((e) => setError(String(e)));
  }, [native, data.project.id]);
  useEffect(() => {
    document.documentElement.lang = preferences.language;
  }, [preferences.language]);
  useEffect(() => {
    setImage("");
    setActiveRegion("");
    if (!page || !native) return;
    let active = true;
    invoke<string>("preview", { pageId: page.id })
      .then((url) => {
        if (active) setImage(url);
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
    };
  }, [page?.id, native]);
  useEffect(() => {
    if (!toast) return;
    const timeout = setTimeout(() => setToast(""), 3000);
    return () => clearTimeout(timeout);
  }, [toast]);
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (
        (event.ctrlKey || event.metaKey) &&
        event.key.toLowerCase() === "v" &&
        !(event.target as HTMLElement).closest(
          'input,textarea,[contenteditable="true"]',
        ) &&
        !working
      ) {
        event.preventDefault();
        paste();
      }
    };
    const pasted = (event: ClipboardEvent) => {
      if (
        !working &&
        Array.from(event.clipboardData?.items ?? []).some((item) =>
          item.type.startsWith("image/"),
        )
      ) {
        event.preventDefault();
        paste();
      }
    };
    window.addEventListener("keydown", handler);
    window.addEventListener("paste", pasted);
    return () => {
      window.removeEventListener("keydown", handler);
      window.removeEventListener("paste", pasted);
    };
  });
  return {
    native,
    data,
    settings,
    setSettings,
    preferences,
    setPreferences,
    t,
    l,
    pages,
    page,
    text,
    working,
    filtered,
    scanIds,
    selectedIds,
    setSelectedIds,
    selectPage,
    selectAll,
    removePages,
    selectionKeys,
    query,
    setQuery,
    scope,
    setScope,
    format,
    setFormat,
    modal,
    setModal,
    settingsSection,
    openSettings,
    error,
    setError,
    toast,
    setToast,
    drag,
    pending,
    image,
    tab,
    setTab,
    drafts,
    setDrafts,
    zoom,
    setZoom,
    showRegions,
    setShowRegions,
    activeRegion,
    setActiveRegion,
    downloadHidden,
    setDownloadHidden,
    updateHidden,
    setUpdateHidden,
    action,
    addFiles,
    paste,
    scan,
    exportFile,
    newProject,
    openProject,
    command,
    saveSettings,
    changeLanguage,
    checkUpdates,
    chooseModel,
  };
}
export type WorkspaceController = ReturnType<typeof useWorkspace>;
