# Spec: Atalho Global `Ctrl+Shift+A` para Abrir Editor Configurado e Configuração em Settings

- **Data**: 2026-11-18
- **Status**: Refinada (Pronta para `plan-designer`)
- **Arquivo**: `spec/atalho-abrir-editor.md`

## 1. O Desejo (Objetivo e Porquê)
Permitir que o usuário pressione `Ctrl+Shift+A` (ou `Cmd+Shift+A` no macOS) de qualquer ponto do aplicativo para abrir imediatamente a pasta do repositório ativo no editor configurado (por exemplo, VS Code via `git config --global core.editor "code"`). Caso nenhum editor esteja configurado, em vez de abrir silenciosamente o gerenciador de arquivos do sistema, o aplicativo deve avisar com um toast claro e oferecer um botão de atalho que leve direto à nova opção de **Editor** em **Settings -> External Tools**.

## 2. Comportamento Esperado e Decisões Fechadas
- **Decisões do Usuário (Fluxo Principal / Estrutura / Abordagem Escolhida)**:
  - **Alvo do atalho**: Sempre abre a **pasta do repositório ativo** no editor configurado, reutilizando o fluxo de `"Open in Editor"` (`openRepositoryInEditor`).
  - **Teclas e superfícies**: Registrar o atalho padrão `Ctrl+Shift+A` (`Cmd+Shift+A` no macOS) como atalho global (`keyboard.service.ts`), listá-lo como customizável no Diálogo de Atalhos (`gn-keyboard-shortcuts-dialog.ts`) e exibir o badge do atalho no comando `"Open in Editor"` da Command Palette (`src/palette-commands.ts`). Não adicionar novo item no menu nativo da aplicação (`app-menu.service.ts` / `menu.rs`).
  - **Proteção contra repetição contínua**: Ignorar eventos de tecla repetida (`e.repeat`) ao segurar `Ctrl+Shift+A`, evitando disparar múltiplas instâncias do editor.
  - **Sem editor configurado (fluxo unificado)**: Aplicar o mesmo comportamento tanto no atalho `Ctrl+Shift+A` quanto em `"Open in Editor"` na Command Palette e no menu de contexto da aba do repositório (`gn-toolbar.ts`). Quando nenhum editor estiver configurado, **não** abrir o Explorador/Finder padrão do SO — exibir um toast avisando que não há editor configurado (citando exemplo como VS Code) com um botão de ação para configurar.
  - **Configuração em Settings -> External Tools**: Ao clicar no botão do toast, abrir o `gn-settings-dialog` na seção **External Tools** (onde já ficam *Merge Tool* e *Diff Tool*), expondo um novo campo **Editor** (`core.editor`) com presets comuns (como VS Code `code`) e opção de comando customizado, salvando globalmente (`git config --global core.editor ...`).

- **Regras de Negócio e Casos de Borda (Erros, Estados Vazios, Limites)**:
  - **Nenhum repositório aberto (tela inicial / welcome)**: Como o atalho usa `requiresRepository(...)`, pressionar `Ctrl+Shift+A` exibe o toast de aviso padrão (`"Please open a repository first"`) sem invocar o backend.
  - **Foco dentro de campos de texto (`<input>`, `<textarea>`, `<select>`, `contentEditable`)**: `Ctrl+Shift+A` (`mod+shift+a`) deve continuar funcionando globalmente (não colide com `NATIVE_EDITING_KEYS`, que bloqueia apenas `mod+a`).
  - **Foco dentro do terminal embutido (`gn-terminal-panel`)**: Seguir a regra atual de `keyboard.service.ts`, que preserva combinações de `Ctrl` para o shell dentro do terminal.
  - **Nenhum editor configurado (`core.editor` local/global e `GIT_EDITOR`/`VISUAL`/`EDITOR` ausentes)**: O comando backend `open_in_configured_editor` (quando o alvo é um diretório ou arquivo) não deve cair para `open_in_default_app`; deve retornar indicação explícita de "editor não configurado" para o frontend exibir um toast `warning` com botão `"Configure Editor"` (que fecha/dispensa o toast e abre `gn-settings-dialog` na seção **External Tools**).
  - **Editor de terminal configurado (ex.: `vim`, `nano`) ao abrir diretório**: Manter o comportamento existente no backend (`open_in_default_app` + `notice` informativo), já que existe um editor configurado no ambiente.
  - **Falha ao iniciar o processo do editor configurado (ex.: comando inexistente ou erro de spawn)**: Exibir toast `error` com a mensagem retornada pelo backend (mantendo compatibilidade com os testes existentes de `open_in_configured_editor` que verificam `OpenResult.success === false`).
  - **Campo Editor em Settings sem repositório aberto**: Embora `Merge Tool` e `Diff Tool` hoje exijam repositório aberto em `gn-settings-dialog.ts`, o campo **Editor (Global)** opera em `core.editor` global e deve estar disponível na seção **External Tools** (mesmo se o usuário acessar Settings sem repo ou via toast com repo aberto).

- **Padrões Assumidos pelo Agente (Revise se discordar)**:
  - **ID do atalho e categoria**: Usar `id: 'open-in-editor'` (mesmo ID do comando na Command Palette) na categoria `'General'` em `registerDefaultShortcuts` (`src/services/keyboard.service.ts`), com descrição `'Open repository in configured editor'`.
  - **Badge dinâmico ou formatado na Command Palette**: Exibir o atalho atual de `open-in-editor` na Command Palette (`${mod}⇧A` no padrão de `src/palette-commands.ts`, ou lido de `keyboardService` se suportado).
  - **Contrato `OpenResult` / motivo explícito no backend**: Adicionar um campo opcional (ex.: `reason?: 'no_editor' | ...` ou `notConfigured?: boolean` em camelCase seguindo a convenção Tauri/TS do projeto) em `OpenResult` (`src-tauri/src/commands/file.rs` e `src/services/git/ai-workspace.ts`) para que `openRepositoryInEditor` saiba exatamente quando anexar o botão `"Configure Editor"` ao toast sem depender de *string matching* frágil, mantendo `success: false` e `message` preenchido.
  - **Presets do campo Editor em `gn-settings-dialog.ts`**: Seguir o mesmo padrão visual de `<select>` + `<input>` (`__custom__`) já usado em *Merge Tool* e *Diff Tool*, com opções como `None` (limpar `core.editor` global), `VS Code (code)`, `Cursor (cursor)`, `Zed (zed)`, `Sublime Text (subl)` e `Custom...`. Ao salvar `None`, remover a chave `core.editor` global (estendendo `unset_git_config` ou `set_editor_config` com string vazia para aceitar `global: true` sem exigir repositório aberto).
  - **Abertura direcionada do Settings Dialog**: Adicionar contexto opcional para `settings` em `DialogContexts` (`src/stores/dialog.store.ts`, ex.: `{ focusSection?: 'external-tools' }`, seguindo o mesmo padrão de `ProfileManagerContext`) para `gn-settings-dialog` rolar suavemente e focar o seletor de Editor ao ser aberto pelo botão do toast.

- **Pontos de Conexão no Projeto**:
  - `src/services/keyboard.service.ts` (`registerDefaultShortcuts`, `handleKeyDown` / guarda de `e.repeat` para a ação ou serviço)
  - `src/app-shell.ts` (ligação em `registerDefaultShortcuts` com `requiresRepository(() => ...)` e passagem de contexto ao `gn-settings-dialog`)
  - `src/palette-commands.ts` (badge de atalho no comando `'open-in-editor'`)
  - `src/services/open-location.service.ts` (`openRepositoryInEditor` exibindo toast com `ToastAction` quando não houver editor configurado)
  - `src/stores/dialog.store.ts` (`DialogContexts` para direcionar `settings` à seção `external-tools`)
  - `src/components/dialogs/gn-settings-dialog.ts` (novo campo **Editor** na seção *External Tools*, usando `getEditorConfig` / `setEditorConfig` de `src/services/git/ai-workspace.ts`)
  - `src-tauri/src/commands/file.rs` (`open_in_configured_editor`, `OpenResult`, `get_editor_config` / `set_editor_config`)

## 3. Rascunho Visual / Fluxo
```text
[Usuário aperta Ctrl+Shift+A]
   │  (ignora se e.repeat === true)
   ▼
[requiresRepository()] ──(sem repo)──▶ Toast warning: "Please open a repository first"
   │ (com repo ativo)
   ▼
[openRepositoryInEditor(repoPath)] ──▶ invoke("open_in_configured_editor", repoPath)
   ├── core.editor / env configurado ──▶ Abre pasta do repo no editor (ex.: VS Code)
   └── nenhum editor configurado     ──▶ Toast warning: "No editor configured (e.g. VS Code)."
                                            └─ Botão [Configure Editor]
                                                  └─▶ Abre Settings -> External Tools -> campo Editor (focado)
                                                        └─▶ Seleciona "VS Code (code)" ou Custom
                                                              └─▶ Salva git config --global core.editor "code"
```

## 4. Fora de Escopo e Restrições (Anti-Goals)
- **Fora de Escopo (O que NÃO fazer nesta entrega)**:
  - Abrir arquivos individuais selecionados no painel de Diff/Changes com `Ctrl+Shift+A` (o atalho abre sempre a pasta raiz do repositório ativo).
  - Adicionar novo item no menu nativo da aplicação (`src-tauri/src/menu.rs` / `src/services/app-menu.service.ts`).
  - Detecção automática de binários instalados no SO para popular a lista de editores (basta lista estática de presets comuns + opção `Custom...`).
- **Anti-Goals (O que evitar para não degradar a experiência/sistema)**:
  - Não abrir o gerenciador de arquivos padrão do SO silenciosamente quando `core.editor` não estiver configurado.
  - Não permitir que segurar `Ctrl+Shift+A` dispare dezenas de processos do editor em sequência (`e.repeat`).
  - Não quebrar os atalhos de terminal quando o foco estiver no `gn-terminal-panel`, nem bloquear `Ctrl+A` dentro de campos de texto.

## 5. Critérios de Aceite
- [ ] Pressionar `Ctrl+Shift+A` (Windows/Linux) ou `Cmd+Shift+A` (macOS) com um repositório aberto e `core.editor` configurado (ex.: `"code"`) abre a pasta do repositório ativo no editor configurado.
- [ ] Segurar a tecla (`e.repeat === true`) dispara a abertura apenas uma vez por pressionamento.
- [ ] O atalho `open-in-editor` aparece listado na categoria `General` do Diálogo de Atalhos (`gn-keyboard-shortcuts-dialog`), pode ser rebindado/resetado pelo usuário e aparece como dica de atalho no item `"Open in Editor"` da Command Palette.
- [ ] Pressionar `Ctrl+Shift+A` sem repositório aberto exibe o aviso `"Please open a repository first"` sem chamar o backend.
- [ ] Acionar `"Open in Editor"` (via `Ctrl+Shift+A`, Command Palette ou menu de contexto da aba) sem nenhum editor configurado (`core.editor` e variáveis de ambiente vazios) **não** abre o gerenciador de arquivos do SO e exibe um toast avisando que não há editor configurado com botão de ação para configurar.
- [ ] Clicar no botão do toast abre o `gn-settings-dialog` já posicionado/focado no novo campo **Editor** dentro de **External Tools**.
- [ ] Selecionar um preset (como `VS Code (code)`) ou informar um comando customizado no campo **Editor** salva a configuração global (`git config --global core.editor`), permitindo que o próximo `Ctrl+Shift+A` abra o repositório diretamente no editor escolhido.
