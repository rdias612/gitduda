# Plano de Desenvolvimento: Ações em Lote no Menu de Contexto para Múltiplos Arquivos Selecionados (`gn-file-status`)

- **Data**: 2026-11-19
- **Status**: [ ] Em Execução
- **Arquivo de Controle**: `plans/issue-5-context-menu-multi-select-stage.md`
- **Issue**: `#5` ([Issue #5](https://github.com/rdias612/gitduda/issues/5))

## 1. Visão Geral e Regras Locais Aplicáveis
- **Objetivo**: Fazer com que as ações `Stage`, `Unstage` e `Discard changes` do menu de contexto em `gn-file-status.ts` operem sobre todos os arquivos selecionados na seção quando múltiplos arquivos daquela seção estiverem selecionados e o clique com o botão direito ocorrer sobre um deles (espelhando o comportamento já existente dos botões da linha `handleStageFile`, `handleUnstageFile` e `handleDiscardFile`), exibindo a contagem nos rótulos do menu (ex.: `Stage (3)`, `Unstage (3)`, `Discard changes (3)`).
- **Restrições do Projeto (`.github/copilot-instructions.md`)**:
  - **Lit Web Components & TypeScript**: Decoradores `@customElement`, `@property()`, `@state()`, tipagem estrita sem `any`, imports relativos com `.ts` (conforme padrão já adotado em `gn-file-status.ts`), sem novos pacotes ou alterações de IPC/Rust.
  - **Testes Obrigatórios**: Testes unitários de regressão em `src/components/__tests__/gn-file-status.test.ts` usando `@open-wc/testing` e mock de `__TAURI_INTERNALS__.invoke`. Não remover nem enfraquecer testes existentes.
  - **Fora de Escopo**: Ações unitárias do menu de contexto (`View diff`, `File history`, `Blame`, `Open in editor`, `Reveal`, `Copy file path`, `Add to .gitignore`) continuam operando apenas sobre `contextMenu.file`.
- **Comando de Build Global**: `npm run lint && npm run typecheck && npm test`

## 2. Checklist de Execução de Etapas
- [x] Etapa 1: Suporte a multi-seleção e rótulos dinâmicos em `gn-file-status.ts`
- [ ] Etapa 2: Testes unitários de regressão em `gn-file-status.test.ts` e validação completa

---

## 3. Especificação das Etapas

### Etapa 1: Suporte a multi-seleção e rótulos dinâmicos em `gn-file-status.ts`
- **Objetivo**: Atualizar `handleContextStage`, `handleContextUnstage`, `handleContextDiscard` e `renderContextMenu` para detectar seleção múltipla na seção alvo, delegar para os handlers em lote já existentes (`handleStageSelected`, `handleUnstageSelected`, `handleDiscardSelected`) e exibir a contagem nos itens do menu de contexto.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src/components/sidebar/gn-file-status.ts#L2011-L2014` (`getSelectedFilesInSection`)
  - `src/components/sidebar/gn-file-status.ts#L2062-L2149` (`handleStageSelected`, `handleUnstageSelected`, `handleDiscardSelected`)
  - `src/components/sidebar/gn-file-status.ts#L2168-L2198` (`handleContextStage`, `handleContextUnstage`)
  - `src/components/sidebar/gn-file-status.ts#L2368-L2416` (`handleContextDiscard`)
  - `src/components/sidebar/gn-file-status.ts#L2821-L2873` e `#L3021-L3047` (`renderContextMenu`)
- **Instruções Técnicas**:
  1. Extrair um helper privado `getContextBatchCount(file: StatusEntry, isStaged: boolean): number` (ou `isBatchContextSelection`) em `GnFileStatus` usando `this.getSelectedFilesInSection(isStaged)`:
     - Só considerar ação em lote quando `this.selectedFiles.has(file.path)` e `selectedInSection.length > 1` (evitando tratar como lote quando o clique direito ocorrer em um arquivo fora da seleção ou quando houver apenas 1 arquivo selecionado naquela seção e outro na seção oposta).
  2. Em `handleContextStage` (`#L2168-L2184`):
     - Capturar `{ file, isStaged } = this.contextMenu` antes de fechar o menu.
     - Se `this.getContextBatchCount(file, isStaged) > 1`, chamar `await this.handleStageSelected(); return;` **antes** da verificação `if (file.isConflicted)` (mesma invariante documentada em `handleStageFile#L1793-L1800`, garantindo que arquivos não conflitantes selecionados sejam staged e os conflitantes ignorados com toast).
  3. Em `handleContextUnstage` (`#L2186-L2198`):
     - Capturar `{ file, isStaged } = this.contextMenu` antes de fechar o menu.
     - Se `this.getContextBatchCount(file, isStaged) > 1`, chamar `await this.handleUnstageSelected(); return;`.
  4. Em `handleContextDiscard` (`#L2368-L2416`):
     - Capturar `{ file, isStaged } = this.contextMenu` antes de fechar o menu.
     - Se `this.getContextBatchCount(file, isStaged) > 1`, chamar `await this.handleDiscardSelected(); return;` **antes** de `if (file.isConflicted)` e antes dos locks `this.discarding` / `tryAcquireRefOp(repoPath)` (pois `handleDiscardSelected` gerencia locks, confirmação em lote e remoção de `selectedFiles`).
  5. Em `renderContextMenu` (`#L2821-L2873` e `#L3021-L3044`):
     - Calcular `const batchCount = this.getContextBatchCount(file, isStaged);` e `const countSuffix = batchCount > 1 ? \` (\${batchCount})\` : '';`.
     - Renderizar os rótulos: `Unstage${countSuffix}`, `Stage${countSuffix}` e `Discard changes${countSuffix}`.
     - Nota: garantir que o trecho de `.context-menu-item` para `Discard changes` permaneça exibindo `"Discard"` e classe `.danger` para compatibilidade com os testes existentes.
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 2` (`npm run lint && npm run typecheck`)
- **Critérios de Aceitação (Code Review)**:
  - Clicar com o botão direito em um dos múltiplos arquivos selecionados em `Changes` exibe `Stage (N)` e `Discard changes (N)` e, ao clicar em `Stage (N)`, envia todos os `N` paths não conflitantes para `stage_files` e limpa-os de `selectedFiles`.
  - Clicar com o botão direito em um dos múltiplos arquivos selecionados em `Staged` exibe `Unstage (N)` e, ao clicar, envia todos os `N` paths para `unstage_files` e limpa-os de `selectedFiles`.
  - Clicar com o botão direito em um arquivo **não** pertencente à seleção múltipla mantém os rótulos unitários (`Stage`, `Unstage`, `Discard changes`) e age apenas sobre o arquivo clicado.
  - Seleção mista (ex.: 1 arquivo em `Staged` e 1 em `Changes`) não ativa modo lote indevidamente no menu de contexto de nenhuma das seções.

### Etapa 2: Testes unitários de regressão em `gn-file-status.test.ts` e validação completa
- **Objetivo**: Cobrir os cenários de multi-seleção via menu de contexto (`Stage`, `Unstage`, `Discard changes`, clique direito em arquivo fora da seleção, seleção mista entre seções e contagem no rótulo) na suíte unitária e validar lint, typecheck e todos os testes.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src/components/__tests__/gn-file-status.test.ts#L569-L736` (`describe('context menu', ...)`)
- **Instruções Técnicas**:
  1. Na suíte `describe('context menu', ...)` em `src/components/__tests__/gn-file-status.test.ts`, adicionar testes cobrindo:
     - **Stage em lote via menu de contexto**: selecionar 2+ arquivos em `Changes`, abrir `contextmenu` em um deles, verificar rótulo `Stage (2)` (ou `Stage (N)`), clicar e confirmar que `stage_files` recebe todos os `N` paths e remove os staged de `.file-item.selected`.
     - **Unstage em lote via menu de contexto**: selecionar 2+ arquivos em `Staged`, abrir `contextmenu` em um deles, verificar rótulo `Unstage (2)`, clicar e confirmar que `unstage_files` recebe todos os `N` paths.
     - **Discard em lote via menu de contexto**: selecionar 2+ arquivos em `Changes`, abrir `contextmenu` em um deles, verificar rótulo `Discard changes (2)`, clicar e confirmar que `discard_changes` recebe todos os `N` paths após confirmação.
     - **Clique direito em arquivo fora da seleção**: com 2 arquivos selecionados, abrir `contextmenu` em um 3º arquivo não selecionado, verificar que o rótulo permanece `Stage` (sem contagem) e que `stage_files` recebe apenas o arquivo clicado.
     - **Seleção mista entre seções (1 em Staged, 1 em Changes)**: abrir `contextmenu` no arquivo de `Changes`, verificar que o rótulo é `Stage` (sem contagem) e que apenas esse arquivo é staged.
  2. Executar `npm run lint`, `npm run typecheck` e `npm test` (ou `npx web-test-runner` + `npm run test:contract`) e confirmar 100% de aprovação sem regressões.
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 2` (`npm run lint && npm run typecheck && npm test`)
- **Critérios de Aceitação (Code Review)**:
  - Todos os novos testes de menu de contexto em `gn-file-status.test.ts` passam sem mocks frágeis ou assertions relaxadas.
  - `npm run lint`, `npm run typecheck` e `npm test` passam integralmente após a edição final.
