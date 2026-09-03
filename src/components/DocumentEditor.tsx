import { invoke } from "@tauri-apps/api/core";
import {
  Check,
  CheckCheck,
  Copy,
  FileImage,
  LoaderCircle,
  Maximize2,
  Minus,
  RotateCcw,
  ScanLine,
  Trash2,
  ZoomIn,
} from "lucide-react";
import { lazy, Suspense } from "react";
const MarkdownResult = lazy(() => import("./MarkdownResult"));
import type { WorkspaceController } from "../hooks/useWorkspace";
import { IconButton } from "./controls";
export function DocumentEditor({ w }: { w: WorkspaceController }) {
  const { t, page } = w;
  return (
    <section className="editor-workspace">
      <div className="source-panel">
        <div className="panel-header">
          <div className="panel-title">
            <FileImage size={16} />
            <strong>{t("source")}</strong>
          </div>
          <div className="panel-tools">
            {!!page?.regions.length && (
              <button
                className="region-toggle"
                aria-pressed={w.showRegions}
                onClick={() => w.setShowRegions(!w.showRegions)}
              >
                {t(w.showRegions ? "hideRegions" : "showRegions")}
              </button>
            )}
            <IconButton
              title={t("zoomOut")}
              onClick={() => w.setZoom(Math.max(25, w.zoom - 25))}
            >
              <Minus size={15} />
            </IconButton>
            <span className="zoom-label">{w.zoom}%</span>
            <IconButton
              title={t("zoomIn")}
              onClick={() => w.setZoom(Math.min(300, w.zoom + 25))}
            >
              <ZoomIn size={15} />
            </IconButton>
            <IconButton title={t("fit")} onClick={() => w.setZoom(100)}>
              <Maximize2 size={15} />
            </IconButton>
          </div>
        </div>
        <div className="source-canvas">
          {w.image ? (
            <div
              className="page-image"
              style={{
                width: w.zoom + "%",
                maxWidth: w.zoom === 100 ? "100%" : "none",
              }}
            >
              <img src={w.image} alt={w.l(page?.name)} />
              {w.showRegions && !!page?.regions.length && (
                <svg
                  className="region-overlay"
                  viewBox={"0 0 " + page.width + " " + page.height}
                  aria-label={t("structure")}
                >
                  {page.regions.map((r) => (
                    <g
                      key={r.id}
                      className={w.activeRegion === r.id ? "active" : ""}
                      tabIndex={0}
                      role="button"
                      aria-label={r.order + ". " + t(r.label)}
                      onClick={() => {
                        w.setActiveRegion(r.id);
                        w.setTab("structure");
                      }}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          e.preventDefault();
                          w.setActiveRegion(r.id);
                          w.setTab("structure");
                        }
                      }}
                    >
                      <title>{r.order + ". " + t(r.label)}</title>
                      <rect
                        x={r.bbox[0]}
                        y={r.bbox[1]}
                        width={r.bbox[2] - r.bbox[0]}
                        height={r.bbox[3] - r.bbox[1]}
                      />
                      <text
                        x={r.bbox[0] + 4}
                        y={r.bbox[1] + 22}
                        fontSize={Math.max(18, page.width / 65)}
                      >
                        {r.order}
                      </text>
                    </g>
                  ))}
                </svg>
              )}
            </div>
          ) : (
            <LoaderCircle className="spin" size={28} />
          )}
        </div>
        <div className="panel-footer">
          <span title={w.l(page?.name)}>{w.l(page?.name)}</span>
          <span>
            {page?.width} × {page?.height}
          </span>
        </div>
      </div>
      <div className="result-panel">
        <div className="panel-header result-header">
          <div className="tabs" role="tablist" aria-label={t("result")}>
            {["preview", "edit", "structure", "raw"].map((tab) => (
              <button
                key={tab}
                role="tab"
                aria-selected={w.tab === tab}
                className={w.tab === tab ? "active" : ""}
                onClick={() => w.setTab(tab)}
              >
                {t(tab)}
                {tab === "edit" && page && w.drafts[page.id] !== undefined && (
                  <i />
                )}
              </button>
            ))}
          </div>
          <div className="panel-tools">
            <IconButton
              title={t("copy")}
              disabled={!w.text}
              onClick={() =>
                void w.action(async () => {
                  await invoke("copy_text", {
                    text: w.tab === "raw" ? page?.rawText : w.text,
                  });
                  w.setToast(t("copied"));
                })
              }
            >
              <Copy size={15} />
            </IconButton>
            <IconButton
              title={t("rescan")}
              disabled={w.working}
              onClick={() => page && w.scan([page.id])}
            >
              <RotateCcw size={15} />
            </IconButton>
            <IconButton
              title={t("remove")}
              disabled={w.working}
              onClick={() => page && w.removePages([page.id])}
            >
              <Trash2 size={15} />
            </IconButton>
          </div>
        </div>
        {page?.error && (
          <div className="page-alert error-text">{w.l(page.error)}</div>
        )}
        {page?.warning && <div className="page-alert">{w.l(page.warning)}</div>}
        <div className="result-content" role="tabpanel">
          {w.tab === "edit" ? (
            <textarea
              aria-label={t("edit")}
              className="result-editor"
              spellCheck={false}
              value={w.text}
              disabled={w.working}
              onChange={(e) =>
                page && w.setDrafts({ ...w.drafts, [page.id]: e.target.value })
              }
              placeholder={t("editHint")}
            />
          ) : w.tab === "raw" ? (
            <pre className="raw-output">{page?.rawText || t("rawHint")}</pre>
          ) : w.tab === "structure" ? (
            <div className="structure-view">
              <p className="structure-note">{t("structureHint")}</p>
              {page?.regions.map((r) => (
                <button
                  className={
                    "region-result " + (w.activeRegion === r.id ? "active" : "")
                  }
                  key={r.id}
                  onClick={() => w.setActiveRegion(r.id)}
                >
                  <strong>
                    {r.order}. {t(r.label)}{" "}
                    <small>{(r.confidence * 100).toFixed(0)}%</small>
                  </strong>
                  <span>
                    {r.bbox.map((n) => Math.round(n)).join(", ")} px ·{" "}
                    {t(r.status === "skipped" ? "skipped" : r.ocrMode)}
                  </span>
                  <p>{r.markdown || t("noText")}</p>
                  {r.warning && (
                    <em className="region-warning">{w.l(r.warning)}</em>
                  )}
                </button>
              ))}
              {page?.blocks.map((block, index) => (
                <div className="structure-block" key={index}>
                  <span>{String(index + 1).padStart(2, "0")}</span>
                  <div>
                    <strong>
                      {t(block.kind)}
                      {block.level ? " H" + block.level : ""}
                      {block.rows
                        ? " · " + t("rows", { count: block.rows.length })
                        : ""}
                    </strong>
                    <p>{block.text}</p>
                  </div>
                </div>
              ))}
              {!page?.blocks.length && (
                <p className="muted">{t("noStructure")}</p>
              )}
            </div>
          ) : w.text && page?.recognizedWith?.mode === "text" ? (
            <pre className="raw-output">{w.text}</pre>
          ) : w.text ? (
            <Suspense fallback={<LoaderCircle className="spin" size={20} />}>
              <MarkdownResult text={w.text} />
            </Suspense>
          ) : (
            <div className="result-empty">
              <div className="result-empty-icon">
                {page?.status === "processing" ? (
                  <LoaderCircle size={28} className="spin" />
                ) : (
                  <ScanLine size={28} />
                )}
              </div>
              <h3>
                {t(
                  page?.status === "processing"
                    ? "scanningTitle"
                    : "readyTitle",
                )}
              </h3>
              <p>
                {t(
                  page?.status === "processing" ? "scanningBody" : "readyBody",
                )}
              </p>
            </div>
          )}
        </div>
        <div className="panel-footer">
          <span>
            {t("characters", {
              count: w.text.length.toLocaleString(w.preferences.language),
            })}
            {page?.elapsedMs
              ? " · " +
                t("seconds", { count: (page.elapsedMs / 1000).toFixed(1) })
              : ""}
          </span>
          {page && w.drafts[page.id] !== undefined ? (
            <button
              className="text-button"
              disabled={w.working}
              onClick={() =>
                void w.action(async () => {
                  w.setToast(t("autosaved"));
                })
              }
            >
              <Check size={14} />
              {t("save")}
            </button>
          ) : (
            <span className="saved-label">
              <CheckCheck size={13} />
              {t("autosaved")}
            </span>
          )}
        </div>
      </div>
    </section>
  );
}
