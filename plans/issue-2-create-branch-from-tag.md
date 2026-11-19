# Plano de Desenvolvimento: Criar branch a partir de uma tag (Issue #2)

- **Data**: 2026-11-19
- **Status**: [ ] Em Execução
- **Arquivo de Controle**: `plans/issue-2-create-branch-from-tag.md`
- **Issue**: `#2`

## 1. Visão Geral e Regras Locais Aplicáveis
- **Objetivo**: Adicionar a ação **Create branch from this tag** aos dois menus de contexto de tags existentes — lista lateral de **Tags** (`gn-tag-list`) e menu de contexto de referências de tag no gráfico de commits (`app-shell`) — reutilizando integralmente o diálogo compartilhado `gn-create-branch-dialog` e o comando backend `create_branch` com a referência qualificada `refs/tags/<nome-da-tag>` para evitar colisões com branches homônimas e suportar tags leves e anotadas.
- **Restrições do Projeto (`eslint.config.mjs` / convenções locais)**:
  - Imports relativos internos usam extensão explícita `.ts`.
  - TypeScript estrito sem `@typescript-eslint/no-explicit-any` nem variáveis não utilizadas (`@typescript-eslint/no-unused-vars`).
  - Abertura de diálogo de criação de branch **não** deve ser bloqueada por `operationInProgress` / `isRefOperationInFlight()`, pois abrir o modal não muta o repositório (o próprio `gn-create-branch-dialog` já observa `RefLockController` apenas quando `checkoutAfterCreate` está ativo).
  - Sempre qualificar a referência da tag como `refs/tags/<nome>` (ou `this.refContextMenu.fullName || \`refs/tags/${refName}\`` no menu do gráfico), espelhando o padrão já adotado em `handleCheckoutTag` (`gn-tag-list.ts#L746`) e `handleRefCheckout` (`app-shell.ts#L2634-L2635`).
  - Testes de componente/integração frontend rodam via `@open-wc/testing` + `web-test-runner`; testes backend Rust rodam via `cargo test --manifest-path src-tauri/Cargo.toml`.
- **Comando de Build Global**: `npm run typecheck && npm run lint && npm run build`

## 2. Checklist de Execução de Etapas
- [x] Etapa 1: Adicionar ação `Create branch from this tag` nos menus de contexto de tags (`gn-tag-list.ts` e `app-shell.ts`)
- [ ] Etapa 2: Adicionar testes de componente e integração frontend (`gn-tag-list.test.ts` e `app-shell-ref-context-menu.integration.test.ts`)
- [ ] Etapa 3: Adicionar testes unitários Rust em `branch.rs` para criação de branch a partir de tags leves, anotadas e com colisão de nome

---

## 3. Especificação das Etapas

### Etapa 1: Adicionar ação `Create branch from this tag` nos menus de contexto de tags (`gn-tag-list.ts` e `app-shell.ts`)
- **Objetivo**: Expor a ação **Create branch from this tag** no menu de contexto da lista lateral de Tags e no menu de contexto de tags do gráfico de commits, abrindo o diálogo compartilhado `gn-create-branch-dialog` com `startPoint` preenchido com `refs/tags/<nome-da-tag>`.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src/components/sidebar/gn-tag-list.ts#L847-L860` (adicionar `handleCreateBranchFromTag` ao lado de `handleCreateTagFromContext`)
  - `src/components/sidebar/gn-tag-list.ts#L1058-L1096` (`renderContextMenu` — inserir botão `Create branch from this tag` logo após `Checkout`)
  - `src/app-shell.ts#L2860-L2954` (adicionar `handleRefCreateBranchFromTag` junto aos handlers `handleRefDeleteTag` / `handleRefPushTag`)
  - `src/app-shell.ts#L6691-L6726` (ramo de `tag` no `refContextMenu` — inserir botão `Create branch from this tag` logo após `Checkout tag`)
  - Referência já pronta (não alterar): `src/app-shell.ts#L6214-L6215` (listener `@create-branch` no `<aside class="left-panel">`), `src/app-shell.ts#L7097-L7101` (`<gn-create-branch-dialog>` com `@branch-created`), `src/components/dialogs/gn-create-branch-dialog.ts#L256-L301`, `#L509-L519` (`open(startPoint)` e campo `Based on`).
- **Instruções Técnicas**:
  - Em `src/components/sidebar/gn-tag-list.ts`:
    - Adicionar o método `private handleCreateBranchFromTag(): void`:
      - Capturar `const tag = this.contextMenu.tag;`.
      - Fechar o menu com `this.contextMenu = { ...this.contextMenu, visible: false };`.
      - Se `!tag`, retornar sem disparar evento.
      - Disparar `new CustomEvent('create-branch', { bubbles: true, composed: true, detail: { startPoint: \`refs/tags/${tag.name}\` } })` (o `<aside class="left-panel">` em `app-shell.ts#L6214-L6215` já captura esse evento e chama `this.createBranchDialog?.open(e.detail?.startPoint)`).
    - Em `renderContextMenu()` (`#L1070-L1085`), inserir logo após o botão `Checkout` um `<button class="context-menu-item" role="menuitem" @click=${this.handleCreateBranchFromTag}>` com ícone de branch SVG (mesmo ícone usado em `app-shell.ts#L6556-L6561`, com `aria-hidden="true"`) e texto `Create branch from this tag` (sem `?disabled=${this.operationInProgress}`, alinhado a `Create Tag Here` e `Create branch from here`).
  - Em `src/app-shell.ts`:
    - Adicionar o método `private handleRefCreateBranchFromTag(): void`:
      - Se `!this.activeRepository` ou `this.refContextMenu.refType !== 'tag'`, retornar.
      - Calcular `const refName = this.refContextMenu.refName;` e `const startPoint = this.refContextMenu.fullName || (refName ? \`refs/tags/${refName}\` : '');`.
      - Fechar o menu: `this.refContextMenu = { ...this.refContextMenu, visible: false };`.
      - Se `startPoint`, chamar `this.createBranchDialog?.open(startPoint)`.
    - No template do `refContextMenu` para tags (`#L6691-L6726`), inserir logo após `Checkout tag` (`#L6701`) um `<button class="context-menu-item" @click=${this.handleRefCreateBranchFromTag}>` com o ícone SVG de branch (`width="14" height="14"`) e texto `Create branch from this tag` (sem `?disabled=${this.isRefOperationInFlight()}`).
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 2 (npm run typecheck && npx eslint src/components/sidebar/gn-tag-list.ts src/app-shell.ts)`
- **Critérios de Aceitação (Code Review)**:
  - Ambos os menus de contexto de tags exibem exatamente o rótulo `Create branch from this tag`.
  - Nenhum código de criação de branch é duplicado: ambos os fluxos abrem o `gn-create-branch-dialog` compartilhado com `refs/tags/<nome-da-tag>`.
  - O menu de contexto fecha imediatamente ao clicar na ação.
  - A ação permanece disponível sem depender de `operationInProgress` / `isRefOperationInFlight()`.

### Etapa 2: Adicionar testes de componente e integração frontend (`gn-tag-list.test.ts` e `app-shell-ref-context-menu.integration.test.ts`)
- **Objetivo**: Garantir cobertura automatizada para a exibição do item de menu e o disparo correto do `startPoint` qualificado (`refs/tags/<nome>`) tanto na lista lateral de Tags quanto no menu de referências do gráfico em `app-shell`.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src/components/__tests__/gn-tag-list.test.ts#L311-L368` (seção de testes do menu de contexto de `gn-tag-list`)
  - `src/__tests__/app-shell-ref-context-menu.integration.test.ts#L171-L255` (testes de handlers do `refContextMenu` em `app-shell`)
- **Instruções Técnicas**:
  - Em `src/components/__tests__/gn-tag-list.test.ts`:
    - Atualizar o teste de itens do menu de contexto (`#L311-L323`) para também verificar a presença de `'Create branch from this tag'`.
    - Adicionar testes verificando que clicar em `Create branch from this tag` em uma tag anotada (ex.: `v2.1.0`) e em uma tag leve (ex.: `v1.0.0`):
      - Fecha o menu de contexto (`shadowRoot.querySelector('.context-menu')` passa a ser `null`).
      - Dispara o evento `create-branch` com `bubbles: true`, `composed: true` e `detail.startPoint === 'refs/tags/<tag.name>'`.
  - Em `src/__tests__/app-shell-ref-context-menu.integration.test.ts`:
    - Adicionar um bloco `describe('handleRefCreateBranchFromTag', ...)` cobrindo:
      - Quando `refContextMenu` está aberto para uma tag com `fullName: 'refs/tags/release'` (inclusive em cenário de colisão com branch de mesmo nome), chamar `handleRefCreateBranchFromTag()` fecha o menu e chama `createBranchDialog.open('refs/tags/release')`.
      - Quando `fullName` está vazio (`''`), o fallback ainda envia `'refs/tags/<refName>'` para `createBranchDialog.open(...)`.
      - Quando `activeRepository` é `null`, não chama `createBranchDialog.open`.
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 2 (npx eslint src/components/__tests__/gn-tag-list.test.ts src/__tests__/app-shell-ref-context-menu.integration.test.ts && npx web-test-runner src/components/__tests__/gn-tag-list.test.ts src/__tests__/app-shell-ref-context-menu.integration.test.ts)`
- **Critérios de Aceitação (Code Review)**:
  - Todos os testes existentes e novos em `gn-tag-list.test.ts` e `app-shell-ref-context-menu.integration.test.ts` passam sem falhas.
  - Nenhum warning ou erro de lint nos arquivos de teste.

### Etapa 3: Adicionar testes unitários Rust em `branch.rs` para criação de branch a partir de tags leves, anotadas e com colisão de nome
- **Objetivo**: Consolidar em teste automatizado o contrato do backend `create_branch` para todos os critérios de aceitação da Issue #2 (tags leves, tags anotadas, checkout após criação e desambiguação quando existe branch com o mesmo nome da tag).
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src-tauri/src/commands/branch.rs#L1768-L1786` (bloco de testes unitários de `create_branch`, logo após `test_create_branch_from_commit`)
  - Referência de helpers: `src-tauri/src/test_utils.rs#L260`, `#L305`, `#L321` (`create_branch`, `create_tag`, `create_lightweight_tag`)
- **Instruções Técnicas**:
  - Em `src-tauri/src/commands/branch.rs`, logo após `test_create_branch_from_commit`, adicionar testes `#[tokio::test]`:
    - `test_create_branch_from_lightweight_tag`: cria commit 1 + `repo.create_lightweight_tag("v1.0.0-light")`, cria commit 2, chama `create_branch(repo.path_str(), "from-light-tag".to_string(), Some("refs/tags/v1.0.0-light".to_string()), Some(true)).await` e valida `branch.target_oid == commit1_oid.to_string()`, `branch.is_head == true` e HEAD apontando para `from-light-tag`.
    - `test_create_branch_from_annotated_tag`: cria commit 1 + `repo.create_tag("v1.0.0")`, cria commit 2, chama `create_branch(repo.path_str(), "from-annotated-tag".to_string(), Some("refs/tags/v1.0.0".to_string()), Some(false)).await` e valida que `branch.target_oid == commit1_oid.to_string()` (peel do objeto tag para o commit) e `branch.is_head == false`.
    - `test_create_branch_from_tag_prefers_tag_when_branch_has_same_name`: cria commit 1 + `repo.create_tag("release")`, cria commit 2 + `repo.create_branch("release")` (branch `release` no commit 2, tag `release` no commit 1), chama `create_branch(repo.path_str(), "hotfix-release".to_string(), Some("refs/tags/release".to_string()), Some(false)).await` e valida que `branch.target_oid == commit1_oid.to_string()` (e `!= commit2_oid.to_string()`).
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 2 (cargo test --manifest-path src-tauri/Cargo.toml test_create_branch_from_)`
- **Critérios de Aceitação (Code Review)**:
  - Os três novos testes Rust passam com `cargo test`.
  - Nenhuma alteração desnecessária em `create_branch` (o código de produção em `branch.rs#L141-L146` já usa `revparse_single` + `peel_to_commit`).
