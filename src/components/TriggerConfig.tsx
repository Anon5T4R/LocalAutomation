import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  TRIGGER_TYPES,
  baseName,
  isBackgroundTrigger,
  triggerWhen,
  type FlowNode,
  type TriggerWhen,
} from "../lib/flow";
import { t, type MessageKey } from "../lib/i18n";

interface Props {
  node: FlowNode;
  onChange: (id: string, key: string, value: string) => void;
}

/**
 * Painel do GATILHO em linguagem de gente (o coração da tarefa "triggers fáceis
 * pra leigos"). Em vez de um textarea de payload JSON, mostra CARDS com ícone +
 * frase + exemplo (descoberta: dá pra entender cada gatilho sem manual), e só o
 * campo simples do tipo escolhido. O jargão (payload) some pro modo Avançado.
 */
export default function TriggerConfig({ node, onChange }: Props) {
  const cfg = node.data.config;
  const when = triggerWhen(cfg);
  const [showAdvanced, setShowAdvanced] = useState(false);

  const set = (key: string, value: string) => onChange(node.id, key, value);

  const pickFolder = async () => {
    // Escolha VISUAL da pasta: diálogo nativo, modo diretório.
    const picked = await open({ directory: true, title: t("trig.folder.choose") });
    if (typeof picked === "string") set("folder", picked);
  };

  return (
    <div className="trigger-config">
      <p className="trig-prompt">{t("trig.pick")}</p>

      <div className="trig-cards">
        {TRIGGER_TYPES.map((tt) => {
          const selected = tt.when === when;
          return (
            <button
              key={tt.when}
              className={`trig-card ${selected ? "selected" : ""}`}
              onClick={() => set("when", tt.when)}
              type="button"
            >
              <span className="trig-card-icon">{tt.icon}</span>
              <span className="trig-card-text">
                <span className="trig-card-title">
                  {t(`trig.${tt.when}.title` as MessageKey)}
                </span>
                <span className="trig-card-example muted">
                  {t(`trig.${tt.when}.example` as MessageKey)}
                </span>
              </span>
            </button>
          );
        })}
      </div>

      <div className="trig-fields">{renderFields(when, cfg, set, pickFolder)}</div>

      {isBackgroundTrigger(when) && <p className="trig-bg-note muted">{t("trig.bgNote")}</p>}

      {/* Modo avançado: o payload JSON cru, escondido de quem não precisa. */}
      <button className="trig-advanced-toggle" onClick={() => setShowAdvanced((v) => !v)}>
        {showAdvanced ? "▾" : "▸"} {t("trig.advanced")}
      </button>
      {showAdvanced && (
        <label className="field">
          <span>{t("field.payload")}</span>
          <textarea
            className="mono"
            rows={4}
            value={cfg.payload ?? ""}
            placeholder='{ "exemplo": 1 }'
            spellCheck={false}
            onChange={(e) => set("payload", e.target.value)}
          />
        </label>
      )}
    </div>
  );
}

/** Campo simples do tipo escolhido — cada um em frase, nada de jargão. */
function renderFields(
  when: TriggerWhen,
  cfg: Record<string, string>,
  set: (key: string, value: string) => void,
  pickFolder: () => void,
) {
  if (when === "folder") {
    const folder = cfg.folder ?? "";
    return (
      <>
        <button className="trig-pick-folder" onClick={pickFolder} type="button">
          📂 {t("trig.folder.choose")}
        </button>
        <p className={`trig-folder-chosen ${folder ? "" : "muted"}`}>
          {folder
            ? t("trig.folder.chosen", { folder: baseName(folder) })
            : t("trig.folder.none")}
        </p>
        {folder && <p className="trig-folder-path muted">{folder}</p>}
        <label className="field">
          <span>{t("trig.folder.types")}</span>
          <input
            value={cfg.fileTypes ?? ""}
            placeholder="mp4, mkv, mp3"
            spellCheck={false}
            onChange={(e) => set("fileTypes", e.target.value)}
          />
          <span className="field-hint muted">{t("trig.folder.typesHint")}</span>
        </label>
      </>
    );
  }
  if (when === "interval") {
    return (
      <label className="field inline">
        <span>{t("trig.interval.every")}</span>
        <input
          type="number"
          min={1}
          className="trig-num"
          value={cfg.minutes ?? "5"}
          onChange={(e) => set("minutes", e.target.value)}
        />
        <span>{t("trig.interval.minutes")}</span>
      </label>
    );
  }
  if (when === "schedule") {
    return (
      <label className="field inline">
        <span>{t("trig.schedule.at")}</span>
        <input
          type="time"
          className="trig-time"
          value={cfg.time ?? "09:00"}
          onChange={(e) => set("time", e.target.value)}
        />
      </label>
    );
  }
  if (when === "startup") {
    return <p className="trig-note muted">{t("trig.startup.note")}</p>;
  }
  // manual
  return <p className="trig-note muted">{t("trig.manual.note")}</p>;
}
