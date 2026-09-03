import {
  ArrowUpRight,
  Check,
  FileText,
  FolderOpen,
  LoaderCircle,
  Plus,
  Search,
  Settings2,
  ShieldCheck,
} from "lucide-react";
import type { WorkspaceController } from "../hooks/useWorkspace";
import { IconButton } from "./controls";
export function Sidebar({ w }: { w: WorkspaceController }) {
  const { t } = w;
  return (
    <aside
      className="sidebar"
      tabIndex={-1}
      onKeyDown={w.selectionKeys}
      onMouseDown={(e) => {
        if (!(e.target as HTMLElement).closest("button,input,select,textarea"))
          e.currentTarget.focus();
      }}
    >
      <div className="brand">
        <img src="/icon.png" alt="" />
        <div>
          <strong>{t("appName")}</strong>
          <small>{t("slogan")}</small>
        </div>
      </div>
      <div className="workspace-card">
        <span className="eyebrow">{t("workspace")}</span>
        <div>
          <strong title={w.data.project.name}>
            {w.data.project.name || t("newDocument")}
          </strong>
          <IconButton
            title={t("openFolder")}
            disabled={!w.native}
            onClick={() => w.command("open_folder")}
          >
            <ArrowUpRight size={16} />
          </IconButton>
        </div>
        <small>
          {t("pageCount", { count: w.pages.length })} · {t("autosaved")}
        </small>
      </div>
      <div className="project-actions">
        <button disabled={w.working} onClick={w.newProject}>
          <Plus size={14} />
          {t("newProject")}
        </button>
        <button disabled={w.working} onClick={w.openProject}>
          <FolderOpen size={14} />
          {t("openProject")}
        </button>
      </div>
      <div className="section-label">
        <h2>{t("documents")}</h2>
        <span>{w.pages.length}</span>
      </div>
      <label className="search">
        <Search size={15} />
        <input
          aria-label={t("search")}
          placeholder={t("search")}
          value={w.query}
          onChange={(e) => w.setQuery(e.target.value)}
        />
      </label>
      {w.pages.length > 0 && (
        <>
          <div className="filter-row">
            {["all", "done", "error"].map((scope) => (
              <button
                key={scope}
                className={w.scope === scope ? "active" : ""}
                onClick={() => w.setScope(scope)}
              >
                {t(scope)}
                {scope !== "all" && (
                  <small>
                    {w.pages.filter((p) => p.status === scope).length}
                  </small>
                )}
              </button>
            ))}
          </div>
          <div className="selection-toolbar">
            <button onClick={w.selectAll}>{t("selectAll")}</button>
            <span>{t("selectedCount", { count: w.scanIds.length })}</span>
            <button
              onClick={() => w.setSelectedIds([])}
              disabled={!w.scanIds.length}
            >
              {t("clearSelection")}
            </button>
          </div>
        </>
      )}
      <div
        className="page-list"
        role="listbox"
        aria-label={t("documents")}
        aria-multiselectable="true"
        tabIndex={0}
      >
        {w.filtered.map((p) => (
          <button
            id={"page-" + p.id}
            key={p.id}
            role="option"
            aria-selected={w.selectedIds.includes(p.id)}
            className={
              "page-row " +
              (w.selectedIds.includes(p.id) ? "selected " : "") +
              (p.id === w.page?.id ? "current" : "")
            }
            onClick={(event) => w.selectPage(p, event)}
          >
            <span className="selection-check">
              {w.selectedIds.includes(p.id) && <Check size={12} />}
            </span>
            <span className={"page-icon " + p.status}>
              {p.status === "processing" ? (
                <LoaderCircle size={18} className="spin" />
              ) : (
                <FileText size={18} />
              )}
            </span>
            <span className="page-meta">
              <strong title={w.l(p.name)}>{w.l(p.name)}</strong>
              <small>
                {p.width} × {p.height} · {t(p.status)}
              </small>
            </span>
          </button>
        ))}
        {!w.pages.length ? (
          <div className="list-empty">
            <FileText size={28} />
            <p>{t("listEmpty")}</p>
          </div>
        ) : !w.filtered.length ? (
          <p className="muted">{t("noSearch")}</p>
        ) : null}
      </div>
      {w.pages.length > 0 && (
        <p className="selection-hint">{t("selectionHint")}</p>
      )}
      <button className="sidebar-add" disabled={w.working} onClick={w.addFiles}>
        <Plus size={17} />
        {t("addFiles")}
      </button>
      <div className="privacy-note">
        <ShieldCheck size={19} />
        <div>
          <strong>{t("privateTitle")}</strong>
          <p>{t("privateBody")}</p>
        </div>
      </div>
      <div className="sidebar-footer">
        <span className="status-dot" />
        <span>
          {t(
            w.data.download.status === "ready" ? "engineReady" : "modelsNeeded",
          )}
        </span>
        <IconButton title={t("settings")} onClick={() => w.setModal(true)}>
          <Settings2 size={17} />
        </IconButton>
      </div>
    </aside>
  );
}
