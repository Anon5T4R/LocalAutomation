import { useSyncExternalStore } from "react";

/** i18n leve da UI (padrão da suíte, ver docs/planos/padrao-apps.md). */

export type Locale = "pt" | "en" | "es";

export const LOCALE_LABELS: Record<Locale, string> = {
  pt: "Português",
  en: "English",
  es: "Español",
};

const LOCALE_KEY = "localautomation.locale";

const pt = {
  // Toolbar
  "top.new": "Novo",
  "top.open": "Abrir…",
  "top.save": "Salvar",
  "top.saveAs": "Salvar como…",
  "top.run": "▶ Executar",
  "top.running": "Executando…",
  "top.settingsTitle": "Configurações",
  "top.untitled": "(sem título)",
  "top.modified": "●",

  // Paleta
  "palette.title": "Adicionar nó",
  "node.trigger": "Gatilho manual",
  "node.http": "Requisição HTTP",
  "node.command": "Rodar comando",
  "node.readfile": "Ler arquivo",
  "node.writefile": "Escrever arquivo",
  "node.transform": "Transformar (JS)",
  "node.condition": "Condição (if)",
  "node.delay": "Esperar",
  "node.notify": "Notificar",

  // Campos
  "field.payload": "Payload inicial (JSON, opcional)",
  "field.method": "Método",
  "field.url": "URL",
  "field.headers": "Cabeçalhos (um por linha, K: V)",
  "field.body": "Corpo",
  "field.command": "Comando (roda no shell do SO)",
  "field.path": "Caminho do arquivo",
  "field.content": "Conteúdo",
  "field.code": "Código JS — `input` é a entrada; a última expressão é a saída",
  "field.expr": "Expressão JS — verdadeiro segue por “true”",
  "field.ms": "Esperar (milissegundos)",
  "field.title": "Título",
  "field.message": "Mensagem",

  // Painel de config
  "config.empty": "Selecione um nó pra configurar.",
  "config.delete": "Excluir nó",
  "config.portTrue": "true",
  "config.portFalse": "false",

  // Logs
  "logs.title": "Execução",
  "logs.clear": "limpar",
  "logs.empty": "Nenhuma execução ainda — monte o fluxo e aperte Executar.",
  "logs.running": "executando…",
  "logs.done": "Fluxo concluído",
  "logs.failed": "Fluxo falhou: {error}",

  // Toasts / diálogos
  "toast.saved": "Salvo em {path}",
  "toast.saveFailed": "Falha ao salvar: {error}",
  "toast.openFailed": "Falha ao abrir: {error}",
  "toast.runFailed": "Falha ao executar: {error}",
  "toast.needTrigger": "O fluxo precisa de um nó Gatilho.",
  "dlg.saveTitle": "Salvar fluxo como",
  "dlg.openTitle": "Abrir fluxo",
  "confirm.discard": "Descartar as mudanças não salvas?",
  "dlg.ok": "OK",
  "dlg.cancel": "Cancelar",

  // Aviso de segurança (Decisão nº 8)
  "warn.imported":
    "⚠️ Fluxo aberto de arquivo: revise os nós de COMANDO e JS antes de executar — eles rodam com as suas permissões.",

  // Settings
  "settings.title": "Configurações",
  "settings.theme": "Tema",
  "settings.themeSystem": "Sistema",
  "settings.themeLight": "Claro",
  "settings.themeDark": "Escuro",
  "settings.language": "Idioma",
  "settings.about":
    " — automação de fluxos 100% local (n8n/Zapier offline): monte grafos de nós (HTTP, comando, arquivos, JS, condição), execute na hora e salve como .tflow. Sem sandbox de propósito: os nós rodam com as SUAS permissões — revise fluxos de terceiros. Parte da suíte Local.",
} as const;

export type MessageKey = keyof typeof pt;

const en: Record<MessageKey, string> = {
  "top.new": "New",
  "top.open": "Open…",
  "top.save": "Save",
  "top.saveAs": "Save as…",
  "top.run": "▶ Run",
  "top.running": "Running…",
  "top.settingsTitle": "Settings",
  "top.untitled": "(untitled)",
  "top.modified": "●",

  "palette.title": "Add node",
  "node.trigger": "Manual trigger",
  "node.http": "HTTP request",
  "node.command": "Run command",
  "node.readfile": "Read file",
  "node.writefile": "Write file",
  "node.transform": "Transform (JS)",
  "node.condition": "Condition (if)",
  "node.delay": "Wait",
  "node.notify": "Notify",

  "field.payload": "Initial payload (JSON, optional)",
  "field.method": "Method",
  "field.url": "URL",
  "field.headers": "Headers (one per line, K: V)",
  "field.body": "Body",
  "field.command": "Command (runs in the OS shell)",
  "field.path": "File path",
  "field.content": "Content",
  "field.code": "JS code — `input` is the input; the last expression is the output",
  "field.expr": "JS expression — truthy goes through “true”",
  "field.ms": "Wait (milliseconds)",
  "field.title": "Title",
  "field.message": "Message",

  "config.empty": "Select a node to configure it.",
  "config.delete": "Delete node",
  "config.portTrue": "true",
  "config.portFalse": "false",

  "logs.title": "Run",
  "logs.clear": "clear",
  "logs.empty": "No runs yet — build the flow and hit Run.",
  "logs.running": "running…",
  "logs.done": "Flow finished",
  "logs.failed": "Flow failed: {error}",

  "toast.saved": "Saved to {path}",
  "toast.saveFailed": "Failed to save: {error}",
  "toast.openFailed": "Failed to open: {error}",
  "toast.runFailed": "Failed to run: {error}",
  "toast.needTrigger": "The flow needs a Trigger node.",
  "dlg.saveTitle": "Save flow as",
  "dlg.openTitle": "Open flow",
  "confirm.discard": "Discard unsaved changes?",
  "dlg.ok": "OK",
  "dlg.cancel": "Cancel",

  "warn.imported":
    "⚠️ Flow opened from a file: review COMMAND and JS nodes before running — they run with your permissions.",

  "settings.title": "Settings",
  "settings.theme": "Theme",
  "settings.themeSystem": "System",
  "settings.themeLight": "Light",
  "settings.themeDark": "Dark",
  "settings.language": "Language",
  "settings.about":
    " — 100% local flow automation (offline n8n/Zapier): build node graphs (HTTP, command, files, JS, condition), run instantly and save as .tflow. Deliberately unsandboxed: nodes run with YOUR permissions — review third-party flows. Part of the Local suite.",
};

const es: Record<MessageKey, string> = {
  "top.new": "Nuevo",
  "top.open": "Abrir…",
  "top.save": "Guardar",
  "top.saveAs": "Guardar como…",
  "top.run": "▶ Ejecutar",
  "top.running": "Ejecutando…",
  "top.settingsTitle": "Configuración",
  "top.untitled": "(sin título)",
  "top.modified": "●",

  "palette.title": "Añadir nodo",
  "node.trigger": "Disparador manual",
  "node.http": "Petición HTTP",
  "node.command": "Ejecutar comando",
  "node.readfile": "Leer archivo",
  "node.writefile": "Escribir archivo",
  "node.transform": "Transformar (JS)",
  "node.condition": "Condición (if)",
  "node.delay": "Esperar",
  "node.notify": "Notificar",

  "field.payload": "Payload inicial (JSON, opcional)",
  "field.method": "Método",
  "field.url": "URL",
  "field.headers": "Cabeceras (una por línea, K: V)",
  "field.body": "Cuerpo",
  "field.command": "Comando (corre en el shell del SO)",
  "field.path": "Ruta del archivo",
  "field.content": "Contenido",
  "field.code": "Código JS — `input` es la entrada; la última expresión es la salida",
  "field.expr": "Expresión JS — verdadero sigue por “true”",
  "field.ms": "Esperar (milisegundos)",
  "field.title": "Título",
  "field.message": "Mensaje",

  "config.empty": "Selecciona un nodo para configurarlo.",
  "config.delete": "Eliminar nodo",
  "config.portTrue": "true",
  "config.portFalse": "false",

  "logs.title": "Ejecución",
  "logs.clear": "limpiar",
  "logs.empty": "Ninguna ejecución todavía — arma el flujo y pulsa Ejecutar.",
  "logs.running": "ejecutando…",
  "logs.done": "Flujo terminado",
  "logs.failed": "El flujo falló: {error}",

  "toast.saved": "Guardado en {path}",
  "toast.saveFailed": "Error al guardar: {error}",
  "toast.openFailed": "Error al abrir: {error}",
  "toast.runFailed": "Error al ejecutar: {error}",
  "toast.needTrigger": "El flujo necesita un nodo Disparador.",
  "dlg.saveTitle": "Guardar flujo como",
  "dlg.openTitle": "Abrir flujo",
  "confirm.discard": "¿Descartar los cambios sin guardar?",
  "dlg.ok": "OK",
  "dlg.cancel": "Cancelar",

  "warn.imported":
    "⚠️ Flujo abierto desde un archivo: revisa los nodos de COMANDO y JS antes de ejecutar — corren con tus permisos.",

  "settings.title": "Configuración",
  "settings.theme": "Tema",
  "settings.themeSystem": "Sistema",
  "settings.themeLight": "Claro",
  "settings.themeDark": "Oscuro",
  "settings.language": "Idioma",
  "settings.about":
    " — automatización de flujos 100% local (n8n/Zapier offline): arma grafos de nodos (HTTP, comando, archivos, JS, condición), ejecútalos al instante y guárdalos como .tflow. Sin sandbox a propósito: los nodos corren con TUS permisos — revisa flujos de terceros. Parte de la suite Local.",
};

const DICTS: Record<Locale, Record<MessageKey, string>> = { pt, en, es };

export function detectLocale(): Locale {
  const l = (typeof navigator !== "undefined" ? navigator.language : "pt").toLowerCase();
  if (l.startsWith("en")) return "en";
  if (l.startsWith("es")) return "es";
  return "pt";
}

function loadLocale(): Locale {
  const v = typeof localStorage !== "undefined" ? localStorage.getItem(LOCALE_KEY) : null;
  return v === "pt" || v === "en" || v === "es" ? v : detectLocale();
}

let current: Locale = loadLocale();
const listeners = new Set<() => void>();

export function getLocale(): Locale {
  return current;
}

export function setLocale(locale: Locale) {
  if (locale === current) return;
  current = locale;
  try {
    localStorage.setItem(LOCALE_KEY, locale);
  } catch {
    /* localStorage indisponível */
  }
  for (const l of listeners) l();
}

function subscribe(l: () => void) {
  listeners.add(l);
  return () => listeners.delete(l);
}

export function useLocale(): Locale {
  return useSyncExternalStore(subscribe, getLocale);
}

export function t(key: MessageKey, params?: Record<string, string | number>): string {
  let msg: string = DICTS[current][key] ?? pt[key] ?? key;
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      msg = msg.split(`{${k}}`).join(String(v));
    }
  }
  return msg;
}
