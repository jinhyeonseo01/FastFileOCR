import { ModelOptions } from "./ModelOptions";
import { useEffect, useRef, useState } from "react";
import {
  Check,
  FolderOpen,
  Settings2,
  ScanLine,
  RefreshCw,
  X,
} from "lucide-react";
import type { WorkspaceController } from "../hooks/useWorkspace";
import { languageNames, type Language } from "../i18n";
import { IconButton } from "./controls";
const updateLabels: Record<string, string> = {
  current: "upToDate",
  unreleased: "updateNoRelease",
  available: "updateAvailable",
  ready: "updateReady",
  checking: "updateChecking",
  downloading: "updateDownloading",
};
export function SettingsDialog({ w }: { w: WorkspaceController }) {
  const { t } = w;
  const [section, setSection] = useState(w.settingsSection);
  const container = useRef<HTMLElement>(null);
  const model = w.data.models.find((m) => m.id === w.settings.modelId);
  useEffect(() => {
    const previous = document.activeElement as HTMLElement;
    container.current?.focus();
    return () => previous?.focus();
  }, []);
  const trap = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      w.setModal(false);
    }
    if (e.key === "Tab") {
      const items = Array.from(
        container.current?.querySelectorAll<HTMLElement>(
          "button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled),a[href]",
        ) ?? [],
      );
      const first = items[0],
        last = items.at(-1);
      if (
        e.shiftKey &&
        (document.activeElement === first ||
          document.activeElement === container.current)
      ) {
        e.preventDefault();
        last?.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first?.focus();
      }
    }
  };
  return (
    <div
      className="modal-backdrop"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) w.setModal(false);
      }}
    >
      <section
        className="settings-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
        tabIndex={-1}
        ref={container}
        onKeyDown={trap}
      >
        <div className="modal-header">
          <div>
            <span className="eyebrow">{t("appName")}</span>
            <h2 id="settings-title">{t("settings")}</h2>
          </div>
          <IconButton title={t("close")} onClick={() => w.setModal(false)}>
            <X size={20} />
          </IconButton>
        </div>
        <div className="settings-body">
          <nav aria-label={t("settings")}>
            {[
              ["general", Settings2],
              ["recognition", ScanLine],
              ["updates", RefreshCw],
            ].map(([id, Icon]) => {
              const C = Icon as typeof Settings2;
              return (
                <button
                  key={id as string}
                  className={section === id ? "active" : ""}
                  onClick={() => setSection(id as string)}
                >
                  <C size={17} />
                  {t(id as string)}
                </button>
              );
            })}
          </nav>
          <div className="settings-content">
            {section === "general" && (
              <>
                <h3>{t("general")}</h3>
                <label className="field">
                  {t("language")}
                  <select
                    value={w.preferences.language}
                    disabled={w.working}
                    onChange={(e) =>
                      w.changeLanguage(e.target.value as Language)
                    }
                  >
                    {Object.entries(languageNames).map(([code, name]) => (
                      <option key={code} value={code}>
                        {name}
                      </option>
                    ))}
                  </select>
                </label>
                <p className="field-hint">{t("languageHint")}</p>
                <label className="field">
                  {t("dataLocation")}
                  <div className="path-field">
                    <input value={w.data.dataRoot} readOnly />
                    <IconButton
                      title={t("openFolder")}
                      disabled={!w.native}
                      onClick={() => w.command("open_data_folder")}
                    >
                      <FolderOpen size={17} />
                    </IconButton>
                  </div>
                </label>
                <p className="field-hint">{t("dataHint")}</p>
              </>
            )}
            {section === "recognition" && (
              <>
                <h3>{t("recognition")}</h3>
                <label className="field">
                  {t("model")}
                  <select
                    value={w.settings.modelId}
                    disabled={w.working}
                    onChange={(e) => w.chooseModel(e.target.value)}
                  >
                    {w.data.models.map((m) => (
                      <option key={m.id} value={m.id}>
                        {m.name}
                      </option>
                    ))}
                  </select>
                </label>
                {model && (
                  <p className="field-hint">{t(model.descriptionKey)}</p>
                )}
                <label className="field">
                  {t("device")}
                  <select
                    value={w.settings.device}
                    disabled={w.working}
                    onChange={(e) =>
                      w.setSettings({ ...w.settings, device: e.target.value })
                    }
                  >
                    {(model?.devices ?? ["auto", "cpu", "vulkan", "cuda"]).map(
                      (device) => (
                        <option key={device} value={device}>
                          {t(device)}
                        </option>
                      ),
                    )}
                  </select>
                </label>
                <p className="field-hint">{t("cudaHint")}</p>
                <p className="field-hint">{t("runtimeDownloadHint")}</p>
                <h4>{t("modelOptions")}</h4>
                {model && (
                  <ModelOptions
                    model={model}
                    settings={w.settings}
                    onChange={w.setSettings}
                    disabled={w.working}
                    t={t}
                  />
                )}
                <label className="field">
                  {t("customInstructions")}
                  <textarea
                    rows={4}
                    maxLength={4000}
                    value={w.settings.instructions}
                    disabled={w.working}
                    placeholder={t("instructionsPlaceholder")}
                    onChange={(e) =>
                      w.setSettings({
                        ...w.settings,
                        instructions: e.target.value,
                      })
                    }
                  />
                </label>
                <p className="field-hint">{t("instructionsHint")}</p>
                <div className="info-card">{t("modelDownloadHint")}</div>
              </>
            )}
            {section === "updates" && (
              <>
                <h3>{t("updates")}</h3>
                <p className="field-hint">
                  {t("currentVersion", {
                    version: w.data.update.currentVersion,
                  })}
                </p>
                <label className="check-field">
                  <input
                    type="checkbox"
                    checked={w.preferences.checkUpdates}
                    onChange={(e) =>
                      w.setPreferences({
                        ...w.preferences,
                        checkUpdates: e.target.checked,
                      })
                    }
                  />
                  {t("autoUpdates")}
                </label>
                <label className="field">
                  {t("repository")}
                  <input
                    placeholder={t("repositoryPlaceholder")}
                    value={w.preferences.githubRepository}
                    onChange={(e) =>
                      w.setPreferences({
                        ...w.preferences,
                        githubRepository: e.target.value.trim(),
                      })
                    }
                  />
                </label>
                <p className="field-hint">{t("repositoryHint")}</p>
                <div className="update-card">
                  <p>
                    {t(updateLabels[w.data.update.status] ?? "updateIdle", {
                      version: w.data.update.version,
                    })}
                  </p>
                  {w.data.update.error && (
                    <p className="error-text">{w.l(w.data.update.error)}</p>
                  )}
                  {w.data.update.status === "downloading" && (
                    <progress
                      max={w.data.update.total || 1}
                      value={w.data.update.downloaded}
                    />
                  )}
                  <div className="button-row">
                    <button
                      className="secondary-button"
                      disabled={
                        w.working ||
                        ["checking", "downloading"].includes(
                          w.data.update.status,
                        ) ||
                        !w.preferences.githubRepository
                      }
                      onClick={w.checkUpdates}
                    >
                      {t("checkNow")}
                    </button>
                    {w.data.update.status === "available" && (
                      <button
                        className="primary-button"
                        onClick={() => w.command("download_update")}
                      >
                        {t("downloadUpdate")}
                      </button>
                    )}
                    {w.data.update.status === "ready" && (
                      <button
                        className="primary-button"
                        disabled={w.working}
                        onClick={() => w.command("install_update")}
                      >
                        {t("installUpdate")}
                      </button>
                    )}
                  </div>
                </div>
                <p className="field-hint">{t("updateHint")}</p>
              </>
            )}
          </div>
        </div>
        <div className="modal-footer">
          <button
            className="secondary-button"
            onClick={() => w.setModal(false)}
          >
            {t("close")}
          </button>
          <button
            className="primary-button"
            disabled={w.working}
            onClick={w.saveSettings}
          >
            <Check size={16} />
            {t("save")}
          </button>
        </div>
      </section>
    </div>
  );
}
