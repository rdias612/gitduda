# Spec: Reestruturação do Layout Frontend para Workspace 2-Painéis (Estilo GitHub Desktop)

- **Data**: 2026-11-18
- **Status**: Refinada (Pronta para `plan-designer`)
- **Arquivo**: `spec/redesign-layout-github-desktop.md`
- **Referência Visual / Estrutural**: [`mockups/github-desktop-ux/opcao_2_dashboard_v2.html`](../mockups/github-desktop-ux/opcao_2_dashboard_v2.html) *(apenas estrutura de layout e fluxos; paleta de cores/tokens atual preservada)*

---

## 1. O Desejo (Objetivo e Porquê)

Reestruturar a arquitetura de layout do frontend do Gitnado (atualmente baseada em 3 colunas com `<gn-graph-canvas>` central permanente, `<gn-context-dashboard>` separado e `<gn-right-panel>` fixo) para o modelo de **workspace em 2 painéis** definido no mockup `opcao_2_dashboard_v2.html`, priorizando o fluxo diário de *working tree* (Changes + Diff em tela cheia), histórico compacto com expansão sob demanda para o grafo completo, inspeção unificada de refs e ferramentas de desenvolvimento em gaveta inferior.

O objetivo é reduzir o ruído visual e a fragmentação entre painéis, preservando integralmente as funcionalidades já implementadas no Gitnado (staging por linha/hunk, IA, providers, diálogos avançados, multi-repo e paleta de temas existente).

---

## 2. Comportamento Esperado e Decisões Fechadas

### Decisões do Usuário (Estrutura e Fluxo)
- **Layout 2 Painéis + Triple Action Bar + Bottom Drawer**:
  - Substituir o layout de 3 colunas (`left-panel` + `center-panel` + `right-panel`) por uma **Sidebar Esquerda** redimensionável (3 abas: `Changes`, `History`, `Branches & Refs`) e um **Main Pane** contextual à direita, além de uma **Status Bar** com **Bottom Drawer** colapsável.
  - **Não** alterar a paleta de cores nem os design tokens atuais de `src/styles/tokens.css` (manter temas dark/light existentes do Gitnado).
- **Menus Nativos do SO Preservados + Faixa Superior na WebView**:
  - Manter a barra de título e os menus nativos do Tauri (`src/services/app-menu.service.ts`), sem janela *frameless* nem botões `─ □ ✕` HTML e sem duplicar o menu `File/Edit/View...` dentro da WebView.
  - A faixa superior da WebView passa a ter dois níveis:
    1. **Faixa Superior Compacta**: marca + abas rápidas dos repositórios abertos (com contador de alterações/estado limpo e fechamento de aba) à esquerda; pill de PR/CI ativo, botão de Command Palette e **Profile Pill** (conectado ao `unifiedProfileStore`, abrindo popover/atalho para Profile Manager e integrações) à direita.
    2. **Triple Action Bar**:
       - **Segmento 1 (`Current repository`)**: exibe o repositório ativo e abre popover/seletor para alternar entre repositórios abertos/recentes ou abrir/clonar/inicializar.
       - **Segmento 2 (`Current branch / ref` — Unified Ref Switcher)**: exibe a branch atual (ou badge de `detached HEAD`), pílula `↑ahead ↓behind` e chip resumo de tags/stashes. Ao clicar, abre popover com busca, criação rápida (`+ New...`: Branch / Tag / Stash) e abas (`Branches`, `Tags`, `Stashes`, `PRs & Flow`).
       - **Segmento 3 (`Remote Sync` Split Button)**: botão principal com operação contextual (`Push origin`, `Pull origin` ou `Fetch origin` + subtítulo de status) e seta lateral para menu completo (`Fetch`, `Pull`, `Push`, `Push tags`, `Force push with lease`, `Prune`).
- **Grafo Completo (`<gn-graph-canvas>`) + Aba `History` Híbrida**:
  - Na aba `History` da sidebar esquerda: lista compacta de commits com filtro (mensagem, SHA, tag, autor), mini-grafo de lanes e suporte a **multi-seleção** (checkbox / `Shift+Click`).
  - No `main-pane` (padrão ao selecionar commit): exibe cabeçalho de detalhes do commit (título, corpo, autor, data, SHA, badge GPG, ações `AI Explain Commit`, `Tag Here`, `Cherry-pick`, `Revert` ou barra de ações multi-commit quando >1 selecionado), lista de arquivos alterados e `<gn-diff-view>` *read-only*.
  - Botão/toggle **"Expand Full Graph"** na aba `History` (ou no cabeçalho do `main-pane`) alterna o `main-pane` para renderizar o `<gn-graph-canvas>` completo com painel de detalhes/diff acoplado, preservando todas as capacidades avançadas do grafo (zoom/pan, busca avançada, colunas extras, menus de contexto).
- **Preservação de Recursos Existentes no Novo Layout**:
  - **Banner de Operação em Andamento**: continua renderizado no topo do `main-pane` sempre que `repository.state !== 'clean' || hasConflictedFiles` (`Resolve Conflicts`, `Skip`, `Abort`, `Manage Bisect`).
  - **Analytics**: preservado como aba dentro do **Bottom Drawer** (junto com `Terminal`, `Git Output` e `AI Review`), acessível também pela Status Bar.
  - **Diálogos Avançados e Welcome**: `<gn-welcome>` (quando não há repositório aberto) e todos os ~50 diálogos em `src/components/dialogs/` permanecem funcionando com o fluxo atual via `dialogStore` (`DIALOG_REGISTRY`).

### Comportamento Detalhado das Abas e `main-pane`
1. **Aba `Changes` (Sidebar) $\rightarrow$ Diff / Clean State (`main-pane`)**:
   - **Sidebar**: barra de filtro (texto + popover por status: `All`, `Staged`, `Unstaged`, `Added`, `Modified`), alternador `Flat` / `Tree`, checkbox mestre de stage/unstage, lista de arquivos com menu de contexto e **Commit Composer** fixo na base (faixa IA `AI Msg`, `Vibe Check`, `Hunk Split`; chips `Conventional`, `Amend`, `Sign off`, `Co-authors`; Summary com contador `0/72`, Description, trailers e botão de Commit).
   - **`main-pane` com alterações**: exibe `<gn-diff-view>` completo (Unified / Split, toggle Whitespace, `AI Explain` inline, `Open in Editor`, popover de configurações de diff, gutter interativo de stage/unstage por linha e por hunk, banner de aviso EOL). Quando `Blame` ou `File History` são acionados, alternam inline no `main-pane` com botão para voltar ao diff.
   - **`main-pane` sem alterações (`files.length === 0`)**: exibe **Clean State** ("No local changes") com atalhos para Editor externo, Explorer/Finder e repositório remoto + **Grid 2×2 de Pulse Cards** (`Last Commit` $\rightarrow$ aba History; `Remote Sync` $\rightarrow$ Fetch/Sync; `Tags & Stashes` $\rightarrow$ aba Refs; `Repo Health & Shell` $\rightarrow$ abre Terminal no Bottom Drawer).
2. **Aba `Branches & Refs` (Sidebar) $\rightarrow$ Ref Inspector (`main-pane`)**:
   - **Sidebar**: filtro textual, botões de criação rápida (`+ Branch`, `+ Tag`, `+ Stash`), pílulas de categoria (`All`, `Branches`, `Tags`, `Stashes`, `PRs`, `Git Flow`) e seções expansíveis.
   - **`main-pane`**: **Ref Inspector** contextual exibindo metadados, ações rápidas da ref selecionada (checkout, merge, rebase, compare, pop/apply stash, push/delete tag, abrir PR) e preview dos arquivos/diff relacionados.
3. **Status Bar & Bottom Drawer**:
   - **Status Bar** no rodapé: caminho do repositório ativo, indicador `Watcher active`, perfil ativo, `HEAD` atual, `UTF-8 • LF`, badges remotos e botões de toggle para abrir/alternar as 4 abas do Bottom Drawer: `Terminal` (`<gn-terminal-panel>`), `Git Output` (`<gn-output-panel>`), `AI Review` (`<gn-ai-review-panel>`) e `Analytics` (`<gn-analytics-panel>`).

### Regras de Negócio e Casos de Borda
- **Troca de Aba de Repositório**: ao trocar o repositório ativo na barra superior ou no `Current repository` picker, a aba selecionada da sidebar, filtros e estado de seleção devem respeitar o repositório ativo sem vazar dados do repositório anterior (mantendo cancelamento por geração e `sweepRepoScopedDialogs`).
- **Último Repositório Fechado**: retorna imediatamente para `<gn-welcome>` e fecha overlays *repo-scoped* via `dialogs.closeRepoScoped()`.
- **Pilha de Overlays (`overlay-stack.ts`) e ESLint**: todos os novos popovers (Repo Picker, Unified Ref Switcher, Remote Split Menu, Filter Status, Profile Popover, Diff Gear) devem gerenciar fechamento com `Escape` respeitando `overlay-stack.ts` e a regra `gitnado/overlay-stack-membership`.
- **Acessibilidade e Movimento Reduzido**: novos componentes Lit devem importar `sharedStyles` primeiro em `static styles` (herdando `focusRingStyles` e `reducedMotionStyles`).

### Padrões Assumidos pelo Agente (Revise se discordar)
- **`<gn-context-dashboard>` substituído pelo Profile Pill**: a faixa horizontal dedicada do Context Dashboard deixa de existir no fluxo principal; seus dados e ações (perfil ativo, troca de perfil, status de contas GitHub/GitLab/etc.) passam a ser acessados pelo **Profile Pill** no header e seu popover/atalhos para os diálogos existentes, mantendo `unifiedProfileStore` intacto.
- **Descontinuação do `<gn-right-panel>` como coluna fixa**: seus três papéis atuais (`Changes`, `Details`, `Analytics`) são redistribuídos para a aba `Changes` da sidebar, o `main-pane` de `History` e a aba `Analytics` do Bottom Drawer, respectivamente.
- **Estado do Drawer e Largura da Sidebar em `uiStore`**: persistir abertura/altura/aba ativa do Bottom Drawer e largura da sidebar em `src/stores/ui.store.ts` (removendo a antiga dependência de `panels.right` quando não mais usada).
- **Atalho de Command Palette**: manter o atalho já registrado em `src/services/keyboard.service.ts` (exibindo o atalho real no chip do botão na barra superior, em vez de hardcodar `Ctrl+K` se o registro atual usar `Ctrl+P`).

### Pontos de Conexão no Projeto
- Orquestrador principal: `src/app-shell.ts`
- Toolbar e barras superiores: `src/components/toolbar/gn-toolbar.ts`
- Sidebars e listas: `src/components/sidebar/gn-left-panel.ts`, `gn-right-panel.ts`, `gn-file-status.ts`, `gn-commit-panel.ts`, `gn-branch-list.ts`, `gn-tag-list.ts`, `gn-stash-list.ts`, `gn-pull-request-list.ts`, `gn-gitflow-panel.ts`
- Painéis principais: `src/components/graph/gn-graph-canvas.ts`, `src/components/panels/gn-diff-view.ts`, `gn-commit-details.ts`, `gn-blame-view.ts`, `gn-file-history.ts`, `gn-terminal-panel.ts`, `gn-output-panel.ts`, `gn-analytics-panel.ts`, `src/components/dialogs/gn-ai-review-panel.ts`
- Stores e Serviços: `src/stores/ui.store.ts`, `src/stores/dialog.store.ts`, `src/stores/repository.store.ts`, `src/stores/commits.store.ts`, `src/stores/unified-profile.store.ts`, `src/services/app-menu.service.ts`, `src/services/keyboard.service.ts`
- Tokens e Testes de Contrato: `src/styles/tokens.css`, `src/utils/overlay-stack.ts`, `scripts/menu-contract.test.mjs`, `e2e/`

---

## 3. Rascunho Visual / Fluxo

```text
+----------------------------------------------------------------------------------+
| [Logo] [Repo Tab 1 (15)] [Repo Tab 2 ✓]            [PR #418] [Palette] [Profile] |
+----------------------------------------------------------------------------------+
| [Current repository ▾] | [Current branch / ref ↑2 ↓0 ▾] | [Sync/Push ▾]          |
+----------------------------------------------------------------------------------+
| [Changes 15] [History] [Branches & Refs] ||  (Banner operação ativa, se houver)  |
|------------------------------------------||--------------------------------------|
| Filtro + [Flat|Tree]                     ||  Changes: <gn-diff-view> (ou Clean   |
| [x] 15 changed files           [Unstage] ||           State + 4 Pulse Cards)     |
|   M src/app-shell.ts             +14 -3  ||  History: Commit Details + Diff      |
|   ...                                    ||           (ou <gn-graph-canvas> exp.)|
|------------------------------------------||  Refs:    Ref Inspector + Diff       |
| [AI Msg] [Vibe Check] [Hunk Split]       ||                                      |
| [Conventional][Amend][Sign off][Co-auth] ||--------------------------------------|
| Summary (0/72) / Description             ||  Bottom Drawer (colapsável):         |
| [ Commit 15 files to main ]              ||  [Terminal][Git Output][AI Rev][Anl] |
+------------------------------------------++--------------------------------------+
| ~/path • Watcher active • Profile • HEAD: main          [>_Term][Output][AI][Anl]|
+----------------------------------------------------------------------------------+
```

---

## 4. Fora de Escopo e Restrições (Anti-Goals)

- **Fora de Escopo (O que NÃO fazer nesta entrega)**:
  - Alterar a paleta de cores, tipografia ou tokens visuais de `src/styles/tokens.css` para copiar as cores do GitHub Desktop.
  - Migrar a janela do Tauri para *frameless* (`decorations: false`), construir controles de janela (`─ □ ✕`) ou substituir o menu nativo do SO (`app-menu.service.ts`) por menus HTML.
  - Redesenhar a tela `<gn-welcome>` ou reescrever os ~50 diálogos existentes em `src/components/dialogs/` para `<dialog>` nativo.
  - Implementar a barra de avaliação (`.eval-bar`) do mockup — ela existe apenas para demonstração estática no HTML de referência.
  - Alterações em comandos Rust (`src-tauri/`) que não sejam estritamente necessárias para suportar consultas já existentes.
- **Anti-Goals (O que evitar)**:
  - Não perder funcionalidades existentes (multi-seleção de commits no grafo, banner de merge/rebase/bisect, Analytics, AI staging/review, Blame, File History).
  - Não quebrar a regra de ESLint `gitnado/overlay-stack-membership`, testes de contrato (`npm run test:contract`) ou a responsividade de troca de abas de repositório em sessões multi-repo.
  - Não exigir mais cliques do que hoje para operações centrais (stage de arquivo/hunk/linha, commit, checkout de branch, fetch/pull/push).

---

## 5. Critérios de Aceite

- [ ] A barra superior exibe abas rápidas de repositórios abertos, pill de PR/CI (quando aplicável), botão de Command Palette e Profile Pill, mantendo o menu nativo do SO intacto.
- [ ] A Triple Action Bar renderiza `Current repository`, `Current branch / ref` (com Unified Ref Switcher popover contendo abas Branches/Tags/Stashes/PRs & Flow) e o split button de sincronização remota (`Fetch`/`Pull`/`Push` + menu dropdown).
- [ ] A sidebar esquerda única substitui o layout antigo de 3 colunas, oferecendo redimensionamento por splitter vertical e as 3 abas funcionais: `Changes` (com filtros, Flat/Tree e Commit Composer integrado), `History` (lista compacta com mini-grafo e multi-seleção) e `Branches & Refs` (com filtros por categoria e criação rápida).
- [ ] O `main-pane` alterna contextualmente entre: `<gn-diff-view>` / Clean State com 4 Pulse Cards (em `Changes`), detalhes + diff do commit e expansão sob demanda para `<gn-graph-canvas>` completo (em `History`), Ref Inspector (em `Branches & Refs`), e subviews inline de Blame/File History.
- [ ] A Status Bar inferior permite abrir, alternar e colapsar o Bottom Drawer unificando `Terminal`, `Git Output`, `AI Review` e `Analytics`.
- [ ] O banner de operação ativa (`merge`, `rebase`, `cherry-pick`, `revert`, `bisect`, conflitos) permanece funcional no topo do `main-pane`.
- [ ] Os temas e tokens atuais de `src/styles/tokens.css` (light/dark) são preservados em toda a nova estrutura, e `npm run lint`, `npm run typecheck` e `npm run test:contract` passam sem regressões.
