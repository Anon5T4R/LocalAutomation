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
  "top.auto": "Auto:",
  "top.autoOff": "desligado",
  "top.autoHint": "Executa o fluxo automaticamente a cada intervalo (enquanto o app estiver aberto)",
  "history.title": "Histórico",
  "history.subtitle": "Execuções recentes (últimas 40)",
  "history.empty": "Nenhuma execução ainda.",
  "history.steps": "{n} passos",
  "history.ok": "sucesso",
  "top.running": "Executando…",
  "top.settingsTitle": "Configurações",
  "top.untitled": "(sem título)",
  "top.modified": "●",

  // Paleta
  "palette.title": "Adicionar nó",
  "node.trigger": "Gatilho",
  "node.http": "Requisição HTTP",
  "node.command": "Rodar comando",
  "node.readfile": "Ler arquivo",
  "node.writefile": "Escrever arquivo",
  "node.transform": "Transformar (JS)",
  "node.condition": "Condição (if)",
  "node.delay": "Esperar",
  "node.notify": "Notificar",

  // Gatilhos em linguagem de gente (cards de descoberta)
  "trig.pick": "Quando este fluxo deve rodar?",
  "trig.folder.title": "Quando um arquivo aparecer numa pasta",
  "trig.folder.example": "Ex.: converter todo vídeo que cair em Downloads",
  "trig.interval.title": "A cada X minutos",
  "trig.interval.example": "Ex.: checar algo de 10 em 10 minutos",
  "trig.schedule.title": "Todo dia num horário",
  "trig.schedule.example": "Ex.: fazer um backup todo dia às 9h",
  "trig.startup.title": "Quando eu abrir o computador",
  "trig.startup.example": "Roda assim que você abre o app",
  "trig.manual.title": "Só quando eu mandar",
  "trig.manual.example": "Roda quando você clica em Executar",
  "trig.folder.choose": "Escolher pasta…",
  "trig.folder.chosen": "Quando um arquivo novo aparecer em {folder}",
  "trig.folder.none": "Nenhuma pasta escolhida ainda",
  "trig.folder.types": "Só estes tipos de arquivo (opcional)",
  "trig.folder.typesHint": "Ex.: mp4, mkv, mp3 — deixe vazio pra qualquer arquivo",
  "trig.interval.every": "A cada",
  "trig.interval.minutes": "minutos",
  "trig.schedule.at": "Todo dia às",
  "trig.startup.note": "Roda uma vez quando você abre o app e ativa o gatilho.",
  "trig.manual.note": "Roda quando você clica em Executar. Nada de segundo plano.",
  "trig.advanced": "Avançado (payload JSON)",
  "trig.bgNote":
    "Este gatilho roda sozinho: aperte “Ativar” lá em cima (funciona enquanto o app estiver aberto).",
  "trig.summary.folder": "Quando cair arquivo em {folder}",
  "trig.summary.folderEmpty": "Quando cair arquivo numa pasta (escolha a pasta)",
  "trig.summary.interval": "A cada {n} min",
  "trig.summary.schedule": "Todo dia às {time}",
  "trig.summary.startup": "Quando eu abrir o app",
  "trig.summary.manual": "Quando eu mandar (manual)",

  // Ativar gatilho (barra de cima)
  "top.arm": "Ativar gatilho",
  "top.armed": "Gatilho ativo ●",
  "top.armHint": "Liga o gatilho pra rodar sozinho enquanto o app está aberto",

  // Modelos prontos
  "tpl.title": "Modelos prontos",
  "tpl.notifyFolder": "Avisar quando cair arquivo numa pasta",
  "tpl.commandFolder": "Rodar um comando pra cada arquivo novo",
  "tpl.loaded": "Modelo carregado — escolha a pasta no gatilho e aperte Ativar.",

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
  "toast.needFolder": "Escolha uma pasta no gatilho primeiro.",
  "toast.watchStarted": "Vigiando {folder} — solte um arquivo lá pra disparar.",
  "toast.armFailed": "Não deu pra ativar o gatilho: {error}",
  "toast.fired": "Arquivo novo: {name}",
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
  "settings.themeNature": "Natureza",
  "settings.themeDarkBlue": "Azul escuro",
  "settings.themeCalmGreen": "Verde calmo",
  "settings.themePastelPink": "Rosa pastel",
  "settings.themePunkPrincess": "PunkPrincess",
  "settings.language": "Idioma",

  // Segundo plano (bandeja/autostart)
  "settings.background": "Segundo plano",
  "settings.closeToTray": "Fechar minimiza pra bandeja",
  "settings.closeToTrayHint":
    "O X esconde a janela em vez de sair; o app segue na bandeja. Com um gatilho ativado o X SEMPRE minimiza, mesmo com esta opção desligada — um agendamento que morre porque você fechou a janela não seria agendamento.",
  "settings.autostart": "Abrir com o sistema",
  "settings.autostartHint":
    "Sobe junto com o login, direto na bandeja (sem roubar a tela) — é o que faz um gatilho diário sobreviver a um reinício. A escolha fica guardada no app e é reimposta a cada boot: se o LocalAutomation mudar de pasta, o atalho de inicialização é reescrito sozinho.",
  "settings.autostartDisabledByOs":
    "A inicialização foi desligada pelo Gerenciador de Tarefas do Windows. Reative por lá, ou marque aqui de novo.",
  "tray.show": "Mostrar/Ocultar",
  "tray.quit": "Sair",
  "toast.settingsFailed": "Não deu pra salvar a configuração: {error}",

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
  "top.auto": "Auto:",
  "top.autoOff": "off",
  "top.autoHint": "Runs the flow automatically on an interval (while the app is open)",
  "history.title": "History",
  "history.subtitle": "Recent runs (last 40)",
  "history.empty": "No runs yet.",
  "history.steps": "{n} steps",
  "history.ok": "success",
  "top.running": "Running…",
  "top.settingsTitle": "Settings",
  "top.untitled": "(untitled)",
  "top.modified": "●",

  "palette.title": "Add node",
  "node.trigger": "Trigger",
  "node.http": "HTTP request",
  "node.command": "Run command",
  "node.readfile": "Read file",
  "node.writefile": "Write file",
  "node.transform": "Transform (JS)",
  "node.condition": "Condition (if)",
  "node.delay": "Wait",
  "node.notify": "Notify",

  "trig.pick": "When should this flow run?",
  "trig.folder.title": "When a file appears in a folder",
  "trig.folder.example": "E.g.: convert every video that lands in Downloads",
  "trig.interval.title": "Every X minutes",
  "trig.interval.example": "E.g.: check something every 10 minutes",
  "trig.schedule.title": "Every day at a set time",
  "trig.schedule.example": "E.g.: back something up every day at 9am",
  "trig.startup.title": "When I turn on the computer",
  "trig.startup.example": "Runs as soon as you open the app",
  "trig.manual.title": "Only when I say so",
  "trig.manual.example": "Runs when you click Run",
  "trig.folder.choose": "Choose folder…",
  "trig.folder.chosen": "When a new file appears in {folder}",
  "trig.folder.none": "No folder chosen yet",
  "trig.folder.types": "Only these file types (optional)",
  "trig.folder.typesHint": "E.g.: mp4, mkv, mp3 — leave empty for any file",
  "trig.interval.every": "Every",
  "trig.interval.minutes": "minutes",
  "trig.schedule.at": "Every day at",
  "trig.startup.note": "Runs once when you open the app and arm the trigger.",
  "trig.manual.note": "Runs when you click Run. No background.",
  "trig.advanced": "Advanced (JSON payload)",
  "trig.bgNote":
    "This trigger runs on its own: hit “Arm” up top (works while the app is open).",
  "trig.summary.folder": "When a file lands in {folder}",
  "trig.summary.folderEmpty": "When a file lands in a folder (pick the folder)",
  "trig.summary.interval": "Every {n} min",
  "trig.summary.schedule": "Every day at {time}",
  "trig.summary.startup": "When I open the app",
  "trig.summary.manual": "When I say so (manual)",

  "top.arm": "Arm trigger",
  "top.armed": "Trigger on ●",
  "top.armHint": "Turns the trigger on so it runs by itself while the app is open",

  "tpl.title": "Ready-made templates",
  "tpl.notifyFolder": "Notify me when a file lands in a folder",
  "tpl.commandFolder": "Run a command for each new file",
  "tpl.loaded": "Template loaded — pick the folder in the trigger and hit Arm.",

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
  "toast.needFolder": "Pick a folder in the trigger first.",
  "toast.watchStarted": "Watching {folder} — drop a file there to fire it.",
  "toast.armFailed": "Couldn't arm the trigger: {error}",
  "toast.fired": "New file: {name}",
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
  "settings.themeNature": "Nature",
  "settings.themeDarkBlue": "Dark blue",
  "settings.themeCalmGreen": "Calm green",
  "settings.themePastelPink": "Pastel pink",
  "settings.themePunkPrincess": "PunkPrincess",
  "settings.language": "Language",

  "settings.background": "Background",
  "settings.closeToTray": "Closing minimizes to the tray",
  "settings.closeToTrayHint":
    "The X hides the window instead of quitting; the app stays in the tray. With a trigger armed the X ALWAYS minimizes, even with this option off — a schedule that dies because you closed the window would not be a schedule.",
  "settings.autostart": "Start with the system",
  "settings.autostartHint":
    "Starts at login, straight into the tray (without taking over the screen) — that is what lets a daily trigger survive a restart. The choice is stored in the app and reapplied on every boot: if LocalAutomation moves to another folder, the startup entry is rewritten by itself.",
  "settings.autostartDisabledByOs":
    "Startup was turned off in the Windows Task Manager. Re-enable it there, or tick this box again.",
  "tray.show": "Show/Hide",
  "tray.quit": "Quit",
  "toast.settingsFailed": "Could not save the setting: {error}",

  "settings.about":
    " — 100% local flow automation (offline n8n/Zapier): build node graphs (HTTP, command, files, JS, condition), run instantly and save as .tflow. Deliberately unsandboxed: nodes run with YOUR permissions — review third-party flows. Part of the Local suite.",
};

const es: Record<MessageKey, string> = {
  "top.new": "Nuevo",
  "top.open": "Abrir…",
  "top.save": "Guardar",
  "top.saveAs": "Guardar como…",
  "top.run": "▶ Ejecutar",
  "top.auto": "Auto:",
  "top.autoOff": "apagado",
  "top.autoHint": "Ejecuta el flujo automáticamente en un intervalo (mientras la app esté abierta)",
  "history.title": "Historial",
  "history.subtitle": "Ejecuciones recientes (últimas 40)",
  "history.empty": "Aún no hay ejecuciones.",
  "history.steps": "{n} pasos",
  "history.ok": "éxito",
  "top.running": "Ejecutando…",
  "top.settingsTitle": "Configuración",
  "top.untitled": "(sin título)",
  "top.modified": "●",

  "palette.title": "Añadir nodo",
  "node.trigger": "Disparador",
  "node.http": "Petición HTTP",
  "node.command": "Ejecutar comando",
  "node.readfile": "Leer archivo",
  "node.writefile": "Escribir archivo",
  "node.transform": "Transformar (JS)",
  "node.condition": "Condición (if)",
  "node.delay": "Esperar",
  "node.notify": "Notificar",

  "trig.pick": "¿Cuándo debe ejecutarse este flujo?",
  "trig.folder.title": "Cuando aparezca un archivo en una carpeta",
  "trig.folder.example": "Ej.: convertir todo video que caiga en Descargas",
  "trig.interval.title": "Cada X minutos",
  "trig.interval.example": "Ej.: revisar algo cada 10 minutos",
  "trig.schedule.title": "Todos los días a una hora",
  "trig.schedule.example": "Ej.: hacer un respaldo todos los días a las 9",
  "trig.startup.title": "Cuando encienda la computadora",
  "trig.startup.example": "Se ejecuta apenas abres la app",
  "trig.manual.title": "Solo cuando yo lo diga",
  "trig.manual.example": "Se ejecuta cuando pulsas Ejecutar",
  "trig.folder.choose": "Elegir carpeta…",
  "trig.folder.chosen": "Cuando aparezca un archivo nuevo en {folder}",
  "trig.folder.none": "Aún no elegiste carpeta",
  "trig.folder.types": "Solo estos tipos de archivo (opcional)",
  "trig.folder.typesHint": "Ej.: mp4, mkv, mp3 — vacío para cualquier archivo",
  "trig.interval.every": "Cada",
  "trig.interval.minutes": "minutos",
  "trig.schedule.at": "Todos los días a las",
  "trig.startup.note": "Se ejecuta una vez cuando abres la app y activas el disparador.",
  "trig.manual.note": "Se ejecuta cuando pulsas Ejecutar. Sin segundo plano.",
  "trig.advanced": "Avanzado (payload JSON)",
  "trig.bgNote":
    "Este disparador corre solo: pulsa “Activar” arriba (funciona mientras la app esté abierta).",
  "trig.summary.folder": "Cuando caiga un archivo en {folder}",
  "trig.summary.folderEmpty": "Cuando caiga un archivo en una carpeta (elige la carpeta)",
  "trig.summary.interval": "Cada {n} min",
  "trig.summary.schedule": "Todos los días a las {time}",
  "trig.summary.startup": "Cuando abra la app",
  "trig.summary.manual": "Cuando yo lo diga (manual)",

  "top.arm": "Activar disparador",
  "top.armed": "Disparador activo ●",
  "top.armHint": "Enciende el disparador para que corra solo mientras la app está abierta",

  "tpl.title": "Plantillas listas",
  "tpl.notifyFolder": "Avisarme cuando caiga un archivo en una carpeta",
  "tpl.commandFolder": "Ejecutar un comando por cada archivo nuevo",
  "tpl.loaded": "Plantilla cargada — elige la carpeta en el disparador y pulsa Activar.",

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
  "toast.needFolder": "Elige una carpeta en el disparador primero.",
  "toast.watchStarted": "Vigilando {folder} — suelta un archivo ahí para dispararlo.",
  "toast.armFailed": "No se pudo activar el disparador: {error}",
  "toast.fired": "Archivo nuevo: {name}",
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
  "settings.themeNature": "Naturaleza",
  "settings.themeDarkBlue": "Azul oscuro",
  "settings.themeCalmGreen": "Verde tranquilo",
  "settings.themePastelPink": "Rosa pastel",
  "settings.themePunkPrincess": "PunkPrincess",
  "settings.language": "Idioma",

  "settings.background": "Segundo plano",
  "settings.closeToTray": "Cerrar minimiza a la bandeja",
  "settings.closeToTrayHint":
    "La X oculta la ventana en vez de salir; la app sigue en la bandeja. Con un disparador activado la X SIEMPRE minimiza, incluso con esta opción desactivada — una programación que muere porque cerraste la ventana no sería una programación.",
  "settings.autostart": "Abrir con el sistema",
  "settings.autostartHint":
    "Arranca junto con el inicio de sesión, directo a la bandeja (sin apropiarse de la pantalla) — es lo que permite que un disparador diario sobreviva a un reinicio. La elección se guarda en la app y se reimpone en cada arranque: si LocalAutomation cambia de carpeta, la entrada de inicio se reescribe sola.",
  "settings.autostartDisabledByOs":
    "El inicio automático fue desactivado desde el Administrador de tareas de Windows. Reactívalo allí, o vuelve a marcar esta casilla.",
  "tray.show": "Mostrar/Ocultar",
  "tray.quit": "Salir",
  "toast.settingsFailed": "No se pudo guardar la configuración: {error}",

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
