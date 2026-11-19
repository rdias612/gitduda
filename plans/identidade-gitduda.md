# Plano de Desenvolvimento: Rebrand e Isolamento de Identidade para Gitduda

- **Data**: 2026-11-18
- **Status**: [x] Concluído
- **Debate Técnico**: Validado e revisado (Etapas 1–6)
- **Arquivo de Controle**: `plans/identidade-gitduda.md`

## 1. Visão Geral e Regras Locais Aplicáveis
- **Objetivo**: Alterar a identidade do fork de **Gitnado** (`io.github.hegsie.gitnado` / `hegsie/gitnado`) para **Gitduda** (`io.github.rdias612.gitduda` / `rdias612/gitduda`) com isolamento completo em relação ao Gitnado original instalado (instância única, diretórios de dados/config, keyring, deep-link e WiX `upgradeCode`), rebrand amplo (UI/menus, `User-Agent`, workflows do GitHub Actions e manifestos em `packaging/`), ajuste do auto-updater (`createUpdaterArtifacts: false` apontando para `rdias612/gitduda`), renomeação do repositório no GitHub para `rdias612/gitduda` e empacotamento **MSIX** para distribuição gratuita via **Microsoft Store Web Installer** (eliminando avisos do SmartScreen sem compra de certificado).
- **Decisões de Escopo Fechadas**:
  - **Identidade principal**: `productName = "Gitduda"`, `identifier = "io.github.rdias612.gitduda"`, nome de pacote/binário `gitduda`, esquema deep-link `gitduda://`, pastas de config/dados `gitduda`, keyring `gitduda-git` / `gitduda-integrations` / `gitduda-ai`, repositório `https://github.com/rdias612/gitduda`.
  - **Updater**: Desativar `createUpdaterArtifacts` (`false`) e apontar `plugins.updater.endpoints` para `https://github.com/rdias612/gitduda/releases/latest/download/latest.json` até que uma chave minisign própria seja configurada.
  - **Símbolos internos preservados (para evitar churn e manter diffs limpos com upstream)**: manter `[lib] name = "gitnado_lib"`, tipos internos Rust (`GitnadoError`), chaves internas de `localStorage` (já isoladas automaticamente pelo `identifier` no WebView2) e variáveis de ambiente internas (`GITNADO_MCP_TOKEN`, `GITNADO_MCP_URL`, `GITNADO_TASK_PROMPT`).
- **Restrições do Projeto**:
  - Não existe `AGENTS.md`/`GEMINI.md` na raiz; respeitar os contratos automatizados de `npm run test:contract` (`scripts/menu-contract.test.mjs`, `scripts/i18n-coverage.test.mjs`, `scripts/ipc-contract.test.mjs`, `scripts/security-lock.test.mjs`) e os testes Rust (`app_paths::tests::the_shipped_identifier_matches_the_bundle`).
  - Gerar um **novo UUID v4** para `bundle.windows.wix.upgradeCode` em `src-tauri/tauri.conf.json` e replicá-lo (em maiúsculas entre chaves `{...}`) no template WiX de `packaging/winget/`, garantindo que o instalador MSI do Gitduda nunca sobrescreva o Gitnado original.
- **Comando de Build Global**: `npm run typecheck && npm run lint && npm test && cargo check --manifest-path src-tauri/Cargo.toml`

## 2. Checklist de Execução de Etapas
- [x] Etapa 1: Identidade de Pacote, Manifesto Tauri, Updater e Scripts Locais
- [x] Etapa 2: Isolamento de Diretórios, Keyring, Deep-Link, Menus, Tray e User-Agent (Backend Rust)
- [x] Etapa 3: Rebrand de UI, Deep-Link OAuth, Snippets MCP, i18n e Specs E2E (Frontend)
- [x] Etapa 4: Workflows do GitHub Actions e Manifestos em `packaging/`
- [x] Etapa 5: Rename do Repositório no GitHub, Atualização do Remote Local e Validação de Coexistência
- [x] Etapa 6: Empacotamento MSIX e Distribuição via Microsoft Store Web Installer (Sem SmartScreen)

---

## 3. Especificação das Etapas

### Etapa 1: Identidade de Pacote, Manifesto Tauri, Updater e Scripts Locais
- **Objetivo**: Atualizar os manifestos de empacotamento, identificador Tauri (em sincronia com `APP_IDENTIFIER`), `upgradeCode` WiX, esquema deep-link, configuração do updater e scripts locais de desenvolvimento para `gitduda` / `Gitduda`.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src-tauri/tauri.conf.json#L3-L82` (`productName`, `identifier`, `app.windows[0].title`, `bundle.publisher`, `bundle.homepage`, `bundle.copyright`, `bundle.createUpdaterArtifacts`, `bundle.windows.wix.upgradeCode`, `plugins.updater.endpoints`, `plugins.deep-link.desktop.schemes`)
  - `src-tauri/src/utils/app_paths.rs#L15` (`APP_IDENTIFIER` — mantido sincronizado com `tauri.conf.json` para preservar `the_shipped_identifier_matches_the_bundle` ao final da Etapa 1)
  - `src-tauri/Cargo.toml#L1-L10` (`[package]` `name`, `authors`, `homepage`, `repository` — preservar `[lib] name = "gitnado_lib"` em `#L17-L19`)
  - `package.json#L2-L9` (`name`, `homepage`, `repository.url`)
  - `index.html#L7` (`<title>`)
  - `run-dev.ps1#L3-L75` (mensagens de log e `$exePath` apontando para `src-tauri\target\release\gitduda.exe`)
- **Instruções Técnicas**:
  - Em `src-tauri/tauri.conf.json`:
    - Alterar `productName` para `"Gitduda"`, `identifier` para `"io.github.rdias612.gitduda"` e `windows[0].title` para `"Gitduda"`.
    - Alterar `bundle.publisher` para `"rdias612"`, `bundle.homepage` para `"https://github.com/rdias612/gitduda"`, `bundle.copyright` para `"Copyright © 2026 rdias612"` e `bundle.createUpdaterArtifacts` para `false`.
    - Substituir `bundle.windows.wix.upgradeCode` por um novo UUID v4 exclusivo (anotar o UUID gerado para reutilizar na Etapa 4 em `packaging/winget/` e `.github/scripts/winget-manifests.test.mjs`).
    - Alterar `plugins.updater.endpoints` para `["https://github.com/rdias612/gitduda/releases/latest/download/latest.json"]` e `plugins.deep-link.desktop.schemes` para `["gitduda"]`.
  - Em `src-tauri/src/utils/app_paths.rs#L15`: atualizar `APP_IDENTIFIER` para `"io.github.rdias612.gitduda"` na mesma etapa de `tauri.conf.json`.
  - Em `src-tauri/Cargo.toml#L1-L10`: alterar `name = "gitduda"`, `authors = ["rdias612"]`, `homepage = "https://github.com/rdias612/gitduda"`, `repository = "https://github.com/rdias612/gitduda"`.
  - Em `package.json#L2-L9`: alterar `name` para `"gitduda"`, `homepage` para `"https://github.com/rdias612/gitduda"` e `repository.url` para `"https://github.com/rdias612/gitduda.git"`.
  - Em `index.html#L7`: alterar `<title>Gitnado</title>` para `<title>Gitduda</title>`.
  - Em `run-dev.ps1#L3-L75`: atualizar textos para `Gitduda` e `$exePath` para `src-tauri\target\release\gitduda.exe`.
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 3 (cargo check --manifest-path src-tauri/Cargo.toml && cargo test --manifest-path src-tauri/Cargo.toml app_paths::tests::the_shipped_identifier_matches_the_bundle && npm run typecheck)`
- **Critérios de Aceitação (Code Review)**:
  - `tauri.conf.json` válido segundo o schema Tauri 2, com `identifier = "io.github.rdias612.gitduda"`, `createUpdaterArtifacts: false`, novo UUID v4 em `upgradeCode` e scheme `"gitduda"`.
  - `cargo test --manifest-path src-tauri/Cargo.toml app_paths::tests::the_shipped_identifier_matches_the_bundle` passa ao final da Etapa 1.
  - `Cargo.toml` compila sem erros (`cargo check`) preservando `gitnado_lib` para não quebrar `main.rs`.
  - `run-dev.ps1` aponta para `src-tauri\target\release\gitduda.exe` e passa na validação de sintaxe PowerShell.

### Etapa 2: Isolamento de Diretórios, Keyring, Deep-Link, Menus, Tray e User-Agent (Backend Rust)
- **Objetivo**: Isolar todos os diretórios de configuração/dados, cofres de credenciais no sistema operacional, validação do deep-link, textos nativos de menu/tray, superfícies externas OAuth/MCP e cabeçalhos `User-Agent` no backend Rust, mantendo toda a suíte unitária Rust verde.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src-tauri/src/utils/app_paths.rs#L11-L12,#L68-L90` (`APP_DIR_NAME` e testes de diretório `app_subdir_returns_*`, `repo_dir_returns_*`, `ensure_app_subdir_creates_directory`)
  - Testes unitários de comandos que validam `repo_dir` com literal `"gitnado"`:
    - `src-tauri/src/commands/branch_rules.rs#L451-L462` (`test_rules_file_location`)
    - `src-tauri/src/commands/custom_actions.rs#L660-L680` (`test_read_actions_malformed_json`, `test_actions_file_path`)
    - `src-tauri/src/commands/gitflow.rs#L1320-L1335` (fixture em `git_repo.path().join("gitnado")`)
    - `src-tauri/src/commands/jira.rs#L940-L969` (`test_save_and_load_config`)
    - `src-tauri/src/commands/validation.rs#L835-L846` (`test_rules_file_location`)
  - `src-tauri/src/services/credentials_service.rs#L15` (`SERVICE_NAME`)
  - `src-tauri/src/commands/credentials.rs#L962` (`INTEGRATION_SERVICE`)
  - `src-tauri/src/services/ai/config.rs#L11` (`AI_KEYRING_SERVICE`)
  - `src-tauri/src/lib.rs#L90-L118` e `#L307-L325` (`EnvFilter`, log de inicialização, filtro `gitduda://` no `tauri_plugin_single_instance`, menu/tooltip do tray)
  - `src-tauri/src/menu.rs#L154` e `#L260-L268` (`APP_MENU` item `"about"`, `AboutMetadata`, `SubmenuBuilder` no macOS)
  - `src-tauri/src/commands/github.rs#L336-L3804` (constante compartilhada `GITHUB_USER_AGENT = "Gitduda-Git-Client"` substituindo as 30 ocorrências de `"Gitnado-Git-Client"`)
  - `src-tauri/src/services/github_app.rs#L174,#L227` (`User-Agent`)
  - `src-tauri/src/services/ai/local/model_manager.rs#L335` (`User-Agent`)
  - `src-tauri/src/services/embedding/embedding_model.rs#L165` (`User-Agent`)
  - Superfícies externas pontuais (sem alterar logs/erros internos de `mcp/server.rs`):
    - `src-tauri/src/services/provider_http.rs#L13,#L175` (`DEFAULT_PROVIDER_USER_AGENT = "Gitduda/1.0"`)
    - `src-tauri/src/services/loopback_server.rs#L521,#L711` (`return to Gitduda`)
    - `src-tauri/src/services/ai/mcp/server.rs#L913,#L1103,#L1323` (`"name": "gitduda"`, `realm="Gitduda MCP"` e `test_handle_initialize`)
    - `src-tauri/src/services/ai/mcp_client.rs#L341` (`"clientInfo": { "name": "gitduda" }`)
- **Instruções Técnicas**:
  - Em `src-tauri/src/utils/app_paths.rs#L11-L12,#L68-L90`: alterar `APP_DIR_NAME = "gitduda"`, atualizando comentários e asserções dos testes de diretório.
  - Atualizar os literais `"gitnado"` para `"gitduda"` (ou `crate::utils::app_paths::APP_DIR_NAME`) nos 6 testes dos 5 módulos listados (`branch_rules.rs`, `custom_actions.rs`, `gitflow.rs`, `jira.rs`, `validation.rs`).
  - Alterar as constantes de serviço do Keyring para `"gitduda-git"` (`credentials_service.rs#L15`), `"gitduda-integrations"` (`commands/credentials.rs#L962`) e `"gitduda-ai"` (`services/ai/config.rs#L11`).
  - Em `src-tauri/src/lib.rs`:
    - Atualizar o `EnvFilter` padrão (`#L91`) para `"gitduda=debug,gitnado_lib=debug,git2=warn"` e o log (`#L96`) para `"Starting Gitduda"`.
    - Atualizar o deep-link (`#L114`) para `url.starts_with("gitduda://")`.
    - Atualizar o tray (`#L308` e `#L323`) para `"Show/Hide Gitduda"` e `.tooltip("Gitduda")`.
  - Em `src-tauri/src/menu.rs`: alterar o rótulo em `#L154` para `"About Gitduda"` (mantendo o `id` `"about"` para preservar o contrato em `scripts/menu-contract.test.mjs`) e os nomes macOS em `#L262,#L266` para `"Gitduda"`.
  - Atualizar os cabeçalhos `User-Agent` para `"Gitduda-Git-Client"` (em `commands/github.rs`, extraindo `const GITHUB_USER_AGENT: &str = "Gitduda-Git-Client";`, além de `services/github_app.rs` e `services/embedding/embedding_model.rs`), `"Gitduda-Git-GUI"` (`services/ai/local/model_manager.rs#L335`) e `"Gitduda/1.0"` (`services/provider_http.rs#L13,#L175`).
  - Atualizar apenas as superfícies externas pontuais em `loopback_server.rs#L521,#L711` (`return to Gitduda`), `mcp/server.rs#L913,#L1103,#L1323` (`"gitduda"` e `realm="Gitduda MCP"`) e `mcp_client.rs#L341` (`"gitduda"`), preservando logs/mensagens internas de `mcp/server.rs`.
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 3 (cargo test --manifest-path src-tauri/Cargo.toml --lib && cargo check --manifest-path src-tauri/Cargo.toml && node --test scripts/menu-contract.test.mjs)`
- **Critérios de Aceitação (Code Review)**:
  - `cargo test --manifest-path src-tauri/Cargo.toml --lib` passa integralmente (incluindo `app_paths`, os 5 módulos de comandos e `mcp/server.rs::test_handle_initialize`).
  - `scripts/menu-contract.test.mjs` passa sem alterações de IDs/ordem.
  - Nenhum serviço Keyring, pasta de dados ou superfície externa (`User-Agent`, OAuth HTML, MCP server/client info) compartilha nomes com `gitnado`.

### Etapa 3: Rebrand de UI, Deep-Link OAuth, Snippets MCP, i18n e Specs E2E (Frontend)
- **Objetivo**: Atualizar todas as strings visíveis ao usuário no frontend, o prefixo de deep-link OAuth (`gitduda://oauth/`), os snippets de cliente MCP, os catálogos i18n (com extração + build), os testes unitários e os specs E2E afetados.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src/services/oauth.service.ts#L188,#L221` (`OAUTH_DEEP_LINK_PREFIXES`) e `src/services/__tests__/oauth.deep-link.test.ts#L22-L46`
  - `src/components/welcome/gn-welcome.ts#L597-L598` (`alt` e `<div class="logo">`)
  - `src/app-shell.ts#L5345-L5348` (`showAboutDialog`)
  - `src/components/dialogs/gn-credentials-dialog.ts#L989` (texto do storage nativo)
  - `src/components/dialogs/gn-settings-dialog.ts#L1562,#L2170,#L2646` (chave MCP e textos de ajuda)
  - `src/components/mcp/gn-mcp-consent-dialog.ts#L217` (texto de consentimento)
  - `src/components/panels/gn-output-panel.ts#L373,#L489` (legenda de comandos libgit2)
  - `src/services/security-sync.service.ts#L65` e `src/services/watcher.service.ts#L55` (mensagens de aviso ao usuário)
  - `src/utils/stack-pr-table.ts#L30` (assinatura da tabela de PRs empilhados — manter marcadores HTML internos ou sincronizar com `src/utils/__tests__/stack-pr-table.test.ts` e `src/services/__tests__/stack-pr.service.test.ts`)
  - `src/services/mcp-snippets.ts#L26-L64` (`claudeCodeSnippet`, `cursorSnippet`, `vscodeSnippet`, `stdioSnippet`: renomear chave de servidor MCP para `gitduda`, mantendo env vars `GITNADO_MCP_TOKEN`/`GITNADO_MCP_URL` alinhadas a `src-tauri/src/services/ai/mcp/stdio.rs#L106-L113`) e `src/services/__tests__/mcp-snippets.test.ts`
  - `src/index.ts#L33` (log de inicialização)
  - `src/i18n/xliff/fr.xlf#L18-L19,#L284-L285,#L510-L511` e `src/i18n/generated/locales/fr.ts#L103,#L170,#L247`
  - Testes unitários afetados em `src/**/__tests__/*.test.ts` (ex.: `gn-credentials-dialog.test.ts`, `stack-pr-table.test.ts`, `mcp-snippets.test.ts`, `oauth.deep-link.test.ts`)
  - Specs E2E afetados (gate do pipeline de release em `build.yml`):
    - `e2e/tests/welcome.spec.ts#L28` (`toHaveText('Gitduda')`)
    - `e2e/tests/mcp-clients.spec.ts#L49` (`claude mcp add --transport http gitduda ...`; manter `#L56` intacta salvo alteração do mock correspondente)
- **Instruções Técnicas**:
  - Atualizar `OAUTH_DEEP_LINK_PREFIXES` para `['gitduda://oauth/']` e sincronizar todos os casos de `oauth.deep-link.test.ts`.
  - Substituir as ocorrências visíveis de `Gitnado` por `Gitduda` nos componentes/serviços listados acima, **sem** alterar chaves internas de `localStorage` nem `window.__GITNADO_STORES__` (evitando quebra de fixtures não relacionadas).
  - Atualizar o fluxo de i18n na sequência exata:
    1. Alterar as strings `msg(...)` no código-fonte.
    2. Executar `npm run localize:extract` para gerar os novos `<trans-unit>` com hashes `generateMsgId()` atualizados em `src/i18n/xliff/fr.xlf`.
    3. Traduzir os 3 novos `<trans-unit>` em `src/i18n/xliff/fr.xlf` e remover os 3 `<trans-unit>` antigos órfãos.
    4. Executar `npm run localize:build` e validar com `node --test scripts/i18n-coverage.test.mjs`.
  - Atualizar os testes unitários afetados e as 2 asserções E2E em `e2e/tests/welcome.spec.ts#L28` e `e2e/tests/mcp-clients.spec.ts#L49`.
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 3 (npm run typecheck && npm run lint && npm test; opcional se Playwright browsers estiverem instalados: npx playwright test --config=e2e/playwright.config.ts e2e/tests/welcome.spec.ts e2e/tests/mcp-clients.spec.ts)`
- **Critérios de Aceitação (Code Review)**:
  - `npm run typecheck`, `npm run lint` e `npm test` (incluindo `test:contract` / `i18n-coverage.test.mjs` e `web-test-runner`) passam sem erros.
  - A tela inicial, o diálogo "About Gitduda", as configurações, o painel de Output e os avisos exibem `Gitduda`.
  - As asserções em `e2e/tests/welcome.spec.ts#L28` e `e2e/tests/mcp-clients.spec.ts#L49` estão sincronizadas com `Gitduda` / `gitduda`.

### Etapa 4: Workflows do GitHub Actions e Manifestos em `packaging/`
- **Objetivo**: Atualizar os workflows de build/publicação (incluindo guard de `latest.json` para `createUpdaterArtifacts: false` e prefixo `manifests/r/`) e todos os manifestos de empacotamento (`homebrew`, `scoop`, `winget`) e seus scripts/testes para `rdias612/gitduda` e `Gitduda`.
- **Arquivos, Linhas e Símbolos Afetados**:
  - `.github/workflows/build.yml#L174,#L183,#L221-L224` (`releaseName`, artifact `name`, guard de `latest.json` no job `finalize-release`)
  - `.github/workflows/publish-packages.yml#L5-L6,#L62-L64,#L110,#L125,#L156-L157,#L193-L215,#L233-L248` (`WINGET_ID`, prefixo `manifests/r/`, nomes de instaladores, tap Homebrew, manifest Scoop)
  - `packaging/homebrew/gitnado.rb.template` $\rightarrow$ renomear para `packaging/homebrew/gitduda.rb.template` (`#L1-L30`)
  - `packaging/scoop/gitnado.json` $\rightarrow$ renomear para `packaging/scoop/gitduda.json` (`#L1-L30`)
  - `packaging/winget/hegsie.Gitnado.yaml.template` $\rightarrow$ `packaging/winget/rdias612.Gitduda.yaml.template` (`#L1-L9`)
  - `packaging/winget/hegsie.Gitnado.installer.yaml.template` $\rightarrow$ `packaging/winget/rdias612.Gitduda.installer.yaml.template` (`#L1-L50`, usando o mesmo `UpgradeCode` da Etapa 1)
  - `packaging/winget/hegsie.Gitnado.locale.en-US.yaml.template` $\rightarrow$ `packaging/winget/rdias612.Gitduda.locale.en-US.yaml.template` (`#L1-L31`)
  - `.github/scripts/winget-manifests.mjs#L2,#L20,#L44` (`PACKAGE_IDENTIFIER = "rdias612.Gitduda"`)
  - `.github/scripts/winget-manifests.test.mjs#L164-L402` (atualizar asserções para `rdias612.Gitduda`, `UpgradeCode` em `#L166` com o novo UUID v4 da Etapa 1, prefixo `manifests/r/` em `#L401`, `manifests/r/rdias612/Gitduda/...`, `Gitduda`, `gitduda`, `https://github.com/rdias612/gitduda/...`)
- **Instruções Técnicas**:
  - Renomear os arquivos em `packaging/` com `git mv` (ou criação + remoção) para `gitduda.rb.template`, `gitduda.json` e `rdias612.Gitduda.*.template`, atualizando URLs, `PackageIdentifier`, `Publisher`, `ProductCode`, `DisplayName`, `DefaultInstallLocation`, `Moniker`, `UpgradeCode` e caminhos `zap trash` (`io.github.rdias612.gitduda`).
  - Em `.github/workflows/build.yml`:
    - Atualizar `#L174` (`releaseName: 'Gitduda v__VERSION__'`) e `#L183` (`name: gitduda-${{ matrix.name }}`).
    - Em `#L221-L224` (logo após gerar `release.json` e antes de `gh release download`), adicionar o guard para quando `createUpdaterArtifacts: false`:
      ```bash
      if ! jq -e '.assets[] | select(.name == "latest.json")' release.json >/dev/null; then
        echo "::notice::No latest.json asset in $tag (updater disabled); skipping normalization."
        exit 0
      fi
      ```
  - Em `.github/workflows/publish-packages.yml`: atualizar `WINGET_ID: rdias612.Gitduda` (`#L110`), alterar `manifests/h/` para `manifests/r/` (`#L125`), e referenciar `Gitduda`, `gitduda`, `homebrew-gitduda`, `packaging/homebrew/gitduda.rb.template` e `packaging/scoop/gitduda.json`.
  - Atualizar `.github/scripts/winget-manifests.mjs` e `.github/scripts/winget-manifests.test.mjs` (incluindo a asserção de `UpgradeCode` em `#L166` e `manifests/r/` em `#L401`).
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 2 (node --test .github/scripts/winget-manifests.test.mjs .github/scripts/normalize-latest-json.test.mjs)`
- **Critérios de Aceitação (Code Review)**:
  - Testes em `.github/scripts/winget-manifests.test.mjs` passam 100%.
  - Nenhuma referência residual a `hegsie/gitnado`, `hegsie.Gitnado`, `manifests/h/` ou `io.github.hegsie.gitnado` permanece nos arquivos de `packaging/` ou nos workflows de build/publicação.
  - O job `finalize-release` em `.github/workflows/build.yml` sai com sucesso (notice) quando `latest.json` não existe na release.
  - O `UpgradeCode` em `rdias612.Gitduda.installer.yaml.template` e em `winget-manifests.test.mjs#L166` corresponde exatamente ao `upgradeCode` definido em `src-tauri/tauri.conf.json`.

### Etapa 5: Rename do Repositório no GitHub, Atualização do Remote Local e Validação de Coexistência
- **Objetivo**: Renomear o repositório remoto no GitHub de `rdias612/gitnado` para `rdias612/gitduda`, atualizar o remote `origin` local e validar que o Gitduda roda em paralelo com o Gitnado original sem conflitos.
- **Arquivos, Linhas e Símbolos Afetados**:
  - Configuração de remote Git local (`.git/config`, via comando `git remote set-url`)
  - Repositório remoto `https://github.com/rdias612/gitnado` $\rightarrow$ `https://github.com/rdias612/gitduda`
- **Instruções Técnicas**:
  - **Passo 1 (GitHub)**: Como o `gh` CLI não está instalado neste ambiente, renomear o repositório pela interface web em `https://github.com/rdias612/gitnado/settings` $\rightarrow$ **Repository name** $\rightarrow$ `gitduda` $\rightarrow$ **Rename** (alternativamente, via API GitHub `PATCH /repos/rdias612/gitnado` com `{"name":"gitduda"}` caso o usuário forneça/configure token, ou via `gh repo rename gitduda`).
  - **Passo 2 (Git local)**: Executar `git remote set-url origin https://github.com/rdias612/gitduda.git` (mantendo `upstream` em `https://github.com/hegsie/gitnado.git`).
  - **Passo 3 (Validação de conectividade)**: Executar `git remote -v` e `git ls-remote origin HEAD`.
  - **Passo 4 (Validação de coexistência)**: Com o Gitnado original aberto, executar `.\run-dev.ps1` (ou `.\run-dev.bat`) e confirmar que o **Gitduda** abre em janela independente e cria seus próprios diretórios (`%APPDATA%\gitduda` e `%APPDATA%\io.github.rdias612.gitduda`).
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 1 (git remote -v && git ls-remote origin HEAD)`
- **Critérios de Aceitação (Code Review)**:
  - `git remote -v` exibe `origin` apontando para `https://github.com/rdias612/gitduda.git` e `git ls-remote origin HEAD` retorna o commit do repositório renomeado.
  - Gitduda e Gitnado original conseguem rodar simultaneamente sem conflito de `single-instance` nem compartilhamento de diretórios/credenciais.

### Etapa 6: Empacotamento MSIX e Distribuição via Microsoft Store Web Installer (Sem SmartScreen)
- **Objetivo**: Incorporar o fluxo proposto em `docs/microsoft-store-web-installer.md` para empacotar o **Gitduda** em formato **MSIX** a partir do build Windows x64 do Tauri, desativando o auto-updater interno na edição da Store (requisito de certificação) e preparando a publicação gratuita na Microsoft Store com distribuição via **Microsoft Store Web Installer** (assinado pela Microsoft após certificação, sem aviso do SmartScreen e sem necessidade de comprar certificado).
- **Arquivos, Linhas e Símbolos Afetados**:
  - `src-tauri/Cargo.toml` (nova feature opcional `msix-store = []` em `[features]`)
  - `src-tauri/src/lib.rs#L104` (condicionar `.plugin(tauri_plugin_updater::Builder::new().build())` com `#[cfg(not(feature = "msix-store"))]`)
  - `src-tauri/src/services/update_service.rs#L196-L232,#L257-L275,#L392-L429` (`UpdateService::start_periodic_check`, `check_and_install_update`, `check_for_update_manual` e `install_update`)
  - `packaging/msix/AppxManifest.xml.template` *(novo arquivo)* — manifesto MSIX com `runFullTrust`, dependência `Microsoft.VCLibs.140.00.UWPDesktop`, protocolo `gitduda`, assets visuais de `src-tauri/icons/` (`Square44x44Logo.png`, `Square150x150Logo.png`, `StoreLogo.png`) e placeholders da identidade do Partner Center
  - `scripts/build-msix.ps1` *(novo arquivo)* — automação local/CI com parâmetro opcional `[string]$Target = ""` para compilar `gitduda.exe` (`--features msix-store`), montar staging apenas com os 3 PNGs referenciados, renderizar `AppxManifest.xml` (versão `X.Y.Z.0`) e gerar o `.msix` via `MakeAppx.exe` (com flag opcional `-TestSign` para sideload local)
  - `.gitignore` (adicionar `dist-msix/` e `*.msix`)
  - `.github/workflows/build.yml#L86-L90,#L180-L192` — geração condicional do artefato `.msix` na perna `Windows-x64` apenas em releases (`workflow_dispatch && inputs.release`) quando as 3 variáveis `MSIX_IDENTITY_NAME`, `MSIX_PUBLISHER` e `MSIX_PUBLISHER_DISPLAY_NAME` estiverem configuradas no GitHub
  - `docs/microsoft-store-web-installer.md#L1-L19` — atualização de `Gitnado` para `Gitduda` e guia passo a passo de submissão no Partner Center, dependência `Microsoft.VCLibs.140.00.UWPDesktop` (automática na Store / pré-requisito em sideload local), validação em Windows limpo (WebView2 + VC++ Redistributable) e adição do selo oficial em modo **Direto** (`https://apps.microsoft.com/badge`) após emissão do Product ID
- **Instruções Técnicas**:
  - Em `src-tauri/Cargo.toml`: adicionar `[features]` com `msix-store = []` (fora de `default`, sem impactar builds MSI/NSIS/macOS/Linux).
  - Em `src-tauri/src/lib.rs#L104`: envolver `.plugin(tauri_plugin_updater::Builder::new().build())` com `#[cfg(not(feature = "msix-store"))]`.
  - Em `src-tauri/src/services/update_service.rs#L196-L232,#L257-L275,#L392-L429`: usar **early-return condicional (`#[cfg(feature = "msix-store")] { return ...; }`) dentro do corpo único** de `start_periodic_check` (não agenda tarefa e loga que atualizações são gerenciadas pela Store), `check_and_install_update` (retorna `Ok(())`), `check_for_update_manual` e `install_update` (ambos retornam `Err(GitnadoError::OperationFailed("Updates are managed by Microsoft Store.".to_string()))`, evitando a mensagem enganosa `"You're up to date!"` sem alterar frontend/IPC/i18n). Aplicar `#[cfg_attr(feature = "msix-store", allow(unreachable_code, unused_variables))]` apenas nessas 4 funções (sem duplicar assinaturas nem ocultar helpers, preservando os testes estruturais do arquivo).
  - Criar `packaging/msix/AppxManifest.xml.template` contendo:
    - `<Identity Name="{{MSIX_IDENTITY_NAME}}" Publisher="{{MSIX_PUBLISHER}}" Version="{{VERSION_4PART}}" ProcessorArchitecture="x64" />`
    - `<Dependencies>` incluindo `<PackageDependency Name="Microsoft.VCLibs.140.00.UWPDesktop" MinVersion="14.0.30704.0" Publisher="CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US" />` (garante `MSVCP140.dll` exigida por `llama-cpp-2` em Windows limpo sem copiar DLLs manualmente).
    - `<rescap:Capability Name="runFullTrust" />` (obrigatório para o Gitduda acessar repositórios locais e invocar `git.exe`).
    - `<uap:Protocol Name="gitduda" />` para manter suporte a deep-links OAuth na edição MSIX.
    - Referências aos 3 ícones em `Assets/` (`Square44x44Logo.png`, `Square150x150Logo.png`, `StoreLogo.png`).
  - Criar `scripts/build-msix.ps1` com suporte a:
    - Parâmetro opcional `[string]$Target = ""` que, quando informado, repassa `--target $Target` a `npm run tauri -- build --no-bundle -- --features msix-store` e lê o executável de `src-tauri\target\$Target\release\gitduda.exe` (ou `src-tauri\target\release\gitduda.exe` quando omitido).
    - Montagem em `dist-msix/` copiando `gitduda.exe`, apenas os 3 PNGs referenciados (`Square44x44Logo.png`, `Square150x150Logo.png`, `StoreLogo.png`) de `src-tauri/icons/` para `dist-msix/Assets/` e o `AppxManifest.xml` renderizado com versão 4 partes `X.Y.Z.0`.
    - Empacotamento via `MakeAppx.exe pack /d dist-msix /p Gitduda_<versao>_x64.msix /o` (localizando automaticamente no Windows Kits SDK).
    - Flag opcional `-TestSign` usando `New-SelfSignedCertificate` + `SignTool.exe` apenas para teste local em sideload (para upload no Partner Center, enviar o `.msix` sem assinatura local para a Microsoft Store assinar na certificação).
  - Em `.gitignore`: adicionar `dist-msix/` e `*.msix`.
  - Em `.github/workflows/build.yml`: na matriz `Windows-x64`, adicionar step condicional apenas em releases com as 3 variáveis definidas (`if: matrix.platform == 'windows-latest' && github.event_name == 'workflow_dispatch' && inputs.release && vars.MSIX_IDENTITY_NAME != '' && vars.MSIX_PUBLISHER != '' && vars.MSIX_PUBLISHER_DISPLAY_NAME != ''`) executando `scripts/build-msix.ps1 -Target ${{ matrix.target }}` e incluir `*.msix` no `actions/upload-artifact`.
  - Atualizar `docs/microsoft-store-web-installer.md` com o checklist completo de publicação no Partner Center, nota sobre `Microsoft.VCLibs.140.00.UWPDesktop` (instalado automaticamente pela Store e necessário em VMs limpas para sideload local), obtenção do Product ID após certificação, inserção do selo oficial em modo **Direto** (`https://apps.microsoft.com/badge`) no `README.md` / `site/index.html` e teste final em um Windows limpo sem aviso do SmartScreen.
- **Verificação Sugerida (Avaliada no Diff)**: `Nível 3 (cargo check --manifest-path src-tauri/Cargo.toml --features msix-store && cargo test --manifest-path src-tauri/Cargo.toml --lib update_service && cargo test --manifest-path src-tauri/Cargo.toml --features msix-store --lib update_service && validação AST PowerShell de scripts/build-msix.ps1)`
- **Critérios de Aceitação (Code Review)**:
  - `cargo check` e `cargo test --lib update_service` passam integralmente tanto no build padrão quanto com `--features msix-store` (sem warnings de `dead_code` e preservando todos os testes estruturais de `update_service.rs`).
  - `tauri_plugin_updater` não é inicializado quando `msix-store` está ativo e `check_for_update_manual` retorna erro claro indicando gerenciamento pela Microsoft Store.
  - `packaging/msix/AppxManifest.xml.template` é XML válido com `Microsoft.VCLibs.140.00.UWPDesktop`, `runFullTrust`, protocolo `gitduda` e referências exatas aos 3 PNGs copiados para `dist-msix/Assets/`.
  - `scripts/build-msix.ps1` aceita `-Target`, passa na validação de sintaxe PowerShell, `.gitignore` ignora `dist-msix/` e `*.msix`, e `.github/workflows/build.yml` permanece verde mesmo sem as variáveis `MSIX_*` configuradas.
