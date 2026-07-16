import { NODE_FIELDS, type FlowNode } from "../lib/flow";
import { t, type MessageKey } from "../lib/i18n";

interface Props {
  node: FlowNode | null;
  onChange: (id: string, key: string, value: string) => void;
  onDelete: (id: string) => void;
}

/** Painel direito: campos do nó selecionado (por tipo). */
export default function ConfigPanel({ node, onChange, onDelete }: Props) {
  if (!node) {
    return <aside className="config-panel muted empty">{t("config.empty")}</aside>;
  }
  const fields = NODE_FIELDS[node.data.kind];

  return (
    <aside className="config-panel">
      <h3>{t(`node.${node.data.kind}` as MessageKey)}</h3>
      {fields.map((f) => {
        const value = node.data.config[f.key] ?? "";
        const label = t(`field.${f.key}` as MessageKey);
        if (f.options) {
          return (
            <label key={f.key} className="field">
              <span>{label}</span>
              <select value={value || f.options[0]} onChange={(e) => onChange(node.id, f.key, e.target.value)}>
                {f.options.map((o) => (
                  <option key={o} value={o}>
                    {o}
                  </option>
                ))}
              </select>
            </label>
          );
        }
        if (f.multiline) {
          return (
            <label key={f.key} className="field">
              <span>{label}</span>
              <textarea
                className={f.mono ? "mono" : ""}
                rows={5}
                value={value}
                placeholder={f.placeholder}
                spellCheck={false}
                onChange={(e) => onChange(node.id, f.key, e.target.value)}
              />
            </label>
          );
        }
        return (
          <label key={f.key} className="field">
            <span>{label}</span>
            <input
              className={f.mono ? "mono" : ""}
              value={value}
              placeholder={f.placeholder}
              spellCheck={false}
              onChange={(e) => onChange(node.id, f.key, e.target.value)}
            />
          </label>
        );
      })}
      <button className="danger delete-btn" onClick={() => onDelete(node.id)}>
        {t("config.delete")}
      </button>
    </aside>
  );
}
