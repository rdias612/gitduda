# Plano de Desenvolvimento: Atalho Configurável para Abrir Repositório no Editor de Código

- **Data**: 2026-11-23
- **Status**: [ ] Em Execução
- **Debate Técnico**: Validado e revisado
- **Arquivo de Controle**: `plans/issue-1-atalho-abrir-editor.md`
- **Issue**: `#1` ([rdias612/gitduda#1](https://github.com/rdias612/gitduda/issues/1))

## 1. Visão Geral e Regras Locais Aplicáveis
- **Objetivo**: Implementar suporte completo à abertura do repositório ativo no editor/IDE configurado pelo usuário via atalho global configurável `Ctrl+Shift+A` (`Cmd+Shift+A` no macOS) e paleta de comandos, com detecção de editores instalados coerentes com o SO em **Settings → External Tools → Editor** (sem opção `None`, mantendo `Custom...` e disponível com ou sem repositório aberto), validação prévia da existência do executável antes do spawn e toast com ação **Configure Editor** (sem fallback silencioso para o explorador de arquivos).
- **Decisões Alinhadas com o Usuário**:
  - **Fonte da verdade**: Requisitos completos da Issue #1 (sobrescrevendo o escopo anterior de `spec/atalho-abrir-editor.md` que omitia detecção de editores no SO e validação de executável).
  - **Lista no seletor de Editor**: Exibir apenas os editores detectados como instalados no SO atual (com fallback para os presets coerentes com o SO caso nenhum seja detectado), sempre mantendo a opção **`Custom...`** para comando manual.
  - **Sem opção `None`**: Não exibir opção `None` no `<select>` de Editor; quando nenhum editor estiver configurado ainda, mostrar apenas um placeholder desabilitado (`Select an editor...`).
- **Restrições do Projeto (`.github/copilot-instructions.md` e contratos locais)**:
  - **Tauri IPC (`snake_case` vs `camelCase`)**: Sempre usar `snake_case` em Rust (`#[serde(rename_all = "camelCase")]` nas structs serializadas) e `camelCase` no TypeScript; nunca introduzir chaves `snake_case` em chamadas/tipos TS.
  - **Registro IPC (`scripts/ipc-contract.test.mjs`)**: Todo novo comando `#[tauri::command]` (como `get_available_editors`) deve ser registrado em `tauri::generate_handler![...]` em `src-tauri/src/lib.rs`.
  - **Cobertura i18n (`scripts/i18n-coverage.test.mjs`)**: `gn-settings-dialog.ts` é arquivo localizado; toda string nova ou alterada em `msg(...)` exige extração (`npm run localize:extract`), tradução em `src/i18n/xliff/fr.xlf` (removendo entradas órfãs) e rebuild (`npm run localize:build` → `src/i18n/generated/locales/fr.ts`).
  - **Barrel Exports (`src/services/__tests__/git-service-modules.test.ts`)**: Novas funções exportadas em `src/services/git/ai-workspace.ts` (como `getAvailableEditors`) devem constar em `keyExports` no teste de contrato do barrel.
  - **Lit Web Components & TypeScript**: Decoradores `@customElement`, `@property()`, `@state()`, tipagem estrita sem `any`, imports com extensão `.ts`, limpeza de listeners no ciclo de vida.
  - **Rust / Backend**: `cargo fmt` e `cargo clippy` sem warnings; `Result<T>` via `crate::error::Result`; testes unitários em `#[cfg(test)]` com `TestRepo` e isolamento sem mutar config global do host (`GIT_CONFIG_GLOBAL=/dev/null` em `src-tauri/src/utils/command.rs#L624`).
  - **Qualidade de Testes**: Unitários obrigatórios em `src/**/__tests__/*.test.ts` (`@open-wc/testing`) e E2E em `e2e/tests/*.spec.ts` (Playwright). Não remover nem enfraquecer testes existentes.
  - **Fora de Escopo**: Não abrir arquivo individual selecionado via `Ctrl+Shift+A`, não adicionar item ao menu nativo (`src-tauri/src/menu.rs` / `src/services/app-menu.service.ts`) e não atuar em repositórios diferentes do repositório ativo.
- **Comando de Build Global**: `npm run lint && npm run typecheck && npm test && cd src-tauri && cargo fmt --check && cargo clippy && cargo test`

## 2. Checklist de Execução de Etapas
- [x] Etapa 1: Backend de Detecção e Resolução de Editores, Contrato `OpenResult`/`DialogContexts` e Toast de Ação
- [ ] Etapa 2: Navegação Direta para Settings (`app-shell.ts` / `focus.ts`), Campo Editor em `gn-settings-dialog.ts` e Sincronização i18n
- [ ] Etapa 3: Atalho Global `Ctrl+Shift+A` (com Guard contra `e.repeat`), Badge na Paleta de Comandos e Wiring no `app-shell.ts`
- [ ] Etapa 4: Mocks E2E, Testes Playwright e Validação Global Final

---

## 3. Especificação das Etapas

### Etapa 1: Backend de Detecção e Resolução de Editores, Contrato `OpenResult`/`DialogContexts` e Toast de Ação
- **Objetivo**: Adicionar detecção de editores disponíveis coerentes com o SO (`get_available_editors`), resolver e validar o caminho real do executável antes do spawn em `open_in_configured_editor` (suportando shims `.cmd`/`.bat` no Windows e informando erro claro quando não encontrado), retornar `notConfigured: true` sem fallback silencioso para o explorador de arquivos quando não houver editor configurado, suportar `get_editor_config`/`set_editor_config` globais sem exigir repositório aberto, antecipar o contrato de tipo `SettingsDialogContext` em `dialog.store.ts` e unificar o toast `warning` com botão `"Configure Editor"` no frontend.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src-tauri/src/commands/file.rs#L25-L45` (`OpenResult`, novo struct `EditorInfo`)
  - `src-tauri/src/commands/file.rs#L267-L355` (`open_in_configured_editor`, `no_editor_configured_for_file`, `editor_unavailable`, novos helpers `resolve_editor_executable`, `is_editor_executable_available`, `open_in_configured_editor_with_deps`)
  - `src-tauri/src/commands/file.rs#L529-L574` (`get_editor_config`, `set_editor_config`, novo comando `get_available_editors` e helper `detect_available_editors_with`)
  - `src-tauri/src/commands/file.rs#L654-L665` e `#L754-L816` (testes unitários Rust)
  - `src-tauri/src/commands/config.rs#L127-L157` (`run_git_config_unset` — reutilizar como `pub(crate)`, sem alterar assinatura)
  - `src-tauri/src/lib.rs#L958-L964` (`tauri::generate_handler![...]` — registrar `commands::file::get_available_editors`)
  - `src/stores/dialog.store.ts#L148-L170` (`SettingsDialogContext`, `DialogContexts`)
  - `src/services/git/ai-workspace.ts#L36-L48` e `#L90-L122` (`OpenResult`, novo `EditorInfo`, nova função `getAvailableEditors`)
  - `src/services/open-location.service.ts#L23-L80` (`reportFailure`, `openRepositoryInEditor`, novo helper compartilhado `handleOpenEditorFailure`)
  - `src/app-shell.ts#L5975-L5996` (`handleOpenFileFromPalette`)
  - `src/services/__tests__/file.service.test.ts#L19-L26` e `#L115-L188`
  - `src/services/__tests__/git-service-modules.test.ts#L364-L399` (`keyExports` de `ai-workspace.ts`)
  - `src/services/__tests__/network-gate-coverage.test.ts#L324` (`LOCAL_COMMANDS`)
  - `src/__tests__/app-shell-palette-open-location.test.ts#L134-L208`
  - `src/__tests__/app-shell-palette-open-file.test.ts#L128-L165`
- **Instruções Técnicas**:
  1. Em `src-tauri/src/commands/file.rs`, adicionar ao `OpenResult` (`#[serde(rename_all = "camelCase")]`) o campo `#[serde(default, skip_serializing_if = "Option::is_none")] pub not_configured: Option<bool>` e atualizar todas as construções existentes de `OpenResult` no arquivo com `not_configured: None`.
  2. Definir `#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)] #[serde(rename_all = "camelCase")] pub struct EditorInfo { pub id: String, pub display_name: String, pub command: String, pub available: bool }` e o comando `#[command] pub async fn get_available_editors() -> Result<Vec<EditorInfo>>`, registrando `commands::file::get_available_editors` em `src-tauri/src/lib.rs#L964`:
     - Montar catálogo de editores coerentes com o SO atual via `cfg!(target_os = ...)`: sempre incluir os editores multiplataforma sugeridos na issue — `VS Code` (`code`), `Cursor` (`cursor`), `Zed` (`zed`), `Sublime Text` (`subl`) — acrescidos apenas de editores nativos coerentes com cada SO (ex.: `Notepad++` / `notepad++` no Windows; `TextMate` / `mate` no macOS; `Kate` / `kate` e `Gedit` / `gedit` no Linux).
     - Extrair helper testável `detect_available_editors_with(probe: impl Fn(&str) -> bool) -> Vec<EditorInfo>` e helper `resolve_editor_executable(executable: &str) -> Option<String>` (com `is_editor_executable_available(executable: &str) -> bool` delegando para `resolve_editor_executable(executable).is_some()`):
       - Para caminhos com separador (`/` ou `\`) ou absolutos, retornar o caminho existente (checando também extensões `.exe`/`.com`/`.cmd`/`.bat` no Windows quando omitida).
       - Para comandos simples no `PATH`, usar `create_command("which").arg(executable)` em Unix/macOS (retornando a primeira linha não vazia) ou `create_command("where").arg(executable)` no Windows (selecionando a primeira linha existente com extensão `.exe`/`.com`/`.cmd`/`.bat`, priorizando `.exe`/`.com` sobre `.cmd`/`.bat` e ignorando o shim POSIX sem extensão, seguindo o padrão de `merge_tool.rs#L165-L177` e `difftool.rs#L81-L95`).
  3. Em `open_in_configured_editor` (`src-tauri/src/commands/file.rs#L269-L355`):
     - Filtrar valores vazios em `GIT_EDITOR`, `VISUAL` e `EDITOR` (`.filter(|v| !v.trim().is_empty())`) e extrair o núcleo para `open_in_configured_editor_with_deps(..., env_var: impl Fn(&str) -> Option<String>, resolve_executable: impl Fn(&str) -> Option<String>)` para testes determinísticos sem depender do ambiente de CI.
     - No braço `None if target.is_dir()`, remover o fallback `open_in_default_app(target_file)` e retornar `Ok(OpenResult { success: false, message: Some("No editor is configured (e.g., VS Code). Configure an editor in Settings.".to_string()), notice: None, not_configured: Some(true) })`. No braço `None` para arquivos (`no_editor_configured_for_file`), retornar também `not_configured: Some(true)` preservando o trecho `"Configure a GUI editor"` exigido pelo teste existente `#L655`.
     - No braço `Some(editor_cmd)` (editores GUI), após `parse_editor_command(&editor_cmd, &target_file, line)`, validar `let Some(resolved_cmd) = resolve_executable(&cmd) else { return Err(GitdudaError::OperationFailed(format!("Configured editor executable '{}' could not be found. Verify it is installed and on your PATH or update Settings → External Tools.", cmd))); };` **antes** do `spawn` (preservando compatibilidade com `test_open_in_configured_editor_accepts_repo_root` em `#L755` e garantindo que nunca caia no explorador de arquivos). Executar `std::process::Command::new(&resolved_cmd)` com `.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())` para desacoplar os handles de I/O do processo filho e aplicar `creation_flags(0x08000000)` (`CREATE_NO_WINDOW`) sob `#[cfg(target_os = "windows")]` para evitar flash de console ao iniciar shims `.cmd`/`.bat`. Manter o braço de editor de terminal (`is_terminal_editor`) intacto.
  4. Em `get_editor_config` e `set_editor_config` (`file.rs#L531-L574`):
     - Extrair helpers internos com `global_config_override: Option<&Path>` (usando `["--file", cfg_path]` nos testes para contornar `GIT_CONFIG_GLOBAL=/dev/null` em `src-tauri/src/utils/command.rs#L624`).
     - Garantir que `global == true` funcione com `path` vazio (sem validar nem usar `repo_path`).
     - Quando `editor.trim().is_empty()` em `set_editor_config`, executar `super::config::run_git_config_unset` no escopo correspondente (`--global` ou `--local`) para nunca gravar string vazia em `core.editor`.
  5. Em `src/services/git/ai-workspace.ts`:
     - Espelhar `notConfigured?: boolean;` em `OpenResult` (`#L37-L42`).
     - Exportar `export interface EditorInfo { id: string; displayName: string; command: string; available: boolean; }` e `export async function getAvailableEditors(): Promise<CommandResult<EditorInfo[]>>` invocando `"get_available_editors"`.
     - Adicionar `"getAvailableEditors"` em `keyExports` de `src/services/__tests__/git-service-modules.test.ts#L365-L393`, adicionar `'get_available_editors'` ao `Set` `LOCAL_COMMANDS` em `src/services/__tests__/network-gate-coverage.test.ts#L324`, e adicionar testes unitários em `src/services/__tests__/file.service.test.ts`.
  6. Em `src/stores/dialog.store.ts#L148-L170`, declarar `export interface SettingsDialogContext { focusSection?: 'external-tools'; }` e registrar `settings: SettingsDialogContext;` em `DialogContexts` (preservando compatibilidade com `dialogs.open('settings')` sem segundo argumento), garantindo que a chamada em `open-location.service.ts` compile desde a Etapa 1.
  7. Em `src/services/open-location.service.ts#L67-L80`, extrair `export function handleOpenEditorFailure(result: CommandResult<OpenResult>, fallbackMessage: string): false`:
     - Se `result.data?.notConfigured === true`, exibir `showToast(text, 'warning', 8000, { label: 'Configure Editor', callback: () => { dialogs.open('settings', { focusSection: 'external-tools' }); } })` (importando `dialogs` de `../stores/dialog.store.ts`) e retornar `false`.
     - Caso contrário, chamar `reportFailure(fallbackMessage, result.data?.message ?? result.error?.message)` (toast `error`).
     - Reutilizar `handleOpenEditorFailure` em `openRepositoryInEditor` (`'Failed to open editor'`) e em `handleOpenFileFromPalette` (`src/app-shell.ts#L5988-L5995`, `'Failed to open file in editor'`), mantendo o toast `info` para `result.data?.notice`.
  8. Expandir os testes unitários Rust em `file.rs` (diretório sem editor retornando `not_configured: Some(true)` sem abrir default app; executável indisponível retornando `OperationFailed` claro antes do spawn; comportamento de `resolve_editor_executable` incluindo shims `.cmd`/`.bat` no Windows; `detect_available_editors_with` marcando disponibilidade; `get_editor_config`/`set_editor_config` globais com `--file` e sem repo) e TS em `app-shell-palette-open-location.test.ts` e `app-shell-palette-open-file.test.ts`.
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 3` (altera contrato compartilhado Rust ↔ TS e registra novo comando IPC: rodar `cargo test --manifest-path src-tauri/Cargo.toml commands::file::tests`, `npm run test:contract`, `npm run typecheck` e testes unitários focais).
- **Critérios de Aceitação (Code Review)**:
  - `get_available_editors` está registrado em `src-tauri/src/lib.rs` e passa no contrato `scripts/ipc-contract.test.mjs`.
  - `open_in_configured_editor` em diretório sem editor configurado retorna `success: false` e `notConfigured: true` sem chamar `open_in_default_app`.
  - Executável configurado inexistente é rejeitado antes do `spawn` com mensagem clara contendo o nome do executável.
  - `openRepositoryInEditor` e `handleOpenFileFromPalette` exibem toast `warning` com ação `"Configure Editor"` apenas quando `notConfigured === true`, mantendo toast `error` para falhas de executável/spawn.
  - Nenhum campo `snake_case` em tipos ou chamadas TypeScript.

---

### Etapa 2: Navegação Direta para Settings (`app-shell.ts` / `focus.ts`), Campo Editor em `gn-settings-dialog.ts` e Sincronização i18n
- **Objetivo**: Repassar o contexto `{ focusSection?: 'external-tools' }` no `app-shell.ts` ao abrir `gn-settings-dialog` diretamente na seção **External Tools → Editor** com rolagem e foco preservado dentro do `<gn-modal>`, expor o seletor de Editor global (disponível com ou sem repositório aberto) listando os editores detectados no SO (com fallback para presets coerentes com o SO se nenhum for detectado) + `Custom...` (sem opção `None`), preservando comandos customizados após reiniciar e mantendo a cobertura i18n (`fr.xlf` + bundle) 100% íntegra.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src/app-shell.ts#L6455-L6468` (renderização de `<gn-settings-dialog>`)
  - `src/utils/focus.ts#L27-L31` (`containsDeepActiveElement`)
  - `src/components/dialogs/gn-settings-dialog.ts#L1-L36` (imports `property`, `EditorInfo`), `#L531-L606` (states, `updated`, `syncSelectValue`), `#L868-L905` (`loadExternalToolsConfig`), `#L1973-L2056` (seção *External Tools*)
  - `src/i18n/xliff/fr.xlf#L193-L237` (novas traduções e atualização da frase de repositório em External Tools)
  - `src/i18n/generated/locales/fr.ts` (gerado por `npm run localize:build`)
  - `src/stores/__tests__/dialog.store.test.ts`
  - `src/utils/__tests__/focus.test.ts#L44-L80`
  - `src/components/dialogs/__tests__/gn-settings-dialog-external-tools.test.ts#L88-L130` e novos testes de Editor
- **Instruções Técnicas**:
  1. Em `src/app-shell.ts#L6462-L6465`, passar `.focusSection=${dialogs.context('settings')?.focusSection ?? null}` para `<gn-settings-dialog>` (consumindo o `SettingsDialogContext` já registrado na Etapa 1).
  2. Em `src/utils/focus.ts#L27-L31`, atualizar `containsDeepActiveElement(host)` para subir pelos ancestrais atravessando `ShadowRoot` (`getRootNode()` → `root instanceof ShadowRoot ? root.host : null`) até confirmar se `host.contains(node) || (host.shadowRoot?.contains(node) ?? false)`. Isso impede que `gn-modal.ts#L225-L244` roube o foco para o botão `×` quando `<gn-settings-dialog>` (slotted) focar `#external-editor-select`.
  3. Em `src/components/dialogs/gn-settings-dialog.ts`:
     - Importar `property` em `#L2` e `EditorInfo` em `#L35`; declarar `@property({ type: String }) focusSection: 'external-tools' | null = null;`.
     - Adicionar estados `@state() private availableEditors: EditorInfo[] = []`, `@state() private editorSelectValue = ''`, `@state() private editorCustomCmd = ''`, `@state() private loadingEditor = false`, além de `private editorWriteToken = 0` e `private pendingEditorFocus = false`.
     - Reestruturar `loadExternalToolsConfig()` (`#L868-L905`) para carregar o Editor global **antes** do `if (!repo) return;` de Merge/Diff tools:
       - Executar `this.loadingEditor = true` (com `finally { this.loadingEditor = false; }`) e chamar em paralelo `gitService.getEditorConfig(repo?.repository.path ?? '', true)` e `gitService.getAvailableEditors()`.
       - Guardar as atribuições contra mocks legados que retornam `null`: `if (editorsResult.success && Array.isArray(editorsResult.data))` e `if (editorConfig.success && editorConfig.data)`.
       - Derivar `getDisplayedEditors(): EditorInfo[]`: se houver editores com `available === true`, retornar apenas esses; caso contrário (nenhum detectado), retornar todos os editores coerentes com o SO vindos de `availableEditors` como fallback.
       - Sincronizar estado inicial: se `editorConfig.data?.editor` existir (trim não vazio) e corresponder ao `command` de um item em `getDisplayedEditors()`, definir `editorSelectValue = editor` e `editorCustomCmd = ''`; se for outro valor não vazio, definir `editorSelectValue = '__custom__'` e `editorCustomCmd = editor` (preservando comandos personalizados após reiniciar o app); se ausente, manter `editorSelectValue = ''`.
     - Na seção **External Tools** (`#L1973-L2056`, adicionando `id="external-tools-section"`):
       - Renderizar a linha **Editor** (`id="external-editor-select"`, `?disabled=${this.loadingEditor}`) **fora** da condicional `repositoryStore.getState().getActiveRepository()` para permanecer acessível com ou sem repositório aberto.
       - **Sem opção `None`**: renderizar `<option value="" disabled>${msg('Select an editor...')}</option>` apenas como placeholder não selecionável quando `editorSelectValue === ''`, seguido pelas opções de `getDisplayedEditors()` (`${editor.displayName} (${editor.command})`) e `<option value="__custom__">${msg('Custom...')}</option>`.
       - Quando `editorSelectValue === '__custom__'`, renderizar a linha **Editor Command** com `<input id="external-editor-custom-input" type="text" .value=${this.editorCustomCmd} @change=${this.handleEditorCustomCmdChange} placeholder=${msg('e.g., code --wait')} />`.
       - Atualizar a mensagem do ramo `else` em `#L2051` para `msg('Open a repository to configure merge and diff tools')`.
     - Implementar `handleEditorSelectChange` e `handleEditorCustomCmdChange`:
       - Ao selecionar `'__custom__'`, apenas alternar `editorSelectValue = '__custom__'` preservando `this.editorCustomCmd` quando já houver um comando customizado carregado/salvo (limpando apenas se `editorCustomCmd` não pertencer a uma configuração customizada prévia) sem sobrescrever `core.editor` ainda, e focar `#external-editor-custom-input` após `updateComplete`.
       - Em `handleEditorCustomCmdChange`, ler `const trimmed = input.value.trim();` **antes** de mutar `this.editorCustomCmd`; se `!trimmed`, restaurar `input.value = this.editorCustomCmd;` e retornar sem chamar `setEditorConfig`.
       - Ao selecionar um editor conhecido ou confirmar comando customizado válido, chamar `gitService.setEditorConfig(repo?.repository.path ?? '', cmd, true)` com proteção de `editorWriteToken`, rollback + toast `error` se `!result.success || !result.data?.success`, e `window.dispatchEvent(new CustomEvent('settings-changed'))` em sucesso.
     - Em `updated(changedProperties: PropertyValues)`:
       - Re-sincronizar `#external-editor-select` via `this.syncSelectValue('#external-editor-select', this.editorSelectValue)`.
       - Ativar `this.pendingEditorFocus = true` quando `this.focusSection === 'external-tools'` na conexão/alteração da prop e, quando `this.pendingEditorFocus && !this.loadingEditor`, executar `scrollIntoView({ block: 'center' })` em `#external-tools-section`, focar `#external-editor-select` se habilitado e limpar `this.pendingEditorFocus = false`.
  4. **Sincronização obrigatória de i18n**:
     - Rodar `npm run localize:extract`, adicionar as traduções francesas em `src/i18n/xliff/fr.xlf` para cada nova string `msg(...)` adicionada (e remover a entrada órfã de `Open a repository to configure external tools` caso substituída) e rodar `npm run localize:build` para regenerar `src/i18n/generated/locales/fr.ts`.
  5. Adicionar/atualizar testes unitários em `src/stores/__tests__/dialog.store.test.ts`, `src/utils/__tests__/focus.test.ts` (foco em shadow root de filho slotted) e `src/components/dialogs/__tests__/gn-settings-dialog-external-tools.test.ts` (mocks de `get_available_editors`/`get_editor_config`/`set_editor_config`, renderização sem repositório aberto, tolerância a retorno `null` em mocks legados, filtro de editores detectados + fallback quando nenhum detectado, ausência de opção `None`, seleção de editor, comando `Custom...` preservado ao recarregar, restauração ao submeter comando customizado vazio, rollback em falha e foco automático com `focusSection === 'external-tools'`).
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 2` (`npm run test:contract`, `npm run lint`, `npm run typecheck` e testes unitários focais de `dialog.store.test.ts`, `focus.test.ts` e `gn-settings-dialog-external-tools.test.ts`).
- **Critérios de Aceitação (Code Review)**:
  - Campo **Editor** aparece em *External Tools* com ou sem repositório aberto, exibe apenas editores detectados (ou fallback coerente com o SO se nenhum detectado) + `Custom...`, e **não** contém opção `None`.
  - Comando customizado salvo globalmente volta selecionado em `Custom...` com o texto preservado ao reabrir Settings.
  - `dialogs.open('settings', { focusSection: 'external-tools' })` rola até `#external-tools-section` e mantém o foco em `#external-editor-select` dentro do `<gn-modal>`.
  - `npm run test:contract` passa sem falhas em `i18n-coverage.test.mjs`.

---

### Etapa 3: Atalho Global `Ctrl+Shift+A` (com Guard contra `e.repeat`), Badge na Paleta de Comandos e Wiring no `app-shell.ts`
- **Objetivo**: Registrar o atalho configurável `'open-in-editor'` (`Ctrl+Shift+A` / `Cmd+Shift+A`) em `keyboard.service.ts` ignorando `e.repeat` (inclusive após rebind), conectá-lo via `requiresRepository` em `app-shell.ts` e exibir o badge de atalho no item `"Open in Editor"` da Command Palette.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src/services/keyboard.service.ts#L8-L18` (`Shortcut`), `#L70-L72` (`shortcutMetadata`), `#L169-L200` (`register`), `#L225-L265` (`rebind`), `#L270-L320` (`resetBinding`, `resetAllBindings`), `#L581-L587` (`handleKeyDown`), `#L707-L736` e `#L851-L870` (`registerDefaultShortcuts`)
  - `src/app-shell.ts#L2190-L2223` (`registerDefaultShortcuts`)
  - `src/palette-commands.ts#L892-L900` (`open-in-editor`)
  - `src/services/__tests__/keyboard.service.test.ts`
  - `src/__tests__/app-shell-palette-open-location.test.ts`
- **Instruções Técnicas**:
  1. Em `src/services/keyboard.service.ts`:
     - Adicionar `noRepeat?: boolean;` em `Shortcut` (`#L8-L17`) e incluir `noRepeat?: boolean` em `shortcutMetadata` (`#L71`, `#L181-L185`), propagando-o em `register`, `rebind`, `resetBinding` e `resetAllBindings` para que rebinds feitos no Diálogo de Atalhos mantenham a proteção contra repetição.
     - Em `handleKeyDown` (`#L583-L587`), manter `e.preventDefault(); e.stopPropagation();` quando o atalho casar e executar `shortcut.action()` somente se `!(e.repeat && shortcut.noRepeat)`.
     - Adicionar `openInEditor?: () => void;` nos parâmetros de `registerDefaultShortcuts` (`#L707-L736`) e, quando fornecido, registrar `'open-in-editor'` na categoria `'General'` com `{ key: 'a', ctrl: true, shift: true, noRepeat: true, action: actions.openInEditor, description: 'Open repository in configured editor', category: 'General' }`.
  2. Em `src/app-shell.ts#L2190-L2222`, passar `openInEditor: this.requiresRepository(() => { void openRepositoryInEditor(this.activeRepository!.repository.path); })` (importando `openRepositoryInEditor` de `./services/open-location.service.ts` se ainda não importado no arquivo).
  3. Em `src/palette-commands.ts#L892-L900`, adicionar `shortcut: `${mod}⇧A`` no comando `'open-in-editor'` (seguindo o padrão de `create-branch` em `#L187`).
  4. Adicionar testes unitários em `src/services/__tests__/keyboard.service.test.ts` (registro em `Ctrl+Shift+A` / `Cmd+Shift+A` na categoria `General`, bloqueio de `e.repeat === true` antes e após `rebind`, funcionamento com foco em `<input>` por não colidir com `mod+a`, e respeito à regra do `gn-terminal-panel`) e em `src/__tests__/app-shell-palette-open-location.test.ts` (disparo via `Ctrl+Shift+A` no `app-shell`, aviso sem repositório aberto e bloqueio de repetição).
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 2` (`npm run lint`, `npm run typecheck`, `keyboard.service.test.ts` e `app-shell-palette-open-location.test.ts`).
- **Critérios de Aceitação (Code Review)**:
  - `Ctrl+Shift+A` (Windows/Linux) e `Cmd+Shift+A` (macOS) abrem o repositório ativo no editor configurado.
  - Segurar as teclas (`e.repeat === true`) executa a ação apenas uma vez, inclusive após rebind no Diálogo de Atalhos.
  - O atalho aparece no Diálogo de Atalhos (`General`) e como badge em `"Open in Editor"` na Command Palette.
  - Sem repositório aberto, exibe o aviso `"Please open a repository first"` sem chamar o backend; com foco em `gn-terminal-panel`, respeita o terminal.

---

### Etapa 4: Mocks E2E, Testes Playwright e Validação Global Final
- **Objetivo**: Garantir mocks padrão para os comandos de editor no harness E2E, cobrir os fluxos de ponta a ponta com Playwright e executar toda a bateria obrigatória de pré-commit (TS + Rust + contratos + E2E).
- **Arquivos, Linhas e Símbolos Afetados**:
  - `e2e/fixtures/tauri-mock.ts#L786-L1352` (`commandHandler` em `setupTauriMocks`) e `#L207-L741` (`createMockHandler`)
  - `e2e/tests/keyboard.spec.ts`
  - `e2e/tests/settings-external-tools.spec.ts`
- **Instruções Técnicas**:
  1. Em `e2e/fixtures/tauri-mock.ts`, adicionar os handlers padrão (`get_available_editors`, `get_editor_config`, `set_editor_config`, `open_in_configured_editor`) ao `commandHandler` dentro de `setupTauriMocks` (`#L786-L1352`), espelhando também em `createMockHandler` (`#L207-L741`) para manter paridade do arquivo (sem refatorar o harness de mock), para que qualquer spec que abra Settings receba respostas coerentes.
  2. Em `e2e/tests/keyboard.spec.ts`, adicionar testes E2E para:
     - `Ctrl+Shift+A` / `Meta+Shift+A` com repositório aberto invocando `open_in_configured_editor` com o caminho do repositório.
     - Quando `open_in_configured_editor` retornar `{ success: false, notConfigured: true, message: ... }`, exibir toast `warning` com botão `"Configure Editor"` que, ao ser clicado, abre `gn-settings-dialog` com `#external-editor-select` focado.
     - Sem repositório aberto, exibir aviso orientando a abrir um repositório sem invocar o backend.
  3. Em `e2e/tests/settings-external-tools.spec.ts`, adicionar testes E2E para:
     - Exibição dos editores detectados disponíveis em **External Tools → Editor** (sem opção `None`), seleção de editor conhecido (`code`) enviando `set_editor_config` com `global: true`.
     - Seleção de `Custom...` com comando personalizado sendo salvo globalmente e preservado ao reabrir Settings (inclusive sem repositório aberto).
     - Erro no salvamento revertendo o controle e exibindo toast de erro.
  4. Executar a bateria global final:
     - `npm run lint`
     - `npm run typecheck`
     - `npm test` (inclui `test:contract` — IPC + i18n + menu + security — e toda a suíte `web-test-runner`)
     - `cd src-tauri && cargo fmt --check && cargo clippy && cargo test`
     - `npx playwright test --config=e2e/playwright.config.ts e2e/tests/keyboard.spec.ts e2e/tests/settings-external-tools.spec.ts`
     - Verificação de ausência de chaves `snake_case` em tipos/chamadas Tauri TS.
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 3` (check global completo JS + Rust + Contratos + E2E).
- **Critérios de Aceitação (Code Review)**:
  - Todos os testes unitários, contratos (`ipc-contract`, `i18n-coverage`), E2E focais, `eslint`, `tsc`, `cargo fmt --check`, `cargo clippy` e `cargo test` passam sem erros ou regressões.
  - Nenhuma alteração fora do escopo (menu nativo intacto, sem `snake_case` em payloads TS, sem opção `None` no seletor de editor).
