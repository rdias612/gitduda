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

function Ensure-CMake {
    if (Get-Command "cmake" -ErrorAction SilentlyContinue) {
        return
    }

    $candidates = @(
        "C:\Program Files\CMake\bin\cmake.exe",
        "C:\Program Files (x86)\CMake\bin\cmake.exe",
        "C:\Program Files\Microsoft Visual Studio\*\*\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe",
        "C:\Program Files (x86)\Microsoft Visual Studio\*\*\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe"
    )

    foreach ($pattern in $candidates) {
        $match = Get-Item $pattern -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($match) {
            $cmakeDir = Split-Path -Parent $match.FullName
            $env:PATH = "$cmakeDir;$env:PATH"
            Write-Host "==> CMake encontrado e adicionado ao PATH: $cmakeDir" -ForegroundColor DarkGray
            return
        }
    }

    Write-Host "[ERRO] 'cmake' nao encontrado no PATH nem no Visual Studio Build Tools. Instale o CMake ou o workload C++ do Visual Studio Build Tools." -ForegroundColor Red
    exit 1
}

function Assert-PortFree {
    param([int]$Port)
    $listener = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($listener) {
        $proc = Get-Process -Id $listener.OwningProcess -ErrorAction SilentlyContinue
        $procName = if ($proc) { "$($proc.ProcessName) (PID $($listener.OwningProcess))" } else { "PID $($listener.OwningProcess)" }
        Write-Host "[ERRO] A porta $Port ja esta em uso por $procName. Encerre o processo antes de iniciar o modo dev." -ForegroundColor Red
        exit 1
    }
}

function Invoke-Step {
    param([string]$Label, [scriptblock]$Command)
    Write-Host "==> $Label" -ForegroundColor Cyan
    $global:LASTEXITCODE = 0
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
    Assert-Command "git" "Instale o Git (https://git-scm.com/)."
    Ensure-CMake

    if ($Install -or -not (Test-Path (Join-Path $PSScriptRoot "node_modules"))) {
        Invoke-Step "Instalando dependencias Node (npm install)..." { npm install }
    }

    if ($Check) {
        Invoke-Step "Verificando tipos (npm run typecheck)..." { npm run typecheck }
        Invoke-Step "Verificando lint (npm run lint)..." { npm run lint }
    }

    if ($Release) {
        Invoke-Step "Compilando release sem pacote (tauri build --no-bundle)..." {
            npm run tauri '--' build --no-bundle
        }

        $exePath = Join-Path $PSScriptRoot "src-tauri\target\release\gitduda.exe"
        if (-not (Test-Path $exePath)) {
            Write-Host "[ERRO] Executavel nao encontrado em: $exePath" -ForegroundColor Red
            exit 1
        }

        Write-Host "==> Iniciando $exePath" -ForegroundColor Green
        Start-Process -FilePath $exePath
        exit 0
    }
    else {
        Assert-PortFree -Port 1420
        Invoke-Step "Iniciando Gitduda em modo dev (npm run tauri:dev)..." {
            npm run tauri:dev
        }
    }
}
finally {
    Pop-Location
}
