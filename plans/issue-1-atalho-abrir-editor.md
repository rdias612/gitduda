# Plano de Desenvolvimento: Atalho Global `Ctrl+Shift+A` para Abrir Editor Configurado e Configuração em Settings

- **Data**: 2026-11-18
- **Status**: [ ] Planejado
- **Debate Técnico**: Validado e revisado
- **Arquivo de Controle**: `plans/issue-1-atalho-abrir-editor.md`
- **Issue**: `#1` ([Issue #1](https://github.com/rdias612/gitduda/issues/1))
- **Spec de Origem**: `spec/atalho-abrir-editor.md`

## 1. Visão Geral e Regras Locais Aplicáveis
- **Objetivo**: Registrar o atalho global `Ctrl+Shift+A` (`Cmd+Shift+A` no macOS) para abrir a pasta do repositório ativo no editor configurado (`core.editor` / env vars), unificando o comportamento de `"Open in Editor"` para avisar com um toast + botão `"Configure Editor"` quando nenhum editor estiver configurado (sem cair no explorador de arquivos do SO) e adicionando o campo **Editor** na seção *External Tools* do `gn-settings-dialog`.
- **Restrições do Projeto (`.github/copilot-instructions.md`)**:
  - **Tauri IPC (`snake_case` vs `camelCase`)**: Sempre usar `snake_case` em Rust (`#[serde(rename_all = "camelCase")]` nas structs serializadas) e `camelCase` no TypeScript; nunca introduzir chaves `snake_case` em chamadas/tipos TS.
  - **Lit Web Components & TypeScript**: Decoradores `@customElement`, `@property()`, `@state()`, tipagem estrita sem `any`, paths/imports com `.ts`, limpeza de listeners no ciclo de vida.
  - **Rust / Backend**: `cargo fmt` e `cargo clippy` sem warnings; seguir `Result<T>` via `crate::error::Result`; testes unitários em `#[cfg(test)]` com `TestRepo`.
  - **Qualidade de Testes**: Unitários obrigatórios em `src/**/__tests__/*.test.ts` (`@open-wc/testing`, mocks de Tauri invoke) e E2E em `e2e/tests/*.spec.ts` (Playwright). Não remover nem enfraquecer testes existentes.
  - **Fora de Escopo**: Não adicionar item no menu nativo (`app-menu.service.ts` / `src-tauri/src/menu.rs`), não abrir arquivo individual selecionado e não fazer auto-detecção de binários instalados no SO.
- **Comando de Build Global**: `npm run lint && npm run typecheck && npm test && cd src-tauri && cargo fmt --check && cargo clippy`

## 2. Checklist de Execução de Etapas
- [ ] Etapa 1: Contrato de Backend (`OpenResult` + Clear Global) e Toast de Ação em `openRepositoryInEditor` / Paleta de Arquivos
- [ ] Etapa 2: Navegação Direta para Settings (`dialog.store.ts`) e Campo Editor em `gn-settings-dialog.ts`
- [ ] Etapa 3: Atalho Global `Ctrl+Shift+A` (com Guard contra `e.repeat`), Badge na Command Palette e Wiring no `app-shell.ts`
- [ ] Etapa 4: Testes E2E e Check Global Final

---

## 3. Especificação das Etapas

### Etapa 1: Contrato de Backend (`OpenResult` + Clear Global) e Toast de Ação em `openRepositoryInEditor` / Paleta de Arquivos
- **Objetivo**: Fazer `open_in_configured_editor` retornar indicação explícita quando nenhum editor estiver configurado (em vez de abrir o explorador de arquivos do SO para diretórios), permitir limpar/salvar `core.editor` global em `set_editor_config` mesmo sem repositório aberto, e compartilhar no frontend um helper de tratamento de `notConfigured` entre `openRepositoryInEditor` e `handleOpenFileFromPalette` para exibir um toast `warning` com botão `"Configure Editor"`.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src-tauri/src/commands/file.rs#L26-L35` (`OpenResult`)
  - `src-tauri/src/commands/file.rs#L269-L355` (`open_in_configured_editor`, `no_editor_configured_for_file`, `editor_unavailable`)
  - `src-tauri/src/commands/file.rs#L531-L574` (`get_editor_config`, `set_editor_config`)
  - `src-tauri/src/commands/file.rs#L654-L665` e `#L790-L816` (testes unitários Rust)
  - `src-tauri/src/commands/config.rs#L127-L135` (`run_git_config_unset` — reutilizar como `pub(crate)`, sem alterar assinatura)
  - `src/services/git/ai-workspace.ts#L36-L43` (`OpenResult`)
  - `src/services/open-location.service.ts#L23-L80` (`reportFailure`, `openRepositoryInEditor`, novo helper compartilhado `handleOpenEditorFailure`)
  - `src/app-shell.ts#L5965-L5986` (`handleOpenFileFromPalette`)
  - `src/__tests__/app-shell-palette-open-location.test.ts#L134-L208`
  - `src/__tests__/app-shell-palette-open-file.test.ts#L142-L154`
- **Instruções Técnicas**:
  1. Em `src-tauri/src/commands/file.rs`, adicionar ao `OpenResult` (`#[serde(rename_all = "camelCase")]`) o campo opcional `#[serde(default, skip_serializing_if = "Option::is_none")] pub not_configured: Option<bool>` (e garantir que inicializações existentes de `OpenResult` no arquivo incluam `not_configured: None`).
  2. Em `open_in_configured_editor` (`file.rs#L296-L340`):
     - Filtrar strings vazias na leitura das variáveis de ambiente `GIT_EDITOR`, `VISUAL` e `EDITOR` (`.filter(|v| !v.trim().is_empty())`) e extrair o núcleo para um helper interno `open_in_configured_editor_with_env(..., env_var: impl Fn(&str) -> Option<String>)` (o comando Tauri repassa `|k| std::env::var(k).ok()`, permitindo aos testes unitários passar `|_| None` sem depender de `GIT_EDITOR=true` no CI).
     - Alterar o braço `None if target.is_dir()` para não chamar `open_in_default_app`: retornar `Ok(no_editor_configured_for_target(target.is_dir()))` (ou helper equivalente) com `success: false`, `not_configured: Some(true)` e mensagem clara citando exemplo (para diretório: `"No editor is configured (e.g., VS Code). Configure an editor in Settings."`; para arquivo: manter a frase `"Configure a GUI editor in Git settings"` para preservar compatibilidade com `missing_editor_for_file_reports_configuration_guidance` em `#L655`, também com `not_configured: Some(true)`). Manter o braço de editor de terminal (`is_terminal_editor(&editor_cmd) && target.is_dir()`) exatamente como está hoje.
  3. Em `set_editor_config` e `get_editor_config` (`file.rs#L531-L574`):
     - Extrair helpers internos locais a `file.rs` aceitando `global_config_override: Option<&Path>` (mantendo `None` nos comandos Tauri de produção): quando `Some(cfg_path)` for passado nos testes, usar `["--file", cfg_path]` no lugar de `["--global"]` ao chamar `git config` / `super::config::run_git_config_unset` (contornando `GIT_CONFIG_GLOBAL=/dev/null` de `command.rs#L624` em `#[cfg(test)]` sem mutex global e sem alterar `config.rs`).
     - Quando `editor.trim().is_empty()` em `set_editor_config`, executar `super::config::run_git_config_unset` no escopo correspondente (`None` + `["--global", "--unset", "core.editor"]` quando `global == true`) e retornar `OpenResult { success: true, message: Some(...), notice: None, not_configured: None }`, para que selecionar `"None"` em Settings limpe `core.editor` global sem exigir repositório aberto. Em `get_editor_config`, garantir que quando `global == true` e `path` for vazio, consulte apenas o escopo global sem falhar.
  4. Em `src/services/git/ai-workspace.ts#L37-L42`, espelhar `notConfigured?: boolean;` em `OpenResult`.
  5. Em `src/services/open-location.service.ts#L67-L80`, extrair um helper compartilhado `export function handleOpenEditorFailure(result: CommandResult<OpenResult>, fallbackMessage: string): false`:
     - Se `result.data?.notConfigured === true`, exibir `showToast(text, 'warning', 8000, { label: 'Configure Editor', callback: () => { dialogs.open('settings', { focusSection: 'external-tools' }); } })` (importando `dialogs` de `../stores/dialog.store.ts`) e retornar `false`.
     - Caso contrário, chamar `reportFailure(fallbackMessage, result.data?.message ?? result.error?.message)` (toast `error`).
     - Reutilizar `handleOpenEditorFailure` em `openRepositoryInEditor` (`'Failed to open editor'`) e em `handleOpenFileFromPalette` (`src/app-shell.ts#L5978-L5983`, `'Failed to open file in editor'`), preservando o toast `info` para `result.data?.notice`.
  6. Adicionar/atualizar testes unitários: em Rust (`file.rs`), testar `open_in_configured_editor_with_env` em diretório sem editor configurado (`|_| None`, verificando `not_configured == Some(true)` e `success == false`) e clear global via `set_editor_config` com arquivo temporário isolado (`--file`); em TS, expandir `src/__tests__/app-shell-palette-open-location.test.ts` (toast `warning` + ação `"Configure Editor"` abrindo `dialogs.open('settings', { focusSection: 'external-tools' })`) e adicionar regressão em `src/__tests__/app-shell-palette-open-file.test.ts`.
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 3` (altera contrato compartilhado Rust <-> TS `OpenResult`: executar `cargo test --manifest-path src-tauri/Cargo.toml commands::file::tests`, `npm run typecheck` e testes unitários focais).
- **Critérios de Aceitação (Code Review)**:
  - `open_in_configured_editor` em diretório sem editor configurado retorna `success: false` e `notConfigured: true` sem chamar `open_in_default_app`.
  - `set_editor_config` com `global: true` e `editor` vazio remove `core.editor` global sem erro mesmo com `path` vazio.
  - `openRepositoryInEditor` mostra toast `warning` com ação `"Configure Editor"` apenas quando `notConfigured` é `true`, mantendo toast `error` para falhas de spawn ou backend.
  - Nenhum campo `snake_case` em tipos/chamadas TypeScript.

---

### Etapa 2: Navegação Direta para Settings (`dialog.store.ts`) e Campo Editor em `gn-settings-dialog.ts`
- **Objetivo**: Suportar contexto `{ focusSection?: 'external-tools' }` ao abrir `settings` no `dialog.store.ts`, repassá-lo no `app-shell.ts` e adicionar o campo **Editor** (`core.editor` global) na seção *External Tools* de `gn-settings-dialog.ts` com presets comuns (`VS Code (code)`, `Cursor`, `Zed`, `Sublime Text`, `Custom...`, `None`), rolagem automática e foco quando direcionado.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src/stores/dialog.store.ts#L148-L170` (`SettingsDialogContext`, `DialogContexts`)
  - `src/app-shell.ts#L6445-L6458` (renderização de `<gn-settings-dialog>`)
  - `src/utils/focus.ts#L27-L31` (`containsDeepActiveElement`)
  - `src/components/dialogs/gn-settings-dialog.ts#L2` (import `property`), `#L531-L605` (state, `connectedCallback`, `updated`, `syncSelectValue`), `#L868-L905` (`loadExternalToolsConfig`), `#L1973-L2056` (seção *External Tools*)
  - `src/stores/__tests__/dialog.store.test.ts`
  - `src/utils/__tests__/focus.test.ts#L44-L80`
  - `src/components/dialogs/__tests__/gn-settings-dialog-external-tools.test.ts`
- **Instruções Técnicas**:
  1. Em `src/stores/dialog.store.ts`, definir `export interface SettingsDialogContext { focusSection?: 'external-tools'; }` e registrar `settings: SettingsDialogContext;` em `DialogContexts` (`#L161-L166`), mantendo `dialogs.open('settings')` sem argumento totalmente compatível.
  2. Em `src/app-shell.ts#L6452-L6455`, passar `.focusSection=${dialogs.context('settings')?.focusSection ?? null}` (usando `dialogs.context`, pois `getContext` não existe) para `<gn-settings-dialog>`.
  3. Em `src/utils/focus.ts#L27-L31`, atualizar `containsDeepActiveElement(host)` para caminhar pelos nós ancestrais via `getRootNode()` / `ShadowRoot.host` a partir de `deepActiveElement()` até verificar se `host.contains(node) || (host.shadowRoot?.contains(node) ?? false)` — garantindo que `<gn-modal>` reconheça elementos focados dentro do `shadowRoot` de filhos *slotted* (como `<gn-settings-dialog>`) e não roube o foco para o botão `×` no `requestAnimationFrame` de abertura (`gn-modal.ts#L225-L244`).
  4. Em `src/components/dialogs/gn-settings-dialog.ts`:
     - Atualizar o import em `#L2` para `import { customElement, property, state } from 'lit/decorators.js';` e declarar `@property({ type: String }) focusSection: 'external-tools' | null = null;`.
     - Definir presets estáticos de editor (ex.: `EDITOR_PRESETS`: `code` -> `VS Code (code)`, `cursor` -> `Cursor (cursor)`, `zed` -> `Zed (zed)`, `subl` -> `Sublime Text (subl)`) e `@state() private editorSelectValue: string = ''`, `@state() private editorCustomCmd: string = ''`, `@state() private loadingEditor = false`, além de token monotônico `private editorWriteToken = 0` e flag `private pendingEditorFocus = false` (seguindo o padrão de `mergeToolWriteToken` / `diffToolWriteToken` em `#L543-L544`).
     - Carregar a configuração global via `gitService.getEditorConfig(repoPath ?? '', true)` no `loadExternalToolsConfig` com `this.loadingEditor = true` e garantia de `this.loadingEditor = false` em `finally` (sem exigir `repo` aberto — separar do guard `if (!repo) return;` de Merge/Diff tool). Se o valor retornado corresponder a um preset conhecido, setar `editorSelectValue` para o comando; caso contrário, se não vazio, setar `editorSelectValue = '__custom__'` e `editorCustomCmd = value`.
     - Na seção **External Tools** (`#L1973-L2056`, adicionando `id="external-tools-section"`), renderizar a linha **Editor** (`id="external-editor-select"`, `?disabled=${this.loadingEditor}` + input condicional quando `editorSelectValue === '__custom__'`, placeholder `e.g., code --wait` ou `e.g., code`) **fora** do bloco condicional `repositoryStore.getState().getActiveRepository()` para que funcione com ou sem repositório aberto, manter *Merge Tool* / *Diff Tool* logo abaixo no condicional e ajustar o texto do ramo `else` em `#L2051` de `msg('Open a repository to configure external tools')` para `msg('Open a repository to configure merge and diff tools')`.
     - Implementar `handleEditorSelectChange` e `handleEditorCustomCmdChange` chamando `gitService.setEditorConfig(repo?.repository.path ?? '', cmd, true)`, com proteção de token monotônico, rollback em caso de erro + toast de erro e `window.dispatchEvent(new CustomEvent('settings-changed'))`. Re-sincronizar `#external-editor-select` em `updated()` via `syncSelectValue('#external-editor-select', this.editorSelectValue)`.
     - Ativar `this.pendingEditorFocus = true` na abertura/mudança de `focusSection === 'external-tools'` e consumir em `updated()` somente quando `this.pendingEditorFocus && !this.loadingEditor`: rolar `#external-tools-section` para a vista (`scrollIntoView({ block: 'center' })`), verificar se `#external-editor-select` existe e não está `disabled`, executar `.focus()` e resetar `this.pendingEditorFocus = false`.
  5. Adicionar testes unitários em `src/stores/__tests__/dialog.store.test.ts` (contexto `settings`), `src/utils/__tests__/focus.test.ts` (foco dentro de shadow root de filho *slotted*) e `src/components/dialogs/__tests__/gn-settings-dialog-external-tools.test.ts` (carregamento do editor global, seleção de preset `"code"`, entrada customizada, limpeza com `"None"`, rollback em falha e foco automático após concluir `loadingEditor` quando `focusSection === 'external-tools'`).
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 2` (`npm run lint`, `npm run typecheck` e execução dos testes unitários focais de `dialog.store.test.ts` e `gn-settings-dialog-external-tools.test.ts`).
- **Critérios de Aceitação (Code Review)**:
  - Campo **Editor** aparece em *External Tools* mesmo sem repositório aberto e persiste `core.editor` global via `setEditorConfig(..., true)`.
  - Valores customizados fora da lista de presets são exibidos no input `Custom...` sem serem sobrescritos para `None` em `updated()`.
  - Abrir `dialogs.open('settings', { focusSection: 'external-tools' })` rola até *External Tools* e foca `#external-editor-select`.

---

### Etapa 3: Atalho Global `Ctrl+Shift+A` (com Guard contra `e.repeat`), Badge na Command Palette e Wiring no `app-shell.ts`
- **Objetivo**: Registrar o atalho padrão `'open-in-editor'` (`Ctrl+Shift+A` / `Cmd+Shift+A`) em `keyboard.service.ts` ignorando `e.repeat`, conectá-lo em `app-shell.ts` via `requiresRepository` para chamar `openRepositoryInEditor` e exibir o badge de atalho no comando `'open-in-editor'` da Command Palette.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src/services/keyboard.service.ts#L8-L18` (`Shortcut`), `#L188-L195` (`register`), `#L230-L231` (`rebind`), `#L289` (`resetBinding`), `#L311` (`resetAllBindings`), `#L581-L587` (`handleKeyDown`), `#L707-L736` e `#L851-L870` (`registerDefaultShortcuts`)
  - `src/app-shell.ts#L2190-L2223` (`registerDefaultShortcuts`)
  - `src/palette-commands.ts#L892-L900` (`open-in-editor`)
  - `src/services/__tests__/keyboard.service.test.ts`
  - `src/__tests__/app-shell-palette-open-location.test.ts`
- **Instruções Técnicas**:
  1. Em `src/services/keyboard.service.ts`:
     - Adicionar `noRepeat?: boolean;` em `Shortcut` (`#L8-L17`), preservando essa flag em `shortcutMetadata` (`#L71`, `#L181-L185`) e ao reconstruir `Shortcut` em `register`, `rebind`, `resetBinding` e `resetAllBindings` (para que o rebind no Diálogo de Atalhos mantenha a proteção de `noRepeat`).
     - Em `handleKeyDown` (`#L583-L587`), se `shortcut` existir: manter `e.preventDefault(); e.stopPropagation();` e só chamar `shortcut.action()` quando `!(e.repeat && shortcut.noRepeat)`.
     - Adicionar `openInEditor?: () => void;` nos parâmetros de `registerDefaultShortcuts` (`#L707-L736`) e, quando fornecido, registrar `'open-in-editor'` com `{ key: 'a', ctrl: true, shift: true, noRepeat: true, action: actions.openInEditor, description: 'Open repository in configured editor', category: 'General' }`.
  2. Em `src/app-shell.ts#L2190-L2222`, passar `openInEditor: this.requiresRepository(() => { void openRepositoryInEditor(this.activeRepository!.repository.path); })` em `registerDefaultShortcuts(...)` (reutilizando a função já importada ou importando `openRepositoryInEditor` de `./services/open-location.service.ts`).
  3. Em `src/palette-commands.ts#L892-L900`, adicionar `shortcut: `${mod}⇧A`` no item `'open-in-editor'` (seguindo exatamente a convenção de `create-branch` em `#L187`, ou formatando a partir do binding ativo se preferir manter sincronizado).
  4. Adicionar testes unitários em `src/services/__tests__/keyboard.service.test.ts` (registro de `'open-in-editor'` em `Ctrl+Shift+A`, categoria `'General'`, bloqueio quando `e.repeat === true` mesmo após `rebind`, e funcionamento em inputs já que `mod+shift+a` não colide com `mod+a`) e em `src/__tests__/app-shell-palette-open-location.test.ts` (disparo via `Ctrl+Shift+A` no `app-shell`, aviso sem repositório e bloqueio de `e.repeat`).
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 2` (`npm run lint`, `npm run typecheck`, `keyboard.service.test.ts` e `app-shell-palette-open-location.test.ts`).
- **Critérios de Aceitação (Code Review)**:
  - `Ctrl+Shift+A` (Windows/Linux) e `Cmd+Shift+A` (macOS) acionam `openRepositoryInEditor` para o repositório ativo.
  - Segurar as teclas (`e.repeat: true`) executa a ação apenas uma vez, inclusive após o usuário rebindar o atalho.
  - O atalho aparece no Diálogo de Atalhos em `General` e como badge no item `"Open in Editor"` da Command Palette.
  - Pressionar `Ctrl+Shift+A` sem repositório aberto exibe `"Please open a repository first"` sem chamar o backend; com o foco em `gn-terminal-panel`, respeita a regra de terminal.

---

### Etapa 4: Testes E2E e Check Global Final
- **Objetivo**: Cobrir os fluxos de ponta a ponta com Playwright (atalho `Ctrl+Shift+A` abrindo editor configurado, aviso + botão `Configure Editor` abrindo Settings -> External Tools -> Editor, e salvamento de `core.editor` global) e validar todas as checagens obrigatórias de pré-commit do projeto.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `e2e/tests/keyboard.spec.ts`
  - `e2e/tests/settings-external-tools.spec.ts`
- **Instruções Técnicas**:
  1. Em `e2e/tests/keyboard.spec.ts`, adicionar testes para:
     - `Ctrl+Shift+A` (`Meta+Shift+A` / `Control+Shift+A` conforme padrão do arquivo) com repositório aberto invocando `open_in_configured_editor` com o caminho do repositório.
     - `Ctrl+Shift+A` quando `open_in_configured_editor` retorna `{ success: false, notConfigured: true, message: ... }` exibindo toast com botão `"Configure Editor"` que, ao ser clicado, abre `gn-settings-dialog` com `#external-editor-select` focado.
  2. Em `e2e/tests/settings-external-tools.spec.ts`, adicionar teste selecionando `VS Code (code)` (e/ou comando customizado) no campo **Editor**, verificando a chamada de `set_editor_config` com `global: true` e persistência na reabertura.
  3. Executar a bateria global de verificação exigida pelo `.github/copilot-instructions.md`:
     - `npm run lint`
     - `npm run typecheck`
     - `npm test`
     - `cd src-tauri && cargo fmt --check && cargo clippy && cargo test`
     - Verificação de ausência de `snake_case` em chamadas Tauri TS e execução dos testes E2E (`npx playwright test e2e/tests/keyboard.spec.ts e2e/tests/settings-external-tools.spec.ts` com config em `e2e/playwright.config.ts`).
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 3` (check global completo JS + Rust + E2E).
- **Critérios de Aceitação (Code Review)**:
  - Todos os testes unitários, E2E focais, `eslint`, `tsc`, `cargo fmt --check` e `cargo clippy` passam sem erros ou regressões.
  - Nenhuma alteração fora do escopo (menu nativo intacto, sem `snake_case` em payloads TS).
