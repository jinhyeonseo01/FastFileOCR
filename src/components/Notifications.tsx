import { Check, Download, Pause, Play, Square, X } from "lucide-react";
import { IconButton } from "./controls";
import type { WorkspaceController } from "../hooks/useWorkspace";
export function Notifications({ w }: { w: WorkspaceController }) {
  const { t } = w,
    d = w.data.download,
    active = [
      "checking",
      "downloading",
      "pausing",
      "paused",
      "extracting",
    ].includes(d.status),
    u = w.data.update,
    title = d.kind === "runtime" ? "runtimeDownloadTitle" : "downloadTitle";
  return (
    <>
      <div className="notification-stack">
        {!w.downloadHidden && !["idle", "ready"].includes(d.status) && (
          <section className="notification" role="region" aria-label={t(title)}>
            <div className="notification-heading">
              <Download size={18} />
              <strong>
                {t(title)} ·{" "}
                {t(d.status === "error" ? "downloadError" : d.status)}
              </strong>
              {!active && (
                <IconButton
                  title={t("close")}
                  onClick={() => w.setDownloadHidden(true)}
                >
                  <X size={15} />
                </IconButton>
              )}
            </div>
            <p className="filename">{d.file}</p>
            <progress
              aria-label={t(title)}
              value={d.downloaded}
              max={d.total || 1}
            />
            <div className="download-numbers">
              <span>
                {(d.downloaded / 1e6).toFixed(1)} / {(d.total / 1e6).toFixed(1)}{" "}
                MB
              </span>
              <strong>
                {Math.min(100, (d.downloaded / (d.total || 1)) * 100).toFixed(
                  1,
                )}
                %
              </strong>
            </div>
            {d.bytesPerSecond > 0 && (
              <small>{(d.bytesPerSecond / 1e6).toFixed(1)} MB/s</small>
            )}
            {d.error && <p className="error-text">{w.l(d.error)}</p>}
            <div className="button-row">
              {active ? (
                <>
                  <button
                    className="secondary-button"
                    disabled={d.status === "pausing"}
                    onClick={() =>
                      w.command(
                        d.status === "paused"
                          ? "resume_download"
                          : "pause_download",
                      )
                    }
                  >
                    {d.status === "paused" ? (
                      <Play size={13} />
                    ) : (
                      <Pause size={13} />
                    )}{" "}
                    {t(d.status === "paused" ? "resume" : "pause")}
                  </button>
                  <button
                    className="secondary-button"
                    onClick={() => w.command("cancel_scan")}
                  >
                    <Square size={13} />
                    {t("stop")}
                  </button>
                </>
              ) : (
                <button
                  className="primary-button"
                  disabled={w.working || !w.pages.length}
                  onClick={() =>
                    w.scan(
                      w.scanIds.length ? w.scanIds : w.pages.map((p) => p.id),
                    )
                  }
                >
                  <Play size={13} />
                  {t("resumeScan")}
                </button>
              )}
            </div>
            <p className="field-hint">{t("downloadFootnote")}</p>
          </section>
        )}
        {!w.updateHidden &&
          ["available", "downloading", "ready"].includes(u.status) && (
            <section className="notification" role="status">
              <div className="notification-heading">
                <Download size={18} />
                <strong>
                  {t(
                    u.status === "ready"
                      ? "updateReady"
                      : u.status === "downloading"
                        ? "updateDownloading"
                        : "updateAvailable",
                    { version: u.version },
                  )}
                </strong>
                <IconButton
                  title={t("close")}
                  onClick={() => w.setUpdateHidden(true)}
                >
                  <X size={15} />
                </IconButton>
              </div>
              {u.status === "downloading" ? (
                <progress max={u.total || 1} value={u.downloaded} />
              ) : (
                <button
                  className="primary-button"
                  disabled={w.working}
                  onClick={() =>
                    w.command(
                      u.status === "ready"
                        ? "install_update"
                        : "download_update",
                    )
                  }
                >
                  {t(u.status === "ready" ? "installUpdate" : "downloadUpdate")}
                </button>
              )}
            </section>
          )}
      </div>
      {w.toast && (
        <div role="status" className="toast">
          <Check size={16} />
          {w.toast}
        </div>
      )}
    </>
  );
}
