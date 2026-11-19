# Plano de Desenvolvimento: Preenchimento Automático de Mensagem de Commit Padrão (Issue #4)

- **Data**: 2026-11-19
- **Status**: [ ] Em Execução
- **Referência**: [Issue #4](https://github.com/rdias612/gitduda/issues/4)
- **Arquivo de Controle**: `plans/auto-default-commit-message.md`

## 1. Visão Geral e Regras Locais Aplicáveis
- **Objetivo**: Preencher automaticamente os campos `Summary` e `Body` do painel de commit (`gn-commit-panel`) com uma mensagem padrão editável derivada dos arquivos atualmente em **Staged**, preservando edições manuais do usuário (assim como amend, templates, histórico e geração por IA) e limpando os campos quando todos os arquivos forem removidos do staged enquanto a mensagem ainda for a padrão.
- **Restrições do Projeto (`eslint.config.mjs` / convenções locais)**:
  - Imports relativos internos usam extensão explícita `.ts` (`../../types/git.types.ts`, `../../utils/default-commit-message.ts`).
  - TypeScript estrito sem `@typescript-eslint/no-explicit-any` nem variáveis não utilizadas (`@typescript-eslint/no-unused-vars`).
  - Componentes em `src/components/**/*.ts` seguem a regra `gitnado/overlay-stack-membership`.
  - Testes unitários e de componente rodam via `@open-wc/testing` + `web-test-runner` (com mock de `__TAURI_INTERNALS__` e `__TAURI_EVENT_PLUGIN_INTERNALS__` antes dos imports de componente).
  - Retrocompatibilidade com testes existentes que instanciam `<gn-commit-panel .repositoryPath=... .stagedCount=...>` sem passar `.stagedFiles` (default `[]`).
- **Comando de Build Global**: `npm run typecheck && npm run lint && npm run build`

## 2. Checklist de Execução de Etapas
- [x] Etapa 1: Utilitário puro de geração de mensagem padrão (`default-commit-message.ts`) e testes unitários
- [ ] Etapa 2: Propagação de `stagedFiles` (`gn-file-status` → `gn-right-panel` → `gn-commit-panel`) e lógica de auto-preenchimento
- [ ] Etapa 3: Testes de componente no `gn-commit-panel` e validação de regressão

---

## 3. Especificação das Etapas

### Etapa 1: Utilitário puro de geração de mensagem padrão (`default-commit-message.ts`) e testes unitários
- **Objetivo**: Isolar em um módulo puro e testável a regra de formatação da mensagem de commit padrão a partir de `StatusEntry[]`.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src/utils/default-commit-message.ts` (novo arquivo: `DefaultCommitMessage`, `formatStagedFileLine`, `buildDefaultCommitMessage`)
  - `src/utils/__tests__/default-commit-message.test.ts` (novo arquivo de testes unitários)
  - Referência de tipos: `src/types/git.types.ts#L265-L282` (`StatusEntry`, `FileStatus`)
- **Instruções Técnicas**:
  - Criar `src/utils/default-commit-message.ts` exportando:
    - Interface `DefaultCommitMessage { summary: string; description: string }`.
    - `formatStagedFileLine(entry: Pick<StatusEntry, 'path' | 'status'>): string` mapeando `FileStatus`:
      - `'new'` / `'untracked'` → `add file ${entry.path}`
      - `'modified'` → `modified file ${entry.path}`
      - `'deleted'` → `deleted file ${entry.path}`
      - `'renamed'` → `renamed file ${entry.path}`
      - `'copied'` → `copied file ${entry.path}`
      - `'typechange'` → `changed file ${entry.path}`
      - fallback (`'conflicted'`, `'ignored'`, etc.) → `update file ${entry.path}`
    - `buildDefaultCommitMessage(stagedFiles: ReadonlyArray<Pick<StatusEntry, 'path' | 'status'>>): DefaultCommitMessage`:
      - `stagedFiles.length === 0` → `{ summary: '', description: '' }`.
      - `stagedFiles.length === 1` → `{ summary: formatStagedFileLine(stagedFiles[0]), description: '' }`.
      - `stagedFiles.length > 1` → `{ summary: 'multiple changes', description: stagedFiles.map(formatStagedFileLine).join('\n') }`.
  - Criar `src/utils/__tests__/default-commit-message.test.ts` cobrindo: lista vazia, 1 arquivo para cada status (`new`, `modified`, `deleted`, `renamed`, `copied`, `typechange`, fallback), múltiplos arquivos (summary `multiple changes` e body linha a linha na ordem recebida) e paths com subdiretórios.
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 2 (npx eslint src/utils/default-commit-message.ts src/utils/__tests__/default-commit-message.test.ts && npx web-test-runner src/utils/__tests__/default-commit-message.test.ts)`
- **Critérios de Aceitação (Code Review)**:
  - Funções puras, sem efeitos colaterais e sem uso de `any`.
  - Todos os formatos exigidos na Issue #4 (`add file`, `modified file`, `deleted file`, `renamed file`, `multiple changes` + body linha a linha) cobertos por testes passando.

### Etapa 2: Propagação de `stagedFiles` (`gn-file-status` → `gn-right-panel` → `gn-commit-panel`) e lógica de auto-preenchimento
- **Objetivo**: Fazer a lista real de `StatusEntry[]` staged chegar ao `gn-commit-panel` e gerenciar o ciclo de vida do auto-preenchimento sem sobrescrever texto customizado, amend, templates, histórico, IA ou drafts em cache de outros repositórios.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src/components/sidebar/gn-file-status.ts#L1592-L1604` (`loadStatus` — despacho do evento `status-changed`)
  - `src/components/sidebar/gn-right-panel.ts#L10` (import de `StatusEntry`), `#L177-L209` (`stagedFiles`, `connectedCallback`, `repositoryStore.subscribe`), `#L348-L354` (binding `.stagedFiles` no `<gn-commit-panel>`), `#L384-L394` (`handleStatusChanged`)
  - `src/components/sidebar/gn-commit-panel.ts`:
    - `#L11-L26` (imports de `StatusEntry` e `buildDefaultCommitMessage`)
    - `#L876-L885` (`@property({ attribute: false }) stagedFiles: StatusEntry[] = []` e flags privadas `isAutoGeneratedMessage`, `hasCustomMessage`)
    - `#L933-L939` (campos `originalIsAutoGenerated`, `originalHasCustomMessage` para amend)
    - `#L972-L974` (`draftCache` incluindo `isAutoGeneratedMessage` e `hasCustomMessage`)
    - `#L1069-L1140` (`willUpdate` — salvar/restaurar flags no `draftCache` e reagir a `changed.has('stagedFiles')`)
    - `#L1258-L1264` (`handleHistorySelect`), `#L1369-L1386` (`loadGitTemplate`), `#L1447-L1462` (`handleTemplateChange`), `#L1676-L1686` (`handleSummaryInput`, `handleDescriptionInput`), `#L1709-L1732` (`snapshotDraftForAmend`, `restoreDraftAfterAmend`), `#L1787-L1811` (`handleGenerateMessage`), `#L1954-L1988` (`handleCommit`), `#L2125-L2134` (`handleAiStagingChanged`)
- **Instruções Técnicas**:
  - Em `gn-file-status.ts#L1594-L1603`, incluir `stagedFiles: this.stagedFiles` no `detail` do evento `status-changed` (mantendo `stagedCount` e `totalCount` inalterados).
  - Em `gn-right-panel.ts`:
    - Importar `StatusEntry` de `../../types/git.types.ts`.
    - Adicionar `@state() private stagedFiles: StatusEntry[] = []`.
    - No `connectedCallback` e no callback de `repositoryStore.subscribe` (quando `newPath !== this.repositoryPath`), inicializar `this.stagedFiles = activeRepo?.stagedFiles ?? []` e `this.stagedCount = this.stagedFiles.length` para evitar vazamento de arquivos staged entre abas de repositórios.
    - Em `handleStatusChanged`, atualizar `this.stagedFiles = e.detail.stagedFiles` quando presente no `detail`.
    - No `render()`, passar `.stagedFiles=${this.stagedFiles}` ao `<gn-commit-panel>`.
  - Em `gn-commit-panel.ts`:
    - Adicionar `@property({ attribute: false }) stagedFiles: StatusEntry[] = []`.
    - Adicionar estado interno:
      - `private isAutoGeneratedMessage: boolean = false;`
      - `private hasCustomMessage: boolean = false;`
      - `private originalIsAutoGenerated: boolean = false;`
      - `private originalHasCustomMessage: boolean = false;`
    - Estender o valor de `draftCache` com `isAutoGeneratedMessage` e `hasCustomMessage` e persistir/restaurar em `willUpdate` quando `changed.has('repositoryPath')`.
    - Em `willUpdate`, após tratar `changed.has('repositoryPath')`, verificar `if (changed.has('stagedFiles'))`:
      - Chamar `this.syncDefaultCommitMessage()`, que:
        - Retorna imediatamente se `this.amend` ou `this.hasCustomMessage` for `true`.
        - Se `this.stagedFiles.length === 0`: se `this.isAutoGeneratedMessage` for `true`, limpa `this.summary = ''`, `this.description = ''` e define `this.isAutoGeneratedMessage = false`.
        - Se `this.stagedFiles.length > 0`: só preenche se `this.isAutoGeneratedMessage` for `true` ou ambos `this.summary.trim() === '' && this.description.trim() === ''`; nesse caso aplica `buildDefaultCommitMessage(this.stagedFiles)` em `this.summary` e `this.description` e marca `this.isAutoGeneratedMessage = true`.
    - Em `handleSummaryInput` e `handleDescriptionInput`:
      - Se `this.summary === '' && this.description === ''`, marcar `this.hasCustomMessage = false` e `this.isAutoGeneratedMessage = false` (permitindo voltar a preencher caso o usuário limpe os campos e altere o staged).
      - Caso contrário, marcar `this.hasCustomMessage = true` e `this.isAutoGeneratedMessage = false`.
    - Em `handleHistorySelect`, `loadGitTemplate` (quando carregar conteúdo não vazio), `handleTemplateChange` (quando aplicar template), `handleGenerateMessage` (no sucesso) e `handleAiStagingChanged` (quando preencher `commitMessage`): marcar `this.hasCustomMessage = true` e `this.isAutoGeneratedMessage = false`.
    - Em `snapshotDraftForAmend` e `restoreDraftAfterAmend`: salvar e restaurar `originalIsAutoGenerated` e `originalHasCustomMessage`.
    - Em `handleCommit` (quando `result.success && this.repositoryPath === repoPath`): resetar `this.isAutoGeneratedMessage = false`, `this.hasCustomMessage = false`, `this.originalIsAutoGenerated = false`, `this.originalHasCustomMessage = false`.
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 3 (npm run typecheck && npx eslint src/components/sidebar/gn-file-status.ts src/components/sidebar/gn-right-panel.ts src/components/sidebar/gn-commit-panel.ts)`
- **Critérios de Aceitação (Code Review)**:
  - Alterações em `stagedFiles` atualizam `summary`/`description` automaticamente apenas quando não há texto customizado e não está em modo amend.
  - Edição manual de `summary` ou `description` nunca é sobrescrita por mudanças subsequentes em `stagedFiles`.
  - Des-stagear todos os arquivos limpa `summary`/`description` apenas quando a mensagem ativa é auto-gerada.
  - Troca de repositório não vaza `stagedFiles` nem estado de customização entre abas.
  - Compatível com `conventionalMode` (preenche apenas o texto do summary, preservando `type`/`scope`).

### Etapa 3: Testes de componente no `gn-commit-panel` e validação de regressão
- **Objetivo**: Garantir cobertura automatizada completa dos fluxos de usabilidade da Issue #4 e validar que nenhum teste existente de commit panel quebrou.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src/components/sidebar/__tests__/gn-commit-panel-default-message.test.ts` (novo arquivo de testes fixture-based)
  - Referência de padrão de setup: `src/components/sidebar/__tests__/gn-commit-panel.test.ts#L1-L60`
- **Instruções Técnicas**:
  - Criar `src/components/sidebar/__tests__/gn-commit-panel-default-message.test.ts` seguindo o mesmo mock Tauri de `gn-commit-panel.test.ts` e cobrindo:
    1. 1 arquivo staged (`new`, `modified`, `deleted`, `renamed`) preenche `summary` corretamente e mantém `description` vazia.
    2. 2+ arquivos staged preenche `summary` com `multiple changes` e `description` com uma linha por arquivo no padrão esperado.
    3. Atualização dinâmica quando arquivos são adicionados/removidos do staged enquanto a mensagem ainda é a padrão (1 → 2 → 1).
    4. Remoção de todos os arquivos staged (`[]`) limpa `summary` e `description` quando a mensagem ainda é a padrão.
    5. Edição manual de `summary` ou `description` impede sobrescrita ao alterar ou limpar `stagedFiles`.
    6. Modo `amend`, seleção de template, histórico e geração por IA não são sobrescritos por mudanças em `stagedFiles`.
    7. Troca de `repositoryPath` preserva draft e flags no `draftCache` sem vazar entre repositórios.
    8. Após commit bem-sucedido, novo arquivo staged volta a preencher a mensagem padrão.
  - Executar a suíte de testes de commit panel, `typecheck` e `lint`.
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 3 (npx web-test-runner src/utils/__tests__/default-commit-message.test.ts src/components/sidebar/__tests__/gn-commit-panel-default-message.test.ts src/components/sidebar/__tests__/gn-commit-panel.test.ts && npm run typecheck && npm run lint)`
- **Critérios de Aceitação (Code Review)**:
  - Todos os novos testes e os testes existentes de `gn-commit-panel` passam sem warnings ou erros.
  - `npm run typecheck` e `npm run lint` passam com 0 erros.
