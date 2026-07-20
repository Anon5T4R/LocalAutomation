import { useEffect, useState } from "react";
import * as backend from "../lib/backend";
import { LOCALE_LABELS, setLocale, t, useLocale, type Locale } from "../lib/i18n";
import { useUi, type Theme } from "../state/ui";

/** Configurações: tema, idioma e segundo plano (padrão da suíte). */
export default function SettingsModal() {
  const open = useUi((s) => s.settingsOpen);
  const setOpen = useUi((s) => s.setSettingsOpen);
  const theme = useUi((s) => s.theme);
  const setTheme = useUi((s) => s.setTheme);
  const pushToast = useUi((s) => s.pushToast);
  const locale = useLocale();

  // Estas duas moram no backend, não no localStorage: a intenção de autostart é
  // lida no boot, antes de existir webview. Relemos ao abrir porque o reconcile
  // do boot pode ter desmarcado (Gerenciador de Tarefas do Windows).
  const [autostart, setAutostart] = useState(false);
  const [closeToTray, setCloseToTray] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open || !backend.isTauri) return;
    void backend.autostartGet().then(setAutostart).catch(() => {});
    void backend.closeToTrayGet().then(setCloseToTray).catch(() => {});
  }, [open]);

  if (!open) return null;

  // Otimista com rollback: se o registro recusar, a checkbox não pode ficar
  // mentindo que está ligada.
  const toggle = (
    set: (v: boolean) => void,
    save: (v: boolean) => Promise<void>,
    v: boolean,
  ) => {
    set(v);
    setBusy(true);
    save(v)
      .catch((e) => {
        set(!v);
        pushToast("error", t("toast.settingsFailed", { error: String(e) }));
      })
      .finally(() => setBusy(false));
  };

  const themes: { value: Theme; label: string }[] = [
    { value: "system", label: t("settings.themeSystem") },
    { value: "light", label: t("settings.themeLight") },
    { value: "dark", label: t("settings.themeDark") },
    { value: "nature", label: t("settings.themeNature") },
    { value: "darkblue", label: t("settings.themeDarkBlue") },
    { value: "calmgreen", label: t("settings.themeCalmGreen") },
    { value: "pastelpink", label: t("settings.themePastelPink") },
    { value: "punkprincess", label: t("settings.themePunkPrincess") },
  ];

  return (
    <div className="modal-backdrop" onClick={() => setOpen(false)}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h2>{t("settings.title")}</h2>

        <div className="settings-row">
          <span>{t("settings.theme")}</span>
          <div className="segmented">
            {themes.map((th) => (
              <button
                key={th.value}
                className={theme === th.value ? "active" : ""}
                onClick={() => setTheme(th.value)}
              >
                {th.label}
              </button>
            ))}
          </div>
        </div>

        <div className="settings-row">
          <span>{t("settings.language")}</span>
          <div className="segmented">
            {(Object.keys(LOCALE_LABELS) as Locale[]).map((l) => (
              <button key={l} className={locale === l ? "active" : ""} onClick={() => setLocale(l)}>
                {LOCALE_LABELS[l]}
              </button>
            ))}
          </div>
        </div>

        {backend.isTauri && (
          <>
            <h3 className="settings-section">{t("settings.background")}</h3>

            <div className="settings-row">
              <span>
                {t("settings.closeToTray")}
                <span className="muted small settings-hint">{t("settings.closeToTrayHint")}</span>
              </span>
              <input
                type="checkbox"
                checked={closeToTray}
                disabled={busy}
                onChange={(e) => toggle(setCloseToTray, backend.closeToTraySet, e.target.checked)}
              />
            </div>

            <div className="settings-row">
              <span>
                {t("settings.autostart")}
                <span className="muted small settings-hint">{t("settings.autostartHint")}</span>
              </span>
              <input
                type="checkbox"
                checked={autostart}
                disabled={busy}
                onChange={(e) => toggle(setAutostart, backend.autostartSet, e.target.checked)}
              />
            </div>
          </>
        )}

        <p className="muted about">
          <strong>LocalAutomation</strong>
          {t("settings.about")}
        </p>

        <div className="modal-actions">
          <button className="primary" onClick={() => setOpen(false)}>
            {t("dlg.ok")}
          </button>
        </div>
      </div>
    </div>
  );
}
