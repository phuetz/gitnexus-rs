<#
.SYNOPSIS
  Builds and assembles the GitNexus portable USB kit.

.DESCRIPTION
  Produces a self-contained directory layout the operator can drop on a USB
  drive and bring to a client site. The kit ships:

    - gitnexus.exe (release build, statically linked)
    - chat-ui dist (Vite production build, same-origin)
    - data/.gitnexus/        — global config, models, registry
    - data/repos/<name>/.gitnexus/ — pre-indexed graph + embeddings + docs
    - launch.bat             — one-click launcher
    - README.md              — quick-start

  The kit reads its `.gitnexus/` directory from `GITNEXUS_HOME` (set by the
  launcher), so the operator's own `%USERPROFILE%\.gitnexus\` is never
  touched.

.PARAMETER OutDir
  Destination directory. Default: D:\CascadeProjects\gitnexus-kit-v0\

.PARAMETER SeedRepo
  Optional path of an indexed repo to embed (its `.gitnexus/` is copied to
  data/repos/<name>/.gitnexus/). Pass "" to skip.

.EXAMPLE
  pwsh scripts\build-kit.ps1 -SeedRepo "D:\taf\Alise_v2"

.NOTES
  Run from the workspace root. Assumes `cargo` and `npm` are on PATH.
#>
param(
    [string]$OutDir = "D:\CascadeProjects\gitnexus-kit-v0",
    [string]$SeedRepo = ""
)

$ErrorActionPreference = "Stop"
$WorkspaceRoot = Split-Path -Parent $PSScriptRoot

function Write-Step($msg) { Write-Host "==> $msg" -ForegroundColor Cyan }
function Write-Done($msg) { Write-Host "    $msg" -ForegroundColor Green }

Write-Step "Workspace: $WorkspaceRoot"
Write-Step "Output:    $OutDir"

# ─── 1. Cargo release build ──────────────────────────────────────────────
Write-Step "Building gitnexus.exe (release, may take 3-5 min on cold cache)"
Push-Location $WorkspaceRoot
try {
    cargo build --release -p gitnexus-cli
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
} finally {
    Pop-Location
}
$BinSrc = Join-Path $WorkspaceRoot "target\release\gitnexus.exe"
if (-not (Test-Path $BinSrc)) { throw "binary not produced: $BinSrc" }
Write-Done "Built: $BinSrc"

# ─── 2. chat-ui production build (same-origin) ───────────────────────────
Write-Step "Building chat-ui dist (same-origin via .env.production)"
Push-Location (Join-Path $WorkspaceRoot "chat-ui")
try {
    npm run build
    if ($LASTEXITCODE -ne 0) { throw "npm run build failed" }
} finally {
    Pop-Location
}
$WebSrc = Join-Path $WorkspaceRoot "chat-ui\dist"
if (-not (Test-Path $WebSrc)) { throw "chat-ui dist missing: $WebSrc" }
Write-Done "Built: $WebSrc"

# ─── 3. Layout the kit ───────────────────────────────────────────────────
Write-Step "Assembling kit at $OutDir"

# Wipe `gitnexus.exe`, `web/`, `launch.bat`, `README.md` only — preserves
# `data/` so the operator's customizations (config, registry, indexed repos)
# survive a re-run of this script.
foreach ($leaf in @("gitnexus.exe", "launch.bat", "README.md")) {
    $p = Join-Path $OutDir $leaf
    if (Test-Path $p) { Remove-Item $p -Force }
}
$WebDst = Join-Path $OutDir "web"
if (Test-Path $WebDst) { Remove-Item $WebDst -Recurse -Force }

New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $OutDir "data\.gitnexus\models") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $OutDir "data\repos") -Force | Out-Null

Copy-Item $BinSrc -Destination (Join-Path $OutDir "gitnexus.exe")
Copy-Item $WebSrc -Destination $WebDst -Recurse
Write-Done "Copied gitnexus.exe + web/"

# ─── 4. Seed repo (optional) ─────────────────────────────────────────────
if ($SeedRepo -and (Test-Path $SeedRepo)) {
    $SeedGitnexus = Join-Path $SeedRepo ".gitnexus"
    if (Test-Path $SeedGitnexus) {
        $RepoName = Split-Path $SeedRepo -Leaf
        $RepoDst = Join-Path $OutDir "data\repos\$RepoName\.gitnexus"
        if (Test-Path $RepoDst) { Remove-Item $RepoDst -Recurse -Force }
        New-Item -ItemType Directory -Path (Split-Path $RepoDst -Parent) -Force | Out-Null
        Write-Step "Embedding indexed repo: $RepoName"
        Copy-Item $SeedGitnexus -Destination $RepoDst -Recurse
        Write-Done "Embedded: $RepoDst"
    } else {
        Write-Warning "Seed repo '$SeedRepo' has no .gitnexus/ — skipping"
    }
}

# ─── 5. Launcher ─────────────────────────────────────────────────────────
#
# Two-stage: launch.bat sets GITNEXUS_HOME and chains rebuild-registry.ps1
# before `gitnexus.exe serve`. The PowerShell script regenerates
# data\.gitnexus\registry.json from whatever sub-directories exist under
# data\repos\, so the kit works no matter which drive letter the USB stick
# mounts on at the client site (D:\, E:\, F:\…).

$RebuildRegistry = @'
# Regenerate registry.json from data/repos/<name>/.gitnexus/meta.json so
# absolute paths reflect the current drive letter. Idempotent: rerunning is
# safe and removes any stale entries whose folder no longer exists.
param(
    [Parameter(Mandatory = $true)] [string]$KitData
)
$reposDir = Join-Path $KitData "repos"
$regDir = Join-Path $KitData ".gitnexus"
$regPath = Join-Path $regDir "registry.json"
New-Item -ItemType Directory -Path $regDir -Force | Out-Null
$entries = @()
foreach ($dir in (Get-ChildItem $reposDir -Directory -ErrorAction SilentlyContinue)) {
    $metaPath = Join-Path $dir.FullName ".gitnexus\meta.json"
    if (-not (Test-Path $metaPath)) {
        Write-Warning "skipping $($dir.Name) — no .gitnexus/meta.json"
        continue
    }
    try {
        $meta = Get-Content $metaPath -Raw | ConvertFrom-Json
    } catch {
        Write-Warning "skipping $($dir.Name) — meta.json parse error: $_"
        continue
    }
    $entries += [pscustomobject]@{
        name = $dir.Name
        path = $dir.FullName
        storagePath = Join-Path $dir.FullName ".gitnexus"
        indexedAt = $meta.indexedAt
        lastCommit = $meta.lastCommit
        stats = $meta.stats
    }
}
ConvertTo-Json -InputObject @($entries) -Depth 5 | Set-Content $regPath -Encoding UTF8
Write-Host ("Registry rebuilt: {0} repo(s)" -f $entries.Count)
'@
$RebuildRegistry | Set-Content -Path (Join-Path $OutDir "rebuild-registry.ps1") -Encoding UTF8
Write-Done "Wrote rebuild-registry.ps1"

$LaunchBat = @'
@echo off
REM GitNexus portable kit launcher
REM
REM Sets GITNEXUS_HOME so the kit reads its own data/ directory instead of
REM the operator's %USERPROFILE%\.gitnexus, then regenerates the registry
REM from the repos under data\repos\ (necessary because absolute paths
REM depend on the drive letter the USB stick mounts on), opens the browser,
REM and starts the server.

setlocal
set GITNEXUS_HOME=%~dp0data

REM API key for the LLM provider (Gemini, OpenAI, etc.). Either set it here,
REM in your shell, or edit data\.gitnexus\chat-config.json.
REM   set GITNEXUS_API_KEY=your-key-here

echo.
echo  GitNexus portable kit
echo  GITNEXUS_HOME = %GITNEXUS_HOME%
echo.

echo  Rebuilding registry from data\repos\...
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0rebuild-registry.ps1" -KitData "%~dp0data"
if errorlevel 1 (
    echo  WARN: registry rebuild failed — continuing anyway
)
echo.

echo  Browser will open at http://localhost:3000
echo  Press Ctrl+C in this window to stop the server
echo.

start "" "http://localhost:3000"
"%~dp0gitnexus.exe" serve --host 127.0.0.1 --port 3000

endlocal
'@
$LaunchBat | Set-Content -Path (Join-Path $OutDir "launch.bat") -Encoding ASCII
Write-Done "Wrote launch.bat"

# ─── 6. README ───────────────────────────────────────────────────────────
$Readme = @'
# GitNexus Portable Kit

A self-contained GitNexus install + pre-indexed knowledge graphs that runs
from any directory (USB stick, network share, sandbox folder).

## Quick start

1. Double-click `launch.bat`.
2. Wait for the console to print `GitNexus HTTP server starting`.
3. The default browser opens at `http://localhost:3000`.
4. Pick a project in the top-right selector and start asking questions.

## Layout

```
.
├── gitnexus.exe          single binary (Rust release build)
├── launch.bat            sets GITNEXUS_HOME + starts the server
├── web/                  chat-ui static (built with VITE_MCP_URL="" so it
│                         talks to the same gitnexus.exe directly)
└── data/
    ├── .gitnexus/
    │   ├── chat-config.json    LLM credentials (edit before first run)
    │   ├── registry.json       indexed-repos list (auto-managed)
    │   └── models/             ONNX embeddings models (optional)
    └── repos/
        └── <project>/
            └── .gitnexus/      pre-indexed graph + embeddings + docs
```

## LLM configuration

Edit `data/.gitnexus/chat-config.json` and fill `api_key` with a Google AI
Studio key (Gemini) or an OpenAI key. A minimal example:

```json
{
  "provider": "gemini",
  "api_key": "YOUR_KEY",
  "base_url": "https://generativelanguage.googleapis.com/v1beta/openai",
  "model": "gemini-2.5-flash",
  "max_tokens": 8192,
  "reasoning_effort": "high"
}
```

If `api_key` is empty, the chat falls back to graph-only answers (no LLM
narration but full search/diagram/hotspot tooling still works).

## Indexing a new repo on the client site

```
"%~dp0gitnexus.exe" analyze "C:\path\to\client\repo"
```

The new repo is added to `data\.gitnexus\registry.json` automatically —
visible in the project selector after a browser refresh.

## Stopping the server

Press `Ctrl+C` in the launcher console, or simply close the window.

## Troubleshooting

- **Port 3000 already in use** — edit `launch.bat`, change `--port 3000`.
- **Browser opens before the server is ready** — wait a couple seconds and
  refresh once the console prints `Chat API: POST ...`.
- **"No LLM configured"** — `data/.gitnexus/chat-config.json` is missing or
  has an empty `api_key`. The graph-only path still works.

---

Built from gitnexus-rs master.
'@
$Readme | Set-Content -Path (Join-Path $OutDir "README.md") -Encoding UTF8
Write-Done "Wrote README.md"

# ─── 7. Sample chat-config (no key) ──────────────────────────────────────
$ConfigPath = Join-Path $OutDir "data\.gitnexus\chat-config.json"
if (-not (Test-Path $ConfigPath)) {
    $SampleConfig = @'
{
  "provider": "gemini",
  "api_key": "",
  "base_url": "https://generativelanguage.googleapis.com/v1beta/openai",
  "model": "gemini-2.5-flash",
  "max_tokens": 8192,
  "reasoning_effort": "high"
}
'@
    $SampleConfig | Set-Content -Path $ConfigPath -Encoding UTF8
    Write-Done "Wrote sample chat-config.json (api_key empty — fill before use)"
}

# ─── 8. Done ─────────────────────────────────────────────────────────────
Write-Step "Kit ready: $OutDir"
$KitSize = (Get-ChildItem $OutDir -Recurse -ErrorAction SilentlyContinue |
    Measure-Object -Property Length -Sum).Sum / 1MB
Write-Done ("Total size: {0:N1} MB" -f $KitSize)
Write-Host ""
Write-Host "Next: cd $OutDir; .\launch.bat"
