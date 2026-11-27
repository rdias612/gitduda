# Microsoft Store Web Installer para o Gitduda

## Objetivo

Oferecer, no site e no repositório do **Gitduda**, uma instalação para Windows via **Microsoft Store Web Installer**. O Gitduda é publicado gratuitamente em formato **MSIX**, permitindo que a Microsoft Store assine o pacote após a certificação sem exigir a compra de um certificado de código EV para esse canal e evitando avisos do Microsoft Defender SmartScreen no fluxo da Store. [Opções de assinatura de código no Windows](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options) · [Microsoft Store Web Installer](https://learn.microsoft.com/pt-br/windows/apps/distribute-through-store/how-to-use-store-web-installer-for-distribution)

## Arquitetura do Empacotamento MSIX

- **Feature `msix-store` (`src-tauri/Cargo.toml`)**: desativa o `tauri_plugin_updater` no `Builder` (`src-tauri/src/lib.rs`) e substitui as verificações periódicas e manuais em `src-tauri/src/services/update_service.rs` por retornos compatíveis com a certificação da Store (`Updates are managed by Microsoft Store.`). As compilações MSI, NSIS, macOS e Linux não habilitam essa feature e mantêm o auto-updater normal.
- **Manifesto (`packaging/msix/AppxManifest.xml.template`)**: declara capacidade `runFullTrust`, protocolo deep-link `gitduda://`, os 3 ícones exigidos (`Square44x44Logo.png`, `Square150x150Logo.png`, `StoreLogo.png`) e a dependência de runtime `Microsoft.VCLibs.140.00.UWPDesktop` (`MinVersion="14.0.30704.0"`).
- **Automação (`scripts/build-msix.ps1`)**: compila `gitduda.exe` com `--features msix-store`, monta o diretório `dist-msix/`, renderiza o `AppxManifest.xml` com versão em 4 partes (`X.Y.Z.0`) e gera `Gitduda_<versao>_x64.msix` via `MakeAppx.exe`.

> [!NOTE]
> A dependência `Microsoft.VCLibs.140.00.UWPDesktop` é resolvida e instalada automaticamente pela Microsoft Store para usuários finais. Ao testar o `.msix` localmente via sideload (`Add-AppxPackage`), certifique-se de que o pacote `Microsoft.VCLibs.140.00.UWPDesktop` (x64) ou o Visual C++ Redistributable esteja instalado na máquina de teste.

## Guia Passo a Passo para Submissão no Partner Center

### 1. Configurar conta e reservar o nome no Partner Center

1. Acesse o [Microsoft Partner Center](https://partner.microsoft.com/dashboard) com a conta de desenvolvedor individual ou corporativa.
2. Crie um novo aplicativo e reserve o nome **Gitduda** na Microsoft Store.
3. Em **Gerenciamento de produtos → Identidade do produto**, copie os 3 valores oficiais:
   - `Package/Identity/Name` → variável `MSIX_IDENTITY_NAME`
   - `Package/Identity/Publisher` → variável `MSIX_PUBLISHER`
   - `Package/Properties/PublisherDisplayName` → variável `MSIX_PUBLISHER_DISPLAY_NAME`

### 2. Configurar geração automática no GitHub Actions (opcional para releases)

Em **Settings → Secrets and variables → Actions → Variables** do repositório `https://github.com/rdias612/gitduda`, cadastre:

- `MSIX_IDENTITY_NAME`
- `MSIX_PUBLISHER`
- `MSIX_PUBLISHER_DISPLAY_NAME`

Com as três variáveis definidas, uma execução manual de **Build & Release** com `release` marcado gera `Gitduda_<versao>_x64.msix` no job do Windows. A página da execução oferece o arquivo `.msix` como download direto na seção **Artifacts**.

### 3. Build local e teste via sideload

Para gerar e assinar localmente um pacote de teste (sem subir na Store):

```powershell
./scripts/build-msix.ps1 -TestSign
```

Em um PowerShell como Administrador, confie no certificado temporário gerado e instale o pacote:

```powershell
Add-AppxPackage .\Gitduda_<versao>_x64.msix
```

Para gerar o `.msix` não assinado para upload direto no Partner Center, defina `MSIX_IDENTITY_NAME`, `MSIX_PUBLISHER` e `MSIX_PUBLISHER_DISPLAY_NAME` no ambiente e execute:

```powershell
./scripts/build-msix.ps1
```

### 4. Submissão e certificação

1. No Partner Center, inicie uma nova submissão para o **Gitduda**.
2. Preencha **Preços e disponibilidade** como aplicativo gratuito, **Classificação etária**, **Propriedades** (categoria *Ferramentas de desenvolvedor*) e **Listagem da Store** (descrição, capturas de tela e ícone).
3. Na etapa **Pacotes**, envie o arquivo `Gitduda_<versao>_x64.msix` (a Store assinará o pacote após a certificação).
4. Em **Observações para certificação**, informe que o Gitduda é um cliente Git desktop que utiliza `runFullTrust` para invocar o Git/SSH local e acessar repositórios escolhidos pelo usuário.
5. Envie para certificação e aguarde a aprovação.

### 5. Obter o Product ID e inserir o selo oficial em modo Direto

1. Após a certificação, copie o **Store ID / Product ID** (formato `9N...`) no painel do Partner Center em **Identidade do produto**.
2. Acesse o [Gerador de selos da Microsoft Store](https://apps.microsoft.com/badge), informe o Product ID e selecione o modo de lançamento **Direto** (*Direct* — baixa e executa o Microsoft Store Web Installer diretamente no navegador).
3. Insira o código HTML/Markdown do selo gerado na área de downloads do `README.md` e em `site/index.html`.

### 6. Checklist de validação em Windows limpo

Em uma máquina virtual Windows 10/11 limpa (sem ambiente de desenvolvimento):

- [ ] Confirmar pré-requisitos de sistema (Microsoft Edge WebView2 Runtime e resolução automática de `Microsoft.VCLibs.140.00.UWPDesktop` pela Store).
- [ ] Baixar o instalador pelo selo oficial da Microsoft Store (modo **Direto**) e executá-lo **sem aviso do SmartScreen**.
- [ ] Abrir o **Gitduda**, testar deep-link `gitduda://`, abrir um repositório Git local e executar operações básicas (`status`, `commit`, terminal).
- [ ] Verificar em Configurações que a verificação manual de atualização informa que as atualizações são gerenciadas pela Microsoft Store (`Updates are managed by Microsoft Store.`).
- [ ] Ao publicar uma versão seguinte no Partner Center, validar a atualização automática do pacote pela Microsoft Store preservando configurações e repositórios locais.
