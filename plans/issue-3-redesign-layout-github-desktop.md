# Plano de Desenvolvimento: Reestruturação do Frontend para Workspace 2-Painéis (Estilo GitHub Desktop)

- **Data**: 2026-11-18
- **Status**: [ ] Planejado
- **Debate Técnico**: Validado e revisado
- **Arquivo de Controle**: `plans/issue-3-redesign-layout-github-desktop.md`
- **Issue**: `#3` ([Issue #3](https://github.com/rdias612/gitduda/issues/3))
- **Spec de Origem**: `spec/redesign-layout-github-desktop.md`

---

## 1. Visão Geral e Regras Locais Aplicáveis
- **Objetivo**: Reestruturar o frontend do Gitduda do layout atual de 3 colunas (`left-panel` + `center-panel` + `right-panel` + `<gn-context-dashboard>`) para o workspace de **2 painéis** definido na spec `spec/redesign-layout-github-desktop.md`: Faixa Superior Compacta + Triple Action Bar + Sidebar Esquerda com 3 abas (`Changes`, `History`, `Branches & Refs`) + `main-pane` contextual (com expansão sob demanda para `<gn-graph-canvas>`) + Status Bar com Bottom Drawer unificado (`Terminal`, `Git Output`, `AI Review`, `Analytics`).
- **Restrições do Projeto e Invariantes**:
  - **Preservação de Tokens**: manter intacta a paleta de cores e temas em `src/styles/tokens.css`; novos estilos de componentes Lit devem incluir `sharedStyles` como primeiro item de `static styles`.
  - **Menus Nativos e Contrato IPC/Menu**: manter `src-tauri/src/menu.rs` e `APP_MENU_ACTIONS` em `src/services/app-menu.service.ts` alinhados (validado por `scripts/menu-contract.test.mjs`), sem janela *frameless* nem duplicação de menus HTML.
  - **Pilha de Overlays (`overlay-stack.ts`)**: todo novo popover/overlay com listener de teclado global ou fechamento por `Escape` deve cumprir a regra de ESLint `gitduda/overlay-stack-membership` (`eslint-rules/overlay-stack-membership.mjs`).
  - **Sem código de demonstração do mockup**: não portar a `.eval-bar` presente em `mockups/github-desktop-ux/opcao_2_dashboard_v2.html`.
- **Comando de Build Global**: `npm run typecheck && npm run build` (e validação de contratos via `npm run test:contract`).

## 2. Checklist de Execução de Etapas
- [ ] Etapa 1: Contratos de Estado (`uiStore`, `dialogStore`) e Ações de Painéis/Menu
- [ ] Etapa 2: Faixa Superior Compacta, Triple Action Bar e Popovers Unificados
- [ ] Etapa 3: Sidebar Esquerda Unificada com 3 Abas (`Changes`, `History`, `Branches & Refs`)
- [ ] Etapa 4: `main-pane` Contextual (Diff, Clean State + Pulse Cards, Commit Details / Full Graph, Ref Inspector)
- [ ] Etapa 5: Bottom Drawer Unificado, Status Bar e Suite de Testes

---

## 3. Especificação das Etapas

### Etapa 1: Contratos de Estado (`uiStore`, `dialogStore`) e Ações de Painéis/Menu
- **Objetivo**: Estender `uiStore` de forma estritamente aditiva para suportar as 3 abas da sidebar, expansão do grafo completo na aba `History`, largura da sidebar e estado do Bottom Drawer, preservando os contratos atuais até a migração nas Etapas 4–5.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src/stores/ui.store.ts#L3-L95` (`PanelId`, `UIState`, `defaultPanelState`, `uiStore`)
  - `src/stores/__tests__/ui.store.test.ts` (cobertura das novas propriedades e actions)
- **Instruções Técnicas**:
  - Adicionar em `UIState` (`src/stores/ui.store.ts`) de modo **100% aditivo**: `activeSidebarTab: 'changes' | 'history' | 'refs'`, `historyGraphExpanded: boolean`, `bottomDrawer: { isOpen: boolean; activeTab: 'terminal' | 'output' | 'review' | 'analytics'; height: number }`, `sidebarWidth: number` e as respectivas actions (`setSidebarTab`, `setHistoryGraphExpanded`, `openBottomDrawer`, `closeBottomDrawer`, `toggleBottomDrawerTab`, `setBottomDrawerHeight`, `setSidebarWidth`).
  - **Não** alterar nem remover `panels.right`, `DIALOG_REGISTRY.outputPanel`/`terminalPanel` (`src/stores/dialog.store.ts#L86-L92`), `toggle-output-panel` ou `toggle-right-panel` (`src/palette-commands.ts#L143-L145,L580-L585` e `src/services/app-menu.service.ts#L91-L107`) nesta etapa, garantindo build e testes 100% verdes entre Etapas 1→4.
  - Atualizar `src/stores/__tests__/ui.store.test.ts` cobrindo os novos estados e ações.
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 3: npm run typecheck && npm run test:contract && npx web-test-runner src/stores/__tests__/ui.store.test.ts`
- **Critérios de Aceitação (Code Review)**:
  - `uiStore` expõe ações tipadas para sidebar tab, expansão de grafo, largura da sidebar e Bottom Drawer sem quebrar `panels.right` existente.
  - `npm run test:contract` (`scripts/menu-contract.test.mjs`) e `ui.store.test.ts` passam sem alterações no backend Rust.

### Etapa 2: Faixa Superior Compacta, Triple Action Bar e Popovers Unificados
- **Objetivo**: Reestruturar `<gn-toolbar>` e substituir `<gn-context-dashboard>` pelo **Profile Pill** no header e pela **Triple Action Bar** (`Current repository`, `Current branch / ref` Unified Ref Switcher e Split Remote Sync), preservando diálogos/busca da toolbar, o modo `<gn-welcome>`, menus nativos do SO e `overlay-stack.ts`.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src/components/toolbar/gn-toolbar.ts#L55-L1482` (`GnToolbar`, `render`, `renderRemoteActions`, `<gn-clone-dialog>`, `<gn-init-dialog>`, `<gn-search-bar>`, listeners `L494-L507`)
  - `src/components/toolbar/index.ts#L1-L5` (novos exports: `<gn-unified-ref-switcher>`, `<gn-profile-pill>`)
  - `src/components/dashboard/gn-context-dashboard.ts#L46-L60` (reaproveitamento de seletores/ações de perfil para o Profile Pill)
  - `src/services/keyboard.service.ts#L48-L80` (leitura dinâmica do atalho de `command-palette`)
  - `src/app-shell.ts#L5071-L5076,L6162-L6203` (`handleToggleSearch`, `render` — remoção da faixa fixa `<gn-context-dashboard>` e ligação de eventos da barra superior)
  - `src/components/toolbar/__tests__/gn-toolbar.test.ts`, `src/components/dashboard/__tests__/gn-context-dashboard*.test.ts`
- **Instruções Técnicas**:
  - Dividir `<gn-toolbar>` em dois níveis visuais dentro do mesmo cabeçalho (usando exclusivamente CSS tokens de `src/styles/tokens.css`):
    1. **Faixa Superior Compacta**: logo + abas rápidas dos repositórios abertos (com badge numérico de alterações ou indicador limpo, menu de contexto e fechar aba) + à direita: pill de PR/CI ativo (quando detectado para a branch), botões compactos **Search** (`Ctrl+F`, visível apenas com `this.activeRepo !== null`), **Command Palette** (exibindo o atalho real obtido via `keyboardService`), **Keyboard Shortcuts** (`?`), **Settings** e **Profile Pill** conectado ao `unifiedProfileStore` (com popover de resumo de perfil/OAuth e atalho para `profileManager`/providers).
    2. **Triple Action Bar (condicionada a `this.activeRepo !== null`, mantendo `<gn-welcome>` livre quando não há repositório aberto)**:
       - Segmento `Current repository` com popover de troca entre abas/recentes + ações Open/Clone/Init/Workspace.
       - Segmento `Current branch / ref` exibindo branch atual (ou pill `detached HEAD`), badge `↑N ↓N` e chip de contagem `tags • stashes`; ao clicar, abre `<gn-unified-ref-switcher>` com busca, strip `+ New...` (`Branch`, `Tag`, `Stash`) e abas `Branches`, `Tags`, `Stashes`, `PRs & Flow`.
       - Split button de Remote Sync com ação primária dinâmica (`Push` / `Pull` / `Fetch`) + seta dropdown (`Fetch`, `Pull`, `Push`, `Push tags`, `Force push with lease`, `Prune`) usando os handlers existentes (`runFetch`, `runPull`, `runPush`, `RefLockController`).
  - **Preservar** em `gn-toolbar.ts`: `<gn-clone-dialog>`, `<gn-init-dialog>`, `<gn-search-bar>` (`L1467-L1473`) e os listeners `focus-search`, `open-repository`, `clone-repository` e `init-repository` (`L494-L507`).
  - Registrar todos os popovers em `src/utils/overlay-stack.ts` (`pushOverlay`, `removeOverlay`, `isTopOverlay`) para fechamento seguro com `Escape`.
  - Atualizar `gn-toolbar.test.ts` e adaptar/substituir testes afetados de `gn-context-dashboard*.test.ts`.
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 3: npm run lint && npm run typecheck` e `npx web-test-runner src/components/toolbar/__tests__/*.test.ts`
- **Critérios de Aceitação (Code Review)**:
  - `<gn-context-dashboard>` não ocupa mais faixa dedicada abaixo da toolbar; seus controles principais estão acessíveis via Profile Pill.
  - `<gn-welcome>` não exibe a Triple Action Bar; `Ctrl+F` (`<gn-search-bar>`), Settings, Shortcuts e menus nativos `File > Open/Clone/Init` continuam operacionais.
  - Nenhum erro de ESLint na regra `gitduda/overlay-stack-membership`.

### Etapa 3: Sidebar Esquerda Unificada com 3 Abas (`Changes`, `History`, `Branches & Refs`)
- **Objetivo**: Consolidar a navegação esquerda e o painel de staging/commit em uma única sidebar esquerda redimensionável com 3 abas segmentadas (`Changes`, `History`, `Branches & Refs`), desacoplar a hidratação de `commitsStore` de `<gn-graph-canvas>` e transferir imediatamente a posse única de `<gn-file-status>`/`<gn-commit-panel>` para `<gn-left-panel>`.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src/components/sidebar/gn-left-panel.ts#L16-L416` (`GnLeftPanel`, `render`, `toggleSection`)
  - `src/components/sidebar/gn-right-panel.ts#L211,L252-L268,L340-L355` (remoção imediata da montagem de `<gn-file-status>`/`<gn-commit-panel>` para evitar listeners duplicados até a Etapa 4)
  - `src/components/sidebar/gn-file-status.ts#L70,L656-L671,L1124-L1133,L1912-L1920,L3126-L3350` (`GnFileStatus`, filtros, `viewMode`, `handleStageAllShown`, `handleUnstageAllShown`)
  - `src/components/sidebar/gn-commit-panel.ts#L37,L862-L900,L1020-L1029` (`GnCommitPanel`, faixa IA, `conventionalMode`, `amend`, sign-off, co-authors, contador `0/72`)
  - `src/app-shell.ts#L1188-L1196,L2446-L2493,L4846-L4899` (`openBranchCleanup`, `containsOpenModal`, `dispatchAmend`, `dispatchToFileStatus`)
  - `src/stores/commits.store.ts#L15-L50` (`CommitsState`, `commitsStore`) e `src/components/graph/gn-graph-canvas.ts#L1336`
  - `src/components/sidebar/index.ts#L1-L7` (export de novo `<gn-history-list>`)
  - `src/components/sidebar/__tests__/gn-commit-panel-refresh-pin.test.ts`, `gn-right-panel-shortcuts.test.ts`
- **Instruções Técnicas**:
  - Em `gn-left-panel.ts`: sincronizar `activeSidebarTab` com `uiStore` e renderizar a barra de 3 abas segmentadas (`Changes`, `History`, `Branches & Refs`) com badges de contagem. Impedir troca de `activeSidebarTab` enquanto houver modal aberto na subárvore da aba atual (reutilizando `containsOpenModal` no padrão de `toggleLeftPanel()` em `app-shell.ts#L2515`), e portar o listener local `Ctrl+Shift+1/2/3` de `gn-right-panel.ts#L252-L268` para `<gn-left-panel>`.
  - **Dono único (`Changes`)**: ao montar `<gn-file-status>` e `<gn-commit-panel>` na aba `Changes` de `<gn-left-panel>`, remover imediatamente a aba `Changes` (montagem desses dois componentes) de `<gn-right-panel>` para evitar listeners duplicados em `window`, e atualizar `dispatchAmend()` e `dispatchToFileStatus()` (`app-shell.ts#L4846-L4899`) para garantir `leftPanelVisible` + `setSidebarTab('changes')` e consultar `gn-left-panel`.
  - Na aba **`Changes`**: ajustar `gn-file-status.ts` para filtro por status (`All`/`Staged`/`Unstaged`/`Added`/`Modified`), toggle `Flat`/`Tree`, checkbox mestre de `Stage`/`Unstage all` e menu de contexto; ajustar `gn-commit-panel.ts` para o layout compacto do **Commit Composer** (botões IA `AI Msg`, `Vibe Check`, `Hunk Split`, chips `Conventional`, `Amend`, `Sign off`, `Co-authors`, Summary `0/72`, Description e trailers).
  - Na aba **`History`** (*prefactoring* de `commitsStore`): desacoplar a hidratação de `commitsStore` de `<gn-graph-canvas>` carregando commits iniciais e refs por commit no `commitsStore` na ativação do repositório / inicialização de `<gn-history-list>`. Criar `src/components/sidebar/gn-history-list.ts` consumindo `commitsStore` com filtro (mensagem/SHA/tag/autor), mini-grafo SVG de lanes (`src/graph/lane-assignment.ts`), suporte a seleção única e **multi-seleção** (`Ctrl`/`Shift` + checkbox) e botão **"Expand Full Graph"** alternando `uiStore.setHistoryGraphExpanded(...)`.
  - Na aba **`Branches & Refs`**: adicionar filtro textual global, botões rápidos (`+ Branch`, `+ Tag`, `+ Stash`), pílulas de categoria (`All`, `Branches`, `Tags`, `Stashes`, `PRs`, `Git Flow`), garantir `setSidebarTab('refs')` em `openBranchCleanup()` (`app-shell.ts#L1188-L1196`) e emitir evento `ref-inspected` para o Ref Inspector.
  - Atualizar `gn-commit-panel-refresh-pin.test.ts` e migrar `gn-right-panel-shortcuts.test.ts` para o novo `<gn-left-panel>`.
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 3: npm run lint && npm run typecheck` e testes de unidade em `src/components/sidebar/__tests__/*.test.ts`.
- **Critérios de Aceitação (Code Review)**:
  - Não existem duas instâncias simultâneas de `<gn-file-status>` ou `<gn-commit-panel>` ouvindo `window`.
  - `commitsStore` é hidratado independentemente de `<gn-graph-canvas>` estar montado; multi-seleção em `History` e atalhos `Ctrl+Shift+1/2/3` funcionam em `<gn-left-panel>`.
  - Troca de aba é bloqueada com feedback se houver modal aberto na aba atual.

### Etapa 4: `main-pane` Contextual (Diff, Clean State + Pulse Cards, Commit Details / Full Graph, Ref Inspector)
- **Objetivo**: Remover a coluna fixa `<gn-right-panel>` e a divisão estática de 3 painéis em `src/app-shell.ts`, transformando a área principal em um `#main-pane` contextual com fallback completo em `commitsStore` quando `<gn-graph-canvas>` estiver desmontado e semântica correta de `Escape` no diff da aba `Changes`.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src/app-shell.ts#L355-L614,L886-L900,L1280-L1282,L2078-L2082,L2191-L2206,L2563-L2569,L4531-L4790,L6022-L6435` (`AppShell`, CSS layout, `handleWatcherEvent`, `registerDefaultShortcuts`, `handleCloseOverlay`, `handleResizeStart`/`Apply`, `loadedGraphCommits`, `syncGraphPaletteData`, `revealCommitInGraph`, `openWorkingTreeDiff`, `render`)
  - `src/palette-commands.ts#L117-L133,L580-L585` (`graph-jump-head`, redirecionamento de `toggle-right-panel`)
  - `src/components/panels/gn-diff-view.ts#L1041-L1090,L3081-L3230` (`GnDiffView`, cabeçalho, `Unified`/`Split`, Whitespace, `AI Explain`, `Open in Editor`, gear popover e banner EOL)
  - `src/components/panels/gn-commit-details.ts#L42-L100` (`GnCommitDetails`)
  - `src/components/panels/index.ts#L1-L8` (exports de `<gn-clean-state>` e `<gn-ref-inspector>`)
  - `src/components/sidebar/__tests__/gn-right-panel-analytics-active.test.ts`, `src/components/dialogs/__tests__/gn-lock-observation.test.ts`, `src/components/panels/__tests__/gn-commit-details.test.ts`, `src/stores/__tests__/ui.store.test.ts`
- **Instruções Técnicas**:
  - Remover `<aside class="right-panel">` (`gn-right-panel`), o splitter direito e o bloco `if (!this.rightPanelVisible)` em `handleWatcherEvent` (`src/app-shell.ts#L1280`, sem substituí-lo por `!this.leftPanelVisible`, pois `.left-panel.hidden` mantém `<gn-file-status>` montado). Redirecionar `toggle-right-panel` (`palette-commands.ts#L580-L585` e `app-shell.ts#L2206`) para alternar o inspetor/expansão do `#main-pane` mantendo `APP_MENU_ACTIONS` compatível com `menu-contract.test.mjs`.
  - **Desacoplamento de `<gn-graph-canvas>`**:
    - Em `loadedGraphCommits()` (`L4604`) e `syncGraphPaletteData()` (`L4705`), usar `this.graphCanvas?.getLoadedCommits() ?? commitsStore.getState().commits`.
    - Em `revealCommitInGraph(oid)` (`L6022`) e `graph-jump-head` (`palette-commands.ts#L117-L133`): quando `!this.graphCanvas`, alternar para `setSidebarTab('history')` e selecionar o commit alvo em `commitsStore`/`selectedCommit` sem toast falso de commit não carregado.
    - Em `navigateUp`..`PageDown` (`L2191-L2196`), delegar para `this.graphCanvas` quando `historyGraphExpanded === true` ou para `<gn-history-list>` no modo compacto.
  - **Semântica de `Escape` e Diff (`L2563-L2569,L4723-L4747`)**: manter `dialogs.open('diff')`, garantindo `setSidebarTab('changes')` em `openWorkingTreeDiff()`, e no ramo `Escape` de `handleCloseOverlay()` (`L2563`) só chamar `handleCloseDiff()` quando `uiStore.getState().activeSidebarTab !== 'changes' || this.diffCommitFile !== null` (evitando zerar `this.diffFile` e deixar a tela em `"No file selected"`).
  - Manter `.operation-banner` no topo de `#main-pane` e implementar o roteamento contextual:
    - Subviews inline (`dialogs.isOpen('blame')` / `fileHistory`) com cabeçalho de retorno.
    - `Changes`: `<gn-diff-view>` completo ou `<gn-clean-state>` ("No local changes" + atalhos + grid 2×2 de **Pulse Cards**) quando `stagedFiles.length === 0 && unstagedFiles.length === 0`.
    - `History`: `<gn-commit-details>` + lista de arquivos + `<gn-diff-view>` *read-only* (ou barra multi-commit quando >1 selecionado); quando `historyGraphExpanded === true`, renderizar `<gn-graph-canvas>` acoplado aos detalhes e botão para recolher.
    - `Branches & Refs`: `<gn-ref-inspector>` contextual com metadados, ações rápidas e diff/arquivos associados.
  - Atualizar os testes unitários afetados pela remoção de `gn-right-panel` (`gn-right-panel-analytics-active.test.ts`, `gn-lock-observation.test.ts`, `gn-commit-details.test.ts`, `ui.store.test.ts`).
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 3: npm run lint && npm run typecheck && npm run test:contract` + testes unitários dos painéis afetados.
- **Critérios de Aceitação (Code Review)**:
  - `#main-pane` alterna sem perda de estado entre `Changes`, `History` (compacto e `<gn-graph-canvas>` expandido) e `Branches & Refs`, mantendo `Escape`, Command Palette, atalhos de navegação e multi-seleção funcionais mesmo com o grafo recolhido.
  - Todos os estilos respeitam as variáveis de `src/styles/tokens.css` em light e dark mode.

### Etapa 5: Bottom Drawer Unificado, Status Bar e Suite de Testes (Unit + E2E)
- **Objetivo**: Criar `<gn-bottom-drawer>` colapsável/redimensionável com fluxo completo para `Terminal`, `Git Output`, `AI Review` e `Analytics`, atualizar `footer.status-bar`, migrar os chamadores restantes de `outputPanel`/`terminalPanel` e atualizar as suítes unitária e E2E.
- **Arquivos, Linhas e Símbolos Afetados**:
  - Novo `src/components/panels/gn-bottom-drawer.ts` (`GnBottomDrawer`) e export em `src/components/panels/index.ts#L1-L8`
  - `src/components/panels/gn-output-panel.ts#L52-L60,L341`, `gn-terminal-panel.ts#L31-L44`, `gn-analytics-panel.ts#L35-L46`
  - `src/components/dialogs/gn-ai-review-panel.ts#L18-L35,L524-L535` + `src/services/ai-review.service.ts#L156-L201`
  - `src/components/dialogs/gn-start-agent-dialog.ts#L207`
  - `src/stores/dialog.store.ts#L86-L92` (`DIALOG_REGISTRY` — remoção de `outputPanel` e `terminalPanel`)
  - `src/palette-commands.ts#L136-L155` (`toggle-output-panel`, `toggleTerminalPanel`)
  - `src/app-shell.ts#L614-L630,L1777-L1779,L2500-L2513,L6380-L6435` (`closeBottomDrawer`, `openNewTerminal`, `toggleTerminalPanel`, `status-bar`)
  - Testes unitários (`src/__tests__/app-shell*.test.ts`, `gn-start-agent-dialog.test.ts`, `gn-output-panel.test.ts`, `gn-terminal-panel.test.ts`) e E2E (`e2e/pages/app.page.ts#L50-L51,L101`, `e2e/pages/panels.page.ts#L225`, `e2e/tests/*.spec.ts`)
- **Instruções Técnicas**:
  - Criar `<gn-bottom-drawer>` com handle de resize vertical na borda superior e 4 abas (`Terminal`, `Git Output` com contador, `AI Review` com contador de achados, `Analytics`).
  - **Fluxo de `AI Review` no drawer**: manter estado leve por `repoPath` (`reviewResult`, `loading`, `error`) e renderizar cabeçalho compacto na aba `AI Review` com seletor de branch base e botão `"Run Branch Review"` chamando `reviewBranchDiff(repoPath, baseRef)` (`ai-review.service.ts#L156`), conectando `@apply-patch` (`applyCommentResolutionPatch` + refresh) e `@dismiss-*`.
  - **Migração única de `outputPanel`/`terminalPanel`**: migrar `toggle-output-panel` (`palette-commands.ts#L144`), `toggleTerminalPanel`/`openNewTerminal` (`app-shell.ts#L2501,L2510`) e os chamadores externos (`gn-start-agent-dialog.ts#L207`, `gn-output-panel.ts#L341`) para `uiStore.getState().openBottomDrawer('terminal' | 'output')`, remover `outputPanel` e `terminalPanel` de `DIALOG_REGISTRY` (`dialog.store.ts#L86-L92`) e chamar `uiStore.getState().closeBottomDrawer()` em `app-shell.ts#L1778` junto a `dialogs.closeRepoScoped()`.
  - Atualizar `footer.status-bar` (caminho do repo, watcher ativo, perfil, `HEAD`, `UTF-8 • LF`, `renderRemoteBadges()`, botões de toggle para as 4 abas do drawer).
  - Atualizar testes unitários (`app-shell*.test.ts`, `gn-start-agent-dialog.test.ts`, `gn-output-panel.test.ts`, `gn-terminal-panel.test.ts`) e aplicar estratégia **Page-Object-First** em `e2e/`: atualizar `e2e/pages/app.page.ts` (`waitForReady` aceitando `gn-welcome, gn-left-panel`, seletores de `#main-pane`) e `e2e/pages/panels.page.ts`, adicionar helper para abrir `History` + `Expand Full Graph` nas specs de grafo e atualizar specs E2E com seletores legados (`gn-context-dashboard` / `gn-right-panel`).
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 3: npm run lint && npm run typecheck && npm run test && npm run build`
- **Critérios de Aceitação (Code Review)**:
  - Sessões PTY em `<gn-terminal-panel>` sobrevivem ao colapsar/reabrir o Bottom Drawer ou alternar entre abas do drawer; nenhum chamador restante usa `dialogs.open('outputPanel' | 'terminalPanel')`.
  - `AI Review`, `Analytics`, `Git Output` e `Terminal` operam dentro do drawer com limpeza automática ao fechar o último repositório.
  - `npm run lint`, `npm run typecheck`, `npm run test` (contratos + `web-test-runner`) e `npm run build` concluem sem erros.
