[CmdletBinding()]
param(
    [string]$Target = "",
    [switch]$TestSign,
    [string]$IdentityName = $env:MSIX_IDENTITY_NAME,
    [string]$Publisher = $env:MSIX_PUBLISHER,
    [string]$PublisherDisplayName = $env:MSIX_PUBLISHER_DISPLAY_NAME
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Find-WindowsSdkTool {
    param([Parameter(Mandatory = $true)][string]$ToolName)

    $kitsRoot = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\bin"
    if (Test-Path $kitsRoot) {
        $candidates = Get-ChildItem -Path $kitsRoot -Filter $ToolName -Recurse -ErrorAction SilentlyContinue |
            Where-Object { $_.Directory.Name -eq "x64" } |
            Sort-Object { $_.Directory.Parent.Name } -Descending
        if ($candidates -and $candidates.Count -gt 0) {
            return $candidates[0].FullName
        }
    }

    $fromPath = Get-Command $ToolName -ErrorAction SilentlyContinue
    if ($fromPath) {
        return $fromPath.Source
    }

    throw "$ToolName not found. Install the Windows 10/11 SDK or ensure it is in PATH."
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

if ([string]::IsNullOrWhiteSpace($IdentityName)) {
    if ($TestSign) {
        $IdentityName = "io.github.rdias612.gitduda"
    } else {
        throw "MSIX_IDENTITY_NAME is required (set env var or pass -IdentityName)."
    }
}

if ([string]::IsNullOrWhiteSpace($Publisher)) {
    if ($TestSign) {
        $Publisher = "CN=GitdudaDev"
    } else {
        throw "MSIX_PUBLISHER is required (set env var or pass -Publisher)."
    }
}

if ([string]::IsNullOrWhiteSpace($PublisherDisplayName)) {
    if ($TestSign) {
        $PublisherDisplayName = "Gitduda Dev"
    } else {
        throw "MSIX_PUBLISHER_DISPLAY_NAME is required (set env var or pass -PublisherDisplayName)."
    }
}

$tauriConfPath = Join-Path $repoRoot "src-tauri\tauri.conf.json"
$tauriConf = Get-Content -Path $tauriConfPath -Raw -Encoding UTF8 | ConvertFrom-Json
$version = [string]$tauriConf.version
if ($version -notmatch '^\d+\.\d+\.\d+$') {
    throw "Expected X.Y.Z version in tauri.conf.json, got '$version'."
}
$version4Part = "$version.0"

Push-Location $repoRoot
try {
    $tauriArgs = @("run", "tauri", "--", "build", "--no-bundle")
    if (-not [string]::IsNullOrWhiteSpace($Target)) {
        $tauriArgs += @("--target", $Target)
    }
    $tauriArgs += @("--", "--features", "msix-store")

    Write-Host "Building Gitduda (msix-store)..."
    & npm @tauriArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Tauri build failed with exit code $LASTEXITCODE."
    }
} finally {
    Pop-Location
}

$exePath = if ([string]::IsNullOrWhiteSpace($Target)) {
    Join-Path $repoRoot "src-tauri\target\release\gitduda.exe"
} else {
    Join-Path $repoRoot "src-tauri\target\$Target\release\gitduda.exe"
}

if (-not (Test-Path $exePath)) {
    throw "Expected executable not found at '$exePath'."
}

$stagingDir = Join-Path $repoRoot "dist-msix"
$assetsDir = Join-Path $stagingDir "Assets"
if (Test-Path $stagingDir) {
    Remove-Item -Path $stagingDir -Recurse -Force
}
New-Item -ItemType Directory -Path $assetsDir -Force | Out-Null

Copy-Item -Path $exePath -Destination (Join-Path $stagingDir "gitduda.exe") -Force

$iconsDir = Join-Path $repoRoot "src-tauri\icons"
foreach ($asset in @("Square44x44Logo.png", "Square150x150Logo.png", "StoreLogo.png")) {
    $srcAsset = Join-Path $iconsDir $asset
    if (-not (Test-Path $srcAsset)) {
        throw "Required MSIX icon asset not found: '$srcAsset'."
    }
    Copy-Item -Path $srcAsset -Destination (Join-Path $assetsDir $asset) -Force
}

$templatePath = Join-Path $repoRoot "packaging\msix\AppxManifest.xml.template"
$manifestContent = Get-Content -Path $templatePath -Raw -Encoding UTF8
$manifestContent = $manifestContent.Replace("{{MSIX_IDENTITY_NAME}}", [System.Security.SecurityElement]::Escape($IdentityName))
$manifestContent = $manifestContent.Replace("{{MSIX_PUBLISHER}}", [System.Security.SecurityElement]::Escape($Publisher))
$manifestContent = $manifestContent.Replace("{{MSIX_PUBLISHER_DISPLAY_NAME}}", [System.Security.SecurityElement]::Escape($PublisherDisplayName))
$manifestContent = $manifestContent.Replace("{{VERSION_4PART}}", $version4Part)

$manifestPath = Join-Path $stagingDir "AppxManifest.xml"
[System.IO.File]::WriteAllText($manifestPath, $manifestContent, [System.Text.UTF8Encoding]::new($false))

$makeAppx = Find-WindowsSdkTool -ToolName "MakeAppx.exe"
$msixPath = Join-Path $repoRoot "Gitduda_${version}_x64.msix"

Write-Host "Packing MSIX with MakeAppx.exe..."
& $makeAppx pack /d $stagingDir /p $msixPath /o
if ($LASTEXITCODE -ne 0) {
    throw "MakeAppx.exe failed with exit code $LASTEXITCODE."
}

if ($TestSign) {
    $signTool = Find-WindowsSdkTool -ToolName "SignTool.exe"
    $cert = New-SelfSignedCertificate `
        -Type Custom `
        -Subject $Publisher `
        -KeyUsage DigitalSignature `
        -FriendlyName "Gitduda MSIX Dev Test" `
        -CertStoreLocation "Cert:\CurrentUser\My" `
        -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")

    Write-Host "Signing MSIX for local sideload with self-signed certificate $($cert.Thumbprint)..."
    & $signTool sign /fd SHA256 /sha1 $cert.Thumbprint $msixPath
    if ($LASTEXITCODE -ne 0) {
        throw "SignTool.exe failed with exit code $LASTEXITCODE."
    }

    Write-Host "To trust this test certificate on your machine (Admin PowerShell):"
    Write-Host "  Export-Certificate -Cert Cert:\CurrentUser\My\$($cert.Thumbprint) -FilePath gitduda-dev.cer"
    Write-Host "  Import-Certificate -FilePath gitduda-dev.cer -CertStoreLocation Cert:\LocalMachine\TrustedPeople"
}

Write-Host "MSIX package generated: $msixPath"
