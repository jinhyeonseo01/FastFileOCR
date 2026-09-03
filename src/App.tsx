import {
  ArrowDownToLine,
  ClipboardPaste,
  FilePlus2,
  LayoutList,
  LoaderCircle,
  Play,
  ScanLine,
  Settings2,
  Square,
  X,
} from "lucide-react";
import { useWorkspace } from "./hooks/useWorkspace";
import { Sidebar } from "./components/Sidebar";
import { DocumentEditor } from "./components/DocumentEditor";
import { SettingsDialog } from "./components/SettingsDialog";
import { Notifications } from "./components/Notifications";
import { IconButton } from "./components/controls";
export default function App() {
  const w = useWorkspace(),
    { t } = w,
    model = w.data.models.find((m) => m.id === w.settings.modelId);
  return (
    <div className="app-shell">
      <Sidebar w={w} />
      <main className="main">
        <header className="topbar">
          <div className="breadcrumb">
            {t("workspace")}
            <span>/</span>
            <strong>{t("documents")}</strong>
          </div>
          <div className="top-actions">
            <span className="local-pill">
              <span />
              {t("local")}
            </span>
            <IconButton title={t("settings")} onClick={() => w.setModal(true)}>
              <Settings2 size={18} />
            </IconButton>
            <div className="export-group">
              <select
                aria-label={t("exportFormat")}
                value={w.format}
                onChange={(e) => w.setFormat(e.target.value)}
              >
                {["md", "txt", "json", "html"].map((format) => (
                  <option key={format} value={format}>
                    {format === "md" ? "Markdown" : format.toUpperCase()}
                  </option>
                ))}
              </select>
              <button
                disabled={w.working || !w.pages.some((p) => p.markdown)}
                onClick={w.exportFile}
              >
                <ArrowDownToLine size={16} />
                {t("export")}
              </button>
            </div>
          </div>
        </header>
        <section className="heading">
          <div>
            <h1>{t("title")}</h1>
            <p>{t("subtitle")}</p>
          </div>
          <span className="heading-badge">
            <ScanLine size={21} />
            {t("slogan")}
          </span>
        </section>
        {!w.native && (
          <div className="browser-notice">{t("browserPreview")}</div>
        )}
        {w.error && (
          <div role="alert" className="error-banner">
            <span>{w.l(w.error)}</span>
            <IconButton title={t("close")} onClick={() => w.setError("")}>
              <X size={16} />
            </IconButton>
          </div>
        )}
        <section className="scan-controls">
          <div className="command-bar">
            <div className="command-left">
              <label className="mode-control">
                <span>{t("mode")}</span>
                <select
                  aria-label={t("mode")}
                  disabled={w.working}
                  value={w.settings.mode}
                  onChange={(e) =>
                    w.setSettings({ ...w.settings, mode: e.target.value })
                  }
                >
                  {(
                    model?.modes ?? [
                      "document",
                      "text",
                      "table",
                      "formula",
                      "comic",
                    ]
                  ).map((mode) => (
                    <option key={mode} value={mode}>
                      {t(mode)}
                    </option>
                  ))}
                </select>
              </label>
              <button
                className={
                  "instruction-button " +
                  (w.settings.instructions ? "has-instructions" : "")
                }
                onClick={() => w.openSettings("recognition")}
              >
                <Settings2 size={15} />
                {t("instructions")}
              </button>
            </div>
            <div className="command-right">
              <span className="queue-count">
                {w.pages.filter((p) => p.status === "done").length} /{" "}
                {w.pages.length}
              </span>
              {w.data.busy ? (
                <button
                  className="stop-button"
                  onClick={() => w.command("cancel_scan")}
                >
                  <Square size={14} />
                  {t("stop")}
                </button>
              ) : (
                <>
                  <button
                    className="secondary-button"
                    disabled={w.working || !w.pages.length}
                    onClick={() => w.scan(w.pages.map((p) => p.id))}
                  >
                    {t("scanAll")}
                  </button>
                  <button
                    className="primary-button"
                    disabled={w.working || !w.scanIds.length}
                    onClick={() => w.scan()}
                  >
                    <Play size={15} />
                    {t(w.pending ? "working" : "scanSelected")}
                    {w.scanIds.length > 0 && (
                      <span className="button-count">{w.scanIds.length}</span>
                    )}
                  </button>
                </>
              )}
            </div>
          </div>
          <div className="layout-options">
            <label>
              <input
                type="checkbox"
                checked={w.settings.useLayout}
                disabled={w.working || model?.supportsLayout === false}
                onChange={(e) =>
                  w.setSettings({ ...w.settings, useLayout: e.target.checked })
                }
              />
              {t("layout")}
            </label>
            <span>
              {t(w.settings.useLayout ? "layoutHint" : "fullPageHint")}
            </span>
          </div>
          {w.settings.mode === "comic" && (
            <p className="mode-hint">{t("comicHint")}</p>
          )}
        </section>
        {w.pages.length ? (
          <DocumentEditor w={w} />
        ) : (
          <section className="empty-workspace">
            <div className="drop-zone">
              <img className="empty-icon" src="/icon.png" alt="" />
              <h2>{t("dropTitle")}</h2>
              <p>{t("dropBody")}</p>
              <div className="empty-buttons">
                <button
                  className="primary-button"
                  disabled={w.working}
                  onClick={w.addFiles}
                >
                  <FilePlus2 size={17} />
                  {t("addFiles")}
                </button>
                <button
                  className="secondary-button"
                  disabled={w.working}
                  onClick={w.paste}
                >
                  <ClipboardPaste size={17} />
                  {t("paste")}
                  <kbd>Ctrl V</kbd>
                </button>
              </div>
              <div className="file-types">
                {["PDF", "PNG", "JPG", "WEBP", "BMP"].map((type) => (
                  <span key={type}>{type}</span>
                ))}
              </div>
            </div>
            <div className="feature-grid">
              {[
                [ScanLine, "featureScan", "featureScanBody"],
                [LayoutList, "featureStructure", "featureStructureBody"],
                [ArrowDownToLine, "featureExport", "featureExportBody"],
              ].map(([Icon, title, body]) => {
                const C = Icon as typeof ScanLine;
                return (
                  <div key={title as string}>
                    <C size={20} />
                    <h3>{t(title as string)}</h3>
                    <p>{t(body as string)}</p>
                  </div>
                );
              })}
            </div>
          </section>
        )}
        <footer className="statusbar">
          <div>
            {w.working ? (
              <LoaderCircle size={13} className="spin" />
            ) : (
              <span className="status-dot" />
            )}
            <span>
              {w.pending
                ? t("working")
                : w.l(w.data.message) || t("initialMessage")}
            </span>
          </div>
          <span>
            {model?.name} · {t(w.settings.device)}
          </span>
        </footer>
      </main>
      <Notifications w={w} />
      {w.drag && (
        <div className="drag-overlay">
          <ScanLine size={44} />
          <h2>{t("dropTitle")}</h2>
        </div>
      )}
      {w.modal && <SettingsDialog w={w} />}
    </div>
  );
}
