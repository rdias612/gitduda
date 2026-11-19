<#
.SYNOPSIS
    Executa o Gitduda localmente sem gerar pacote/instalador.

.DESCRIPTION
    Por padrao, instala dependencias (se node_modules nao existir) e inicia o app
    em modo de desenvolvimento via `npm run tauri:dev`.

.PARAMETER Release
    Compila o binario de release sem gerar instalador (`--no-bundle`) e o executa.

.PARAMETER Install
    Forca a execucao de `npm install` mesmo que `node_modules` ja exista.

.PARAMETER Check
    Executa `npm run typecheck` e `npm run lint` antes de iniciar o app.
#>
param(
    [switch]$Release,
    [switch]$Install,
    [switch]$Check
)

function Assert-Command {
    param([string]$Name, [string]$Hint)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        Write-Host "[ERRO] '$Name' nao encontrado no PATH. $Hint" -ForegroundColor Red
        exit 1
    }
}

function Invoke-Step {
    param([string]$Label, [scriptblock]$Command)
    Write-Host "==> $Label" -ForegroundColor Cyan
    & $Command
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERRO] Falha em: $Label (codigo $LASTEXITCODE)" -ForegroundColor Red
        exit $LASTEXITCODE
    }
}

Push-Location $PSScriptRoot
try {
    Assert-Command "node" "Instale o Node.js (https://nodejs.org/)."
    Assert-Command "npm" "Instale o Node.js/npm."
    Assert-Command "cargo" "Instale o Rust via rustup (https://rustup.rs/)."

    if ($Install -or -not (Test-Path (Join-Path $PSScriptRoot "node_modules"))) {
        Invoke-Step "Instalando dependencias Node (npm install)..." { npm install }
    }

    if ($Check) {
        Invoke-Step "Verificando tipos (npm run typecheck)..." { npm run typecheck }
        Invoke-Step "Verificando lint (npm run lint)..." { npm run lint }
    }

    if ($Release) {
        Invoke-Step "Compilando release sem pacote (tauri build --no-bundle)..." {
            npm run tauri -- build --no-bundle
        }

        $exePath = Join-Path $PSScriptRoot "src-tauri\target\release\gitduda.exe"
        if (-not (Test-Path $exePath)) {
            Write-Host "[ERRO] Executavel nao encontrado em: $exePath" -ForegroundColor Red
            exit 1
        }

        Write-Host "==> Iniciando $exePath" -ForegroundColor Green
        & $exePath
        exit $LASTEXITCODE
    }
    else {
        Invoke-Step "Iniciando Gitduda em modo dev (npm run tauri:dev)..." {
            npm run tauri:dev
        }
    }
}
finally {
    Pop-Location
}
