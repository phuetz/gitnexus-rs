[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet(
        "help",
        "chat",
        "desktop",
        "login",
        "logout",
        "config-chatgpt",
        "test-chatgpt",
        "ask",
        "analyze",
        "serve",
        "docs",
        "check",
        "doctor"
    )]
    [string] $Command = "help",

    [string] $Repo = (Get-Location).Path,
    [string] $Question = "Resume ce projet en 5 lignes.",
    [string] $Model = "gpt-5.5",

    [ValidateSet("none", "minimal", "low", "medium", "high", "xhigh")]
    [string] $Reasoning = "high",

    [int] $MaxTokens = 8192,
    [int] $BackendPort = 3010,
    [int] $ChatPort = 5176,
    [int] $DesktopUiPort = 1421,
    [string] $OutputDir = "",

    [switch] $NoBackend,
    [switch] $NoBrowser,
    [switch] $Enrich,
    [switch] $RestartBackend,
    [switch] $RestartChat
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Split-Path -Parent $ScriptDir
$CliTargetDir = "target-codex/run"
$DesktopTargetDir = "target-codex/desktop"

function Write-Step {
    param([string] $Text)
    Write-Host "==> $Text" -ForegroundColor Cyan
}

function Resolve-HomeDir {
    if ($env:USERPROFILE -and $env:USERPROFILE.Trim()) {
        return $env:USERPROFILE
    }
    if ($env:HOME -and $env:HOME.Trim()) {
        return $env:HOME
    }
    throw "Impossible de trouver le dossier utilisateur (USERPROFILE/HOME)."
}

function Require-Command {
    param([string] $Name)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Commande introuvable: $Name. Installe-la ou ajoute-la au PATH."
    }
}

function Quote-ForPowerShell {
    param([string] $Value)
    return "'" + $Value.Replace("'", "''") + "'"
}

function Invoke-GitNexusCli {
    param([Parameter(ValueFromRemainingArguments = $true)] [string[]] $CliArgs)
    Set-Location -LiteralPath $RepoRoot
    & cargo run -p gitnexus-cli --target-dir $CliTargetDir -- @CliArgs
}

function Ensure-NpmInstall {
    param([string] $Directory)
    Require-Command "npm"
    if (-not (Test-Path -LiteralPath (Join-Path $Directory "node_modules"))) {
        Write-Step "Installation npm dans $Directory"
        Push-Location -LiteralPath $Directory
        try {
            & npm install
        }
        finally {
            Pop-Location
        }
    }
}

function Set-DotEnvValue {
    param(
        [string] $Path,
        [string] $Key,
        [string] $Value
    )

    $line = "$Key=$Value"
    if (-not (Test-Path -LiteralPath $Path)) {
        Write-Utf8NoBom -Path $Path -Lines @($line)
        return
    }

    $content = Get-Content -LiteralPath $Path
    $found = $false
    $updated = foreach ($existing in $content) {
        if ($existing -match "^\s*$([regex]::Escape($Key))=") {
            $found = $true
            $line
        }
        else {
            $existing
        }
    }
    if (-not $found) {
        $updated += $line
    }
    Write-Utf8NoBom -Path $Path -Lines $updated
}

function Write-Utf8NoBom {
    param(
        [string] $Path,
        [string[]] $Lines
    )

    $utf8NoBom = New-Object System.Text.UTF8Encoding $false
    $text = ($Lines -join [Environment]::NewLine) + [Environment]::NewLine
    [System.IO.File]::WriteAllText($Path, $text, $utf8NoBom)
}

function Start-PowerShellWindow {
    param(
        [string] $Title,
        [string] $CommandLine
    )

    $encoded = @"
`$Host.UI.RawUI.WindowTitle = '$($Title.Replace("'", "''"))'
$CommandLine
"@

    Start-Process powershell -ArgumentList @(
        "-NoExit",
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        $encoded
    ) -WindowStyle Normal | Out-Null
}

function Invoke-InDirectory {
    param(
        [string] $Directory,
        [scriptblock] $Script
    )

    Push-Location -LiteralPath $Directory
    try {
        & $Script
    }
    finally {
        Pop-Location
    }
}

function Test-HttpOk {
    param([string] $Url)
    try {
        $response = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 3
        return ($response.StatusCode -ge 200 -and $response.StatusCode -lt 400)
    }
    catch {
        return $false
    }
}

function Get-HttpJson {
    param([string] $Url)
    try {
        $response = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 3
        if ($response.StatusCode -lt 200 -or $response.StatusCode -ge 400) {
            return $null
        }
        $content = [string] $response.Content
        if (-not $content.Trim()) {
            return $null
        }
        return $content | ConvertFrom-Json
    }
    catch {
        return $null
    }
}

function Test-ChatUiOk {
    param([string] $Url)
    try {
        $response = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 3
        if ($response.StatusCode -lt 200 -or $response.StatusCode -ge 400) {
            return $false
        }
        $content = [string] $response.Content
        return ($content -match '<title>\s*GitNexus Chat\s*</title>' -and $content -match 'id="root"')
    }
    catch {
        return $false
    }
}

function Wait-HttpOk {
    param(
        [string] $Url,
        [int] $TimeoutSeconds = 45
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        if (Test-HttpOk -Url $Url) {
            return $true
        }
        Start-Sleep -Milliseconds 750
    }
    return $false
}

function Test-PortListening {
    param([int] $Port)
    try {
        $connection = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue |
            Select-Object -First 1
        return $null -ne $connection
    }
    catch {
        return $false
    }
}

function Write-DoctorLine {
    param(
        [string] $Label,
        [string] $State,
        [string] $Detail = ""
    )

    $color = switch ($State) {
        "OK" { "Green" }
        "WARN" { "Yellow" }
        "KO" { "Red" }
        default { "Gray" }
    }

    $suffix = if ($Detail.Trim()) { " - $Detail" } else { "" }
    Write-Host ("[{0}] {1}{2}" -f $State, $Label, $suffix) -ForegroundColor $color
}

function Get-ListeningProcessSummary {
    param([int] $Port)

    $pids = @(Get-ListeningProcessIds -Port $Port)
    if ($pids.Count -eq 0) {
        return "libre"
    }

    $items = foreach ($pidValue in $pids) {
        $proc = Get-Process -Id $pidValue -ErrorAction SilentlyContinue
        if ($proc) {
            "$($proc.ProcessName) PID $pidValue"
        }
        else {
            "PID $pidValue"
        }
    }
    return ($items -join ", ")
}

function Show-Doctor {
    Write-Step "Diagnostic GitNexus"

    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        Write-DoctorLine "cargo" "OK" (cargo --version)
    }
    else {
        Write-DoctorLine "cargo" "KO" "introuvable dans le PATH"
    }

    if (Get-Command npm -ErrorAction SilentlyContinue) {
        Write-DoctorLine "npm" "OK" (npm --version)
    }
    else {
        Write-DoctorLine "npm" "KO" "introuvable dans le PATH"
    }

    $chatDir = Join-Path $RepoRoot "chat-ui"
    $desktopUiDir = Join-Path $RepoRoot "crates\gitnexus-desktop\ui"
    Write-DoctorLine "chat-ui" ($(if (Test-Path -LiteralPath $chatDir) { "OK" } else { "KO" })) $chatDir
    Write-DoctorLine "desktop-ui" ($(if (Test-Path -LiteralPath $desktopUiDir) { "OK" } else { "KO" })) $desktopUiDir

    $homeDir = Resolve-HomeDir
    $configPath = Join-Path $homeDir ".gitnexus\chat-config.json"
    if (Test-Path -LiteralPath $configPath) {
        try {
            $cfg = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
            $provider = if ($cfg.provider) { [string] $cfg.provider } else { "non renseigne" }
            $model = if ($cfg.model) { [string] $cfg.model } else { "non renseigne" }
            $reasoning = if ($cfg.reasoning_effort) { [string] $cfg.reasoning_effort } elseif ($cfg.reasoningEffort) { [string] $cfg.reasoningEffort } else { "non renseigne" }
            Write-DoctorLine "config ChatGPT" "OK" "provider=$provider model=$model reasoning=$reasoning"
        }
        catch {
            Write-DoctorLine "config ChatGPT" "KO" "JSON illisible: $($_.Exception.Message)"
        }
    }
    else {
        Write-DoctorLine "config ChatGPT" "WARN" "absente; lance .\config-chatgpt.cmd"
    }

    $authPath = Join-Path $homeDir ".gitnexus\auth\openai.json"
    if (Test-Path -LiteralPath $authPath) {
        Write-DoctorLine "login OAuth ChatGPT" "OK" "tokens presents dans $authPath (valeurs masquees)"
    }
    else {
        Write-DoctorLine "login OAuth ChatGPT" "WARN" "absent; lance .\login-chatgpt.cmd"
    }

    $backendUrl = "http://127.0.0.1:$BackendPort"
    $backendOk = Test-HttpOk -Url "$backendUrl/health"
    if ($backendOk) {
        Write-DoctorLine "backend HTTP :$BackendPort" "OK" "$backendUrl/health"
    }
    elseif (Test-PortListening -Port $BackendPort) {
        Write-DoctorLine "backend HTTP :$BackendPort" "WARN" "port occupe par $(Get-ListeningProcessSummary -Port $BackendPort), mais /health ne repond pas"
    }
    else {
        Write-DoctorLine "backend HTTP :$BackendPort" "WARN" "port libre; lance .\gitnexus.cmd chat"
    }

    if ($backendOk) {
        $diag = Get-HttpJson -Url "$backendUrl/api/diagnostics"
        if ($diag) {
            $repoCount = if ($diag.repos -and $null -ne $diag.repos.count) { [string] $diag.repos.count } else { "?" }
            $llmProvider = if ($diag.llm -and $diag.llm.provider) { [string] $diag.llm.provider } elseif ($diag.llm -and $diag.llm.configured -eq $false) { "non configure" } else { "?" }
            $llmModel = if ($diag.llm -and $diag.llm.model) { [string] $diag.llm.model } else { "modele ?" }
            Write-DoctorLine "diagnostic backend" "OK" "repos=$repoCount llm=$llmProvider/$llmModel"
        }
        else {
            Write-DoctorLine "diagnostic backend" "WARN" "$backendUrl/api/diagnostics ne repond pas"
        }

        $repoPayload = Get-HttpJson -Url "$backendUrl/api/repos"
        if ($repoPayload -and $null -ne $repoPayload.repos) {
            $repoItems = @($repoPayload.repos)
            $repoNames = @($repoItems |
                ForEach-Object {
                    if ($_.name) { [string] $_.name } elseif ($_.id) { [string] $_.id } else { "repo ?" }
                } |
                Select-Object -First 3)
            $extra = if ($repoItems.Count -gt 3) { ", +$($repoItems.Count - 3)" } else { "" }
            $detail = "repos=$($repoItems.Count)"
            if ($repoNames.Count -gt 0) {
                $detail += " ($($repoNames -join ', ')$extra)"
            }
            Write-DoctorLine "api/repos" "OK" $detail
        }
        else {
            Write-DoctorLine "api/repos" "WARN" "$backendUrl/api/repos ne repond pas"
        }
    }

    $chatUrl = "http://127.0.0.1:$ChatPort"
    if (Test-ChatUiOk -Url $chatUrl) {
        Write-DoctorLine "client React :$ChatPort" "OK" $chatUrl
    }
    elseif (Test-PortListening -Port $ChatPort) {
        Write-DoctorLine "client React :$ChatPort" "WARN" "port occupe par $(Get-ListeningProcessSummary -Port $ChatPort), mais ce n'est pas le chat GitNexus"
    }
    else {
        Write-DoctorLine "client React :$ChatPort" "WARN" "port libre; lance .\gitnexus.cmd chat"
    }

    $envPath = Join-Path $chatDir ".env.local"
    if (Test-Path -LiteralPath $envPath) {
        $viteUrl = (Get-Content -LiteralPath $envPath | Where-Object { $_ -match "^\s*VITE_MCP_URL=" } | Select-Object -Last 1)
        if ($viteUrl) {
            Write-DoctorLine "chat-ui/.env.local" "OK" $viteUrl
        }
        else {
            Write-DoctorLine "chat-ui/.env.local" "WARN" "VITE_MCP_URL absent"
        }
    }
    else {
        Write-DoctorLine "chat-ui/.env.local" "WARN" "sera cree par .\gitnexus.cmd chat"
    }
}

function Get-ChatViteProcesses {
    param(
        [string] $ChatDir,
        [int] $Port
    )

    $escapedChatDir = [regex]::Escape($ChatDir)
    Get-CimInstance Win32_Process -Filter "Name = 'node.exe'" -ErrorAction SilentlyContinue |
        Where-Object {
            $cmd = $_.CommandLine
            $cmd -and
                $cmd -match $escapedChatDir -and
                $cmd -match "vite[\\/]bin[\\/]vite\.js" -and
                $cmd -match "(^|\s)--port\s+$Port(\s|$)"
        }
}

function Get-ListeningProcessIds {
    param([int] $Port)
    try {
        @(Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue |
            Select-Object -ExpandProperty OwningProcess -Unique)
    }
    catch {
        @()
    }
}

function Stop-StaleChatViteProcesses {
    param(
        [string] $ChatDir,
        [int] $Port
    )

    $processes = @(Get-ChatViteProcesses -ChatDir $ChatDir -Port $Port)
    foreach ($proc in $processes) {
        $cmd = $proc.CommandLine
        if ($cmd -notmatch "(^|\s)--host\s+127\.0\.0\.1(\s|$)") {
            Write-Step "Arret ancien client chat GitNexus PID $($proc.ProcessId) sur le port $Port"
            Stop-Process -Id $proc.ProcessId -Force -ErrorAction SilentlyContinue
        }
    }
}

function Stop-ChatViteProcesses {
    param(
        [string] $ChatDir,
        [int] $Port
    )

    $processes = @(Get-ChatViteProcesses -ChatDir $ChatDir -Port $Port)
    foreach ($proc in $processes) {
        Write-Step "Arret client chat GitNexus PID $($proc.ProcessId) sur le port $Port"
        Stop-Process -Id $proc.ProcessId -Force -ErrorAction SilentlyContinue
    }
}

function Stop-GitNexusBackendOnPort {
    param([int] $Port)

    $pids = @(Get-ListeningProcessIds -Port $Port)
    foreach ($pidValue in $pids) {
        if (-not $pidValue -or $pidValue -eq 0) {
            continue
        }
        $proc = Get-Process -Id $pidValue -ErrorAction SilentlyContinue
        if (-not $proc) {
            continue
        }
        if ($proc.ProcessName -ne "gitnexus") {
            throw "Le port backend $Port est occupe par $($proc.ProcessName) PID $pidValue, pas par GitNexus. Arret refuse."
        }
        Write-Step "Arret backend GitNexus PID $pidValue sur le port $Port"
        Stop-Process -Id $pidValue -Force -ErrorAction SilentlyContinue
    }
}

function Write-ChatGptConfig {
    param(
        [string] $ModelName,
        [string] $Effort,
        [int] $TokenBudget
    )

    $homeDir = Resolve-HomeDir
    $configDir = Join-Path $homeDir ".gitnexus"
    $configPath = Join-Path $configDir "chat-config.json"
    New-Item -ItemType Directory -Force -Path $configDir | Out-Null

    $config = [ordered]@{
        provider = "chatgpt"
        api_key = ""
        base_url = "https://chatgpt.com/backend-api/codex"
        model = $ModelName
        max_tokens = $TokenBudget
        reasoning_effort = $Effort
    }

    Write-Utf8NoBom -Path $configPath -Lines @($config | ConvertTo-Json -Depth 4)
    Write-Host "Config ChatGPT ecrite: $configPath" -ForegroundColor Green
}

function Show-Help {
    Write-Host @"
GitNexus helper scripts

Usage:
  .\gitnexus.cmd chat                 Lance le client chat React + backend HTTP
  .\gitnexus.cmd desktop              Lance l'application desktop Tauri
  .\gitnexus.cmd login                Connexion OAuth ChatGPT
  .\gitnexus.cmd config-chatgpt       Configure ChatGPT Pro avec gpt-5.5
  .\gitnexus.cmd test-chatgpt         Teste la config et le login
  .\gitnexus.cmd ask -Question "..."  Pose une question en CLI
  .\gitnexus.cmd analyze -Repo D:\x   Indexe un repo
  .\gitnexus.cmd docs -Repo D:\x      Genere le site de doc HTML
  .\gitnexus.cmd check                Lance les validations principales
  .\gitnexus.cmd doctor               Diagnostique ports/config/login sans secret

Options utiles:
  -Repo <path>             Repo cible pour ask/analyze/docs
  -Model gpt-5.5           Modele ChatGPT
  -Reasoning high          none|minimal|low|medium|high|xhigh
  -BackendPort 3010        Port du serveur GitNexus HTTP
  -ChatPort 5176           Port du client React chat-ui
  -DesktopUiPort 1421      Port Vite de l'UI Tauri
  -NoBackend               Lance seulement le client React
  -NoBrowser               N'ouvre pas le navigateur
  -Enrich                  Active l'enrichissement LLM pour docs
  -RestartBackend          Redemarre le backend GitNexus du port backend
  -RestartChat             Redemarre le client React du port chat

Notes:
  chat reutilise un backend/client deja sain si le port repond.
  Ajoute -RestartBackend apres une modification Rust du backend.
  Ajoute -RestartChat apres une modification du client React si HMR semble bloque.
  Si un port est occupe par autre chose, le script s'arrete au lieu
  de basculer silencieusement vers un autre port.
"@
}

Set-Location -LiteralPath $RepoRoot

switch ($Command) {
    "help" {
        Show-Help
    }

    "config-chatgpt" {
        Write-ChatGptConfig -ModelName $Model -Effort $Reasoning -TokenBudget $MaxTokens
    }

    "login" {
        Require-Command "cargo"
        Invoke-GitNexusCli login
    }

    "logout" {
        Require-Command "cargo"
        Invoke-GitNexusCli logout
    }

    "test-chatgpt" {
        Require-Command "cargo"
        Invoke-GitNexusCli config test
    }

    "serve" {
        Require-Command "cargo"
        Invoke-GitNexusCli serve --port "$BackendPort"
    }

    "chat" {
        Require-Command "cargo"
        Require-Command "npm"

        if ($BackendPort -eq $ChatPort) {
            throw "BackendPort et ChatPort doivent etre differents (recu $BackendPort). Exemple: -BackendPort 3010 -ChatPort 5176."
        }

        $chatDir = Join-Path $RepoRoot "chat-ui"
        Ensure-NpmInstall -Directory $chatDir
        if ($RestartChat) {
            Stop-ChatViteProcesses -ChatDir $chatDir -Port $ChatPort
        }
        Stop-StaleChatViteProcesses -ChatDir $chatDir -Port $ChatPort

        $backendUrl = "http://127.0.0.1:$BackendPort"
        Set-DotEnvValue -Path (Join-Path $chatDir ".env.local") -Key "VITE_MCP_URL" -Value $backendUrl
        $backendHealthUrl = "$backendUrl/health"

        if (-not $NoBackend) {
            if ($RestartBackend) {
                Stop-GitNexusBackendOnPort -Port $BackendPort
            }
            if (Test-HttpOk -Url $backendHealthUrl) {
                Write-Step "Backend GitNexus deja disponible sur $backendUrl"
            }
            elseif (Test-PortListening -Port $BackendPort) {
                throw "Le port backend $BackendPort est occupe mais $backendHealthUrl ne repond pas. Ferme l'ancien processus ou choisis -BackendPort."
            }
            else {
                Write-Step "Lancement backend GitNexus HTTP sur $backendUrl"
                $backendCommand = "Set-Location -LiteralPath $(Quote-ForPowerShell $RepoRoot); cargo run -p gitnexus-cli --target-dir $(Quote-ForPowerShell $CliTargetDir) -- serve --port $BackendPort"
                Start-PowerShellWindow -Title "GitNexus backend :$BackendPort" -CommandLine $backendCommand
                if (-not (Wait-HttpOk -Url $backendHealthUrl -TimeoutSeconds 90)) {
                    throw "Le backend ne repond pas sur $backendHealthUrl. Consulte la fenetre GitNexus backend :$BackendPort."
                }
            }
        }
        elseif (-not (Test-HttpOk -Url $backendHealthUrl)) {
            Write-Host "Attention: -NoBackend est actif mais $backendHealthUrl ne repond pas." -ForegroundColor Yellow
        }

        $chatUrl = "http://127.0.0.1:$ChatPort"
        if (Test-ChatUiOk -Url $chatUrl) {
            Write-Step "Client chat React deja disponible sur $chatUrl"
        }
        elseif (Test-PortListening -Port $ChatPort) {
            throw "Le port chat $ChatPort est occupe mais $chatUrl ne sert pas le client React GitNexus. Ferme l'ancien processus ou choisis -ChatPort."
        }
        else {
            Write-Step "Lancement client chat React sur $chatUrl"
            $chatCommand = "Set-Location -LiteralPath $(Quote-ForPowerShell $chatDir); npm run dev -- --host 127.0.0.1 --port $ChatPort --strictPort"
            Start-PowerShellWindow -Title "GitNexus chat React :$ChatPort" -CommandLine $chatCommand
            $deadline = (Get-Date).AddSeconds(45)
            while ((Get-Date) -lt $deadline -and -not (Test-ChatUiOk -Url $chatUrl)) {
                Start-Sleep -Milliseconds 750
            }
            if (-not (Test-ChatUiOk -Url $chatUrl)) {
                throw "Le client React ne repond pas sur $chatUrl. Consulte la fenetre GitNexus chat React :$ChatPort."
            }
        }

        if (-not $NoBrowser) {
            Start-Process $chatUrl | Out-Null
        }
    }

    "desktop" {
        Require-Command "cargo"
        Require-Command "npm"

        $desktopUiDir = Join-Path $RepoRoot "crates\gitnexus-desktop\ui"
        Ensure-NpmInstall -Directory $desktopUiDir

        Write-Step "Lancement Vite pour l'UI desktop sur http://localhost:$DesktopUiPort"
        $uiCommand = "Set-Location -LiteralPath $(Quote-ForPowerShell $desktopUiDir); npm run dev -- --port $DesktopUiPort --strictPort"
        Start-PowerShellWindow -Title "GitNexus desktop UI :$DesktopUiPort" -CommandLine $uiCommand
        Start-Sleep -Seconds 2

        Write-Step "Lancement application desktop Tauri"
        Set-Location -LiteralPath $RepoRoot
        & cargo run -p gitnexus-desktop --target-dir $DesktopTargetDir
    }

    "ask" {
        Require-Command "cargo"
        Invoke-GitNexusCli ask "$Question" --path "$Repo"
    }

    "analyze" {
        Require-Command "cargo"
        Invoke-GitNexusCli analyze "$Repo"
    }

    "docs" {
        Require-Command "cargo"
        $args = @("generate", "html", "--path", "$Repo")
        if ($OutputDir.Trim()) {
            $args += @("--output-dir", "$OutputDir")
        }
        if ($Enrich) {
            $args += @("--enrich")
        }
        Invoke-GitNexusCli @args
    }

    "check" {
        Require-Command "cargo"
        Require-Command "npm"

        $chatDir = Join-Path $RepoRoot "chat-ui"
        $desktopUiDir = Join-Path $RepoRoot "crates\gitnexus-desktop\ui"
        Ensure-NpmInstall -Directory $chatDir
        Ensure-NpmInstall -Directory $desktopUiDir

        Write-Step "Verification chat-ui: lint, tests, build"
        Invoke-InDirectory -Directory $chatDir -Script {
            & npm run lint
            & npm run test
            & npm run build
        }

        Write-Step "Verification desktop UI: lint, build"
        Invoke-InDirectory -Directory $desktopUiDir -Script {
            & npm run lint
            & npm run build
        }

        Write-Step "Verification Rust: fmt + tests CLI/MCP/Desktop"
        Set-Location -LiteralPath $RepoRoot
        & cargo fmt --check
        & cargo test -p gitnexus-cli -p gitnexus-mcp -p gitnexus-desktop --target-dir "target-codex/check"
    }

    "doctor" {
        Show-Doctor
    }
}
