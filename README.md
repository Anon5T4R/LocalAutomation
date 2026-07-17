# LocalAutomation

Automação de fluxos **100% local** da suíte Local — o n8n/Zapier offline: a
"cola" entre os apps. "Quando X, faça Y", sem nuvem, sem conta.

## Recursos (v0.1)

- **Editor de nós** (React Flow): canvas com arrastar/conectar, minimap,
  seleção com painel de configuração por tipo
- **Nós base:** Gatilho manual (payload JSON opcional) · Requisição HTTP ·
  Rodar comando (shell do SO) · Ler arquivo · Escrever arquivo ·
  **Transformar (JS)** · **Condição (if)** com saídas true/false ·
  **Esperar** (delay em ms) · **Notificar** (notificação da bandeja do SO)
- **Executar na hora** com **log por nó** (status, duração, preview da saída;
  nó colore no canvas: rodando/ok/erro) — erro num ramo não derruba os outros
- **`.tflow`** = JSON do grafo: salvar/abrir (associação registrada);
  **abrir arquivo de terceiro mostra aviso** pra revisar comando/JS antes de
  executar (Decisão nº 8: sem sandbox — os nós rodam com as suas permissões)
- Motor JS = **boa** (100% Rust, sem dependência C) — `input` é a entrada do
  nó, a última expressão é a saída
- Tema claro/escuro/sistema · UI em **PT/EN/ES**

**Roadmap:** v0.2 = gatilhos reais (cron, pasta observada, webhook local) em
background/bandeja, variáveis e credenciais (keyring), loop/merge, retry ·
v0.3 = integrações da suíte + **template de destaque de BACKUP** (Decisão
nº 6) · v0.4 = nó de LLM local (runtime compartilhado).

## Stack

Tauri 2 + React 19 + Vite + TS (`@xyflow/react`); Rust no back (executor DAG
próprio, `boa_engine`, `reqwest` rustls).

## Dev

```bash
npm install
npm run tauri dev   # porta 1472
```

## Release

Tag `vX.Y.Z` → GitHub Actions builda NSIS (Windows) + AppImage (Linux) e
publica a Release. Parte da suíte [Local](https://github.com/Anon5T4R).

## Licença

MIT
