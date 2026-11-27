$ErrorActionPreference = "Stop"

# Urbaniac runtime-only supervisor (extended for backend + Discord Activity frontend).
# The laptop downloads prebuilt Windows executables and prebuilt frontend-dist.zip from GitHub Releases.
# It never runs Cargo, Node.js, or builds Rust/frontend locally.

$Owner = "hyenavia"
$Repository = "urbaniac"
$Branch = "main"
$CheckInterval = 300
$RestartDelay = 10

$RepoPath = "D:\Projects\urbaniac"
$DeployRoot = Join-Path $RepoPath "target"
$LogDirectory = "C:\UrbaniacDeploy\logs"
$DownloadRoot = "C:\UrbaniacDeploy\downloads"

$FrontendRoot = "C:\UrbaniacDeploy\frontend"
$FrontendCurrentDir = Join-Path $FrontendRoot "current"
$FrontendPreviousDir = Join-Path $FrontendRoot "previous"
$FrontendStagingRoot = Join-Path $FrontendRoot "staging"

$WebhookFile = "C:\UrbaniacDeploy\webhook-url.txt"
$DeployedCommitFile = "C:\UrbaniacDeploy\deployed-commit.txt"
$DeployedExeFile = "C:\UrbaniacDeploy\deployed-exe.txt"
$DeployedBackendHashFile = "C:\UrbaniacDeploy\deployed-backend-hash.txt"
$DeployedFrontendCommitFile = "C:\UrbaniacDeploy\deployed-frontend-commit.txt"
$BotPidFile = "C:\UrbaniacDeploy\bot.pid"

$StdoutLog = Join-Path $LogDirectory "bot-stdout.log"
$StderrLog = Join-Path $LogDirectory "bot-stderr.log"
$SupervisorLog = Join-Path $LogDirectory "supervisor.log"

$GitExe = (Get-Command git -ErrorAction Stop).Source
$script:BotProcess = $null
$script:ApiHeaders = @{
    "User-Agent" = "Urbaniac-Deployment-Supervisor"
    "Accept" = "application/vnd.github+json"
}

New-Item -ItemType Directory -Force -Path $LogDirectory, $DownloadRoot, $DeployRoot, $FrontendRoot, $FrontendStagingRoot | Out-Null

function Write-Log([string]$Message) {
    $line = "[{0}] {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $Message
    try {
        Add-Content -Path $SupervisorLog -Value $line
    }
    catch {
        # Never crash the supervisor loop if log file write encounters transient contention.
    }
}

function Send-DiscordNotification([string]$Message) {
    try {
        if (-not (Test-Path $WebhookFile)) { return }
        $url = (Get-Content $WebhookFile -Raw).Trim()
        if ([string]::IsNullOrWhiteSpace($url)) { return }
        $payload = @{ content = $Message } | ConvertTo-Json -Compress
        Invoke-RestMethod -Uri $url -Method Post `
            -ContentType "application/json; charset=utf-8" `
            -Body $payload -TimeoutSec 15 | Out-Null
        Write-Log "Discord notification sent."
    }
    catch {
        Write-Log "Discord notification failed: $($_.Exception.Message)"
    }
}

function Read-TrimmedFile([string]$Path) {
    if (-not (Test-Path $Path)) { return "" }
    return (Get-Content $Path -Raw -ErrorAction SilentlyContinue).Trim()
}

function Invoke-WithRetry([scriptblock]$Action, [int]$Attempts = 3, [int]$DelayMs = 200) {
    for ($i = 1; $i -le $Attempts; $i++) {
        try {
            & $Action
            return
        }
        catch {
            if ($i -eq $Attempts) { throw }
            Start-Sleep -Milliseconds $DelayMs
        }
    }
}

function Download-FileAtomic([string]$Uri, [string]$DestPath, [int]$TimeoutSec) {
    $tmpPath = "$DestPath.tmp"
    Remove-Item $tmpPath -Force -ErrorAction SilentlyContinue
    try {
        Invoke-WebRequest -Uri $Uri -OutFile $tmpPath -UseBasicParsing -TimeoutSec $TimeoutSec
        Move-Item -Path $tmpPath -Destination $DestPath -Force
    }
    catch {
        Remove-Item $tmpPath -Force -ErrorAction SilentlyContinue
        throw
    }
}

function Get-ExpectedSha256([string]$SumFile, [string]$FileName) {
    if (-not (Test-Path $SumFile)) { return $null }
    $escaped = [regex]::Escape($FileName)
    $sumLine = Get-Content $SumFile |
        Where-Object { $_ -match $escaped } |
        Select-Object -First 1
    if (-not $sumLine -or $sumLine -notmatch "^\s*([0-9a-fA-F]{64})\s+\*?$escaped\s*$") {
        return $null
    }
    return $Matches[1].ToLowerInvariant()
}

function Test-Sha256([string]$FilePath, [string]$ExpectedHash) {
    if (-not (Test-Path $FilePath) -or [string]::IsNullOrWhiteSpace($ExpectedHash)) {
        return $false
    }
    $actual = (Get-FileHash $FilePath -Algorithm SHA256).Hash.ToLowerInvariant()
    return ($actual -eq $ExpectedHash.ToLowerInvariant())
}

function Get-RemoteCommit {
    $url = "https://github.com/$Owner/$Repository.git"
    $output = & $GitExe ls-remote $url "refs/heads/$Branch" 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to check GitHub main: $($output -join ' ')"
    }
    $line = @($output | Where-Object { "$_" -match "^[0-9a-f]{40}\s+refs/heads/$Branch$" }) | Select-Object -First 1
    if (-not $line) { throw "GitHub did not return the main branch commit." }
    return (($line -split "\s+")[0]).Trim()
}

function Get-ReleaseAssets([string]$Commit) {
    $tag = "urbaniac-$Commit"
    $uri = "https://api.github.com/repos/$Owner/$Repository/releases/tags/$tag"
    try {
        return Invoke-RestMethod -Uri $uri -Headers $script:ApiHeaders -Method Get -TimeoutSec 30
    }
    catch {
        $status = $null
        if ($_.Exception.Response) { $status = [int]$_.Exception.Response.StatusCode }
        if ($status -eq 404) { return $null }
        throw "GitHub Release lookup failed: $($_.Exception.Message)"
    }
}

function Get-AssetUrl($Release, [string]$Name) {
    $asset = @($Release.assets | Where-Object { $_.name -eq $Name } | Select-Object -First 1)
    if ($asset.Count -eq 0) { return $null }
    return $asset[0].browser_download_url
}

function Get-VerifiedCandidate([string]$Commit) {
    $short = $Commit.Substring(0, 7)
    $stageDir = Join-Path $DeployRoot ("deploy-" + $short)
    $releaseDir = Join-Path $stageDir "release"
    $candidateExe = Join-Path $releaseDir "urbaniac.exe"
    $downloadDir = Join-Path $DownloadRoot $Commit

    New-Item -ItemType Directory -Force -Path $releaseDir, $downloadDir | Out-Null

    $release = Get-ReleaseAssets $Commit
    if (-not $release) {
        Write-Log "Release for commit $short is not published yet. Keeping current deployment."
        return $null
    }

    $exeUrl = Get-AssetUrl $release "urbaniac.exe"
    $sumUrl = Get-AssetUrl $release "SHA256SUMS.txt"
    $feUrl = Get-AssetUrl $release "frontend-dist.zip"
    $feSumUrl = Get-AssetUrl $release "frontend-dist.zip.sha256"
    $manifestUrl = Get-AssetUrl $release "release-manifest.json"

    if (-not $exeUrl -or -not $sumUrl -or -not $feUrl -or -not $feSumUrl) {
        Write-Log "Release $short does not have all required assets (urbaniac.exe, SHA256SUMS.txt, frontend-dist.zip, frontend-dist.zip.sha256) yet. Keeping current deployment."
        return $null
    }

    $downloadExe = Join-Path $downloadDir "urbaniac.exe"
    $downloadSum = Join-Path $downloadDir "SHA256SUMS.txt"
    $downloadFeZip = Join-Path $downloadDir "frontend-dist.zip"
    $downloadFeSum = Join-Path $downloadDir "frontend-dist.zip.sha256"
    $downloadManifest = Join-Path $downloadDir "release-manifest.json"

    Write-Log "Downloading checksums and release assets for commit $short."
    Download-FileAtomic $sumUrl $downloadSum 60
    Download-FileAtomic $feSumUrl $downloadFeSum 60

    $expectedExeHash = Get-ExpectedSha256 $downloadSum "urbaniac.exe"
    if (-not $expectedExeHash) {
        throw "SHA256SUMS.txt has an invalid format for urbaniac.exe."
    }

    $expectedFeHash = Get-ExpectedSha256 $downloadFeSum "frontend-dist.zip"
    if (-not $expectedFeHash) {
        $expectedFeHash = Get-ExpectedSha256 $downloadSum "frontend-dist.zip"
    }
    if (-not $expectedFeHash) {
        throw "frontend-dist.zip.sha256 has an invalid format."
    }

    if (-not (Test-Sha256 $downloadExe $expectedExeHash)) {
        Write-Log "Downloading prebuilt executable for commit $short."
        Download-FileAtomic $exeUrl $downloadExe 300
        if (-not (Test-Sha256 $downloadExe $expectedExeHash)) {
            Remove-Item $downloadExe -Force -ErrorAction SilentlyContinue
            throw "SHA-256 verification failed for urbaniac.exe ($short)."
        }
    }

    if (-not (Test-Sha256 $downloadFeZip $expectedFeHash)) {
        Write-Log "Downloading prebuilt frontend-dist.zip for commit $short."
        Download-FileAtomic $feUrl $downloadFeZip 180
        if (-not (Test-Sha256 $downloadFeZip $expectedFeHash)) {
            Remove-Item $downloadFeZip -Force -ErrorAction SilentlyContinue
            throw "SHA-256 verification failed for frontend-dist.zip ($short)."
        }
    }

    $backendTreeHash = ""
    if ($manifestUrl) {
        try {
            Download-FileAtomic $manifestUrl $downloadManifest 60
            $manifest = Get-Content $downloadManifest -Raw | ConvertFrom-Json
            if ($manifest.backend_tree_hash) {
                $backendTreeHash = [string]$manifest.backend_tree_hash
            }
        }
        catch {
            Write-Log "Optional manifest read failed for $short : $($_.Exception.Message)"
        }
    }

    Copy-Item $downloadExe $candidateExe -Force
    Write-Log "Assets downloaded and SHA-256 verified for commit $short."

    return [PSCustomObject]@{
        Commit          = $Commit
        ExePath         = $candidateExe
        ExeHash         = $expectedExeHash
        FrontendZip     = $downloadFeZip
        FrontendHash    = $expectedFeHash
        BackendTreeHash = $backendTreeHash
    }
}

function Publish-StagedFrontend([string]$ZipPath, [string]$Commit) {
    $short = $Commit.Substring(0, 7)
    $stagePath = Join-Path $FrontendStagingRoot ("stage-" + $short + "-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
    New-Item -ItemType Directory -Force -Path $stagePath | Out-Null

    try {
        Expand-Archive -Path $ZipPath -DestinationPath $stagePath -Force
        $stagedIndex = Join-Path $stagePath "index.html"
        if (-not (Test-Path $stagedIndex)) {
            throw "Staged frontend is missing index.html."
        }

        if (Test-Path $FrontendPreviousDir) {
            Invoke-WithRetry { Remove-Item -Path $FrontendPreviousDir -Recurse -Force }
        }

        $hadCurrent = Test-Path $FrontendCurrentDir
        if ($hadCurrent) {
            Invoke-WithRetry { Move-Item -Path $FrontendCurrentDir -Destination $FrontendPreviousDir -Force }
        }

        try {
            Invoke-WithRetry { Move-Item -Path $stagePath -Destination $FrontendCurrentDir -Force }
        }
        catch {
            if ($hadCurrent -and (Test-Path $FrontendPreviousDir) -and -not (Test-Path $FrontendCurrentDir)) {
                try {
                    Invoke-WithRetry { Move-Item -Path $FrontendPreviousDir -Destination $FrontendCurrentDir -Force }
                }
                catch {
                    Write-Log "Failed to restore previous frontend after swap error: $($_.Exception.Message)"
                }
            }
            throw
        }

        Write-Log "Frontend atomically deployed to $FrontendCurrentDir for commit $short."
    }
    finally {
        if (Test-Path $stagePath) {
            Remove-Item -Path $stagePath -Recurse -Force -ErrorAction SilentlyContinue
        }
        # Clean up older leftover staging folders, keeping staging root clean.
        Get-ChildItem -Path $FrontendStagingRoot -Directory -ErrorAction SilentlyContinue |
            ForEach-Object { Remove-Item $_.FullName -Recurse -Force -ErrorAction SilentlyContinue }
    }
}

function Restore-PreviousFrontend {
    if (-not (Test-Path $FrontendPreviousDir)) { return $false }
    try {
        if (Test-Path $FrontendCurrentDir) {
            $failedTmp = Join-Path $FrontendStagingRoot ("failed-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
            Invoke-WithRetry { Move-Item -Path $FrontendCurrentDir -Destination $failedTmp -Force }
            Remove-Item -Path $failedTmp -Recurse -Force -ErrorAction SilentlyContinue
        }
        Invoke-WithRetry { Move-Item -Path $FrontendPreviousDir -Destination $FrontendCurrentDir -Force }
        Write-Log "Previous frontend directory restored."
        return $true
    }
    catch {
        Write-Log "Failed to restore previous frontend: $($_.Exception.Message)"
        return $false
    }
}

function Get-BotProcesses {
    $root = [System.IO.Path]::GetFullPath($DeployRoot).TrimEnd('\') + '\'
    return @(
        Get-CimInstance Win32_Process |
            Where-Object {
                $_.Name -eq "urbaniac.exe" -and
                $_.ExecutablePath -and
                $_.ExecutablePath.StartsWith($root, [System.StringComparison]::OrdinalIgnoreCase)
            }
    )
}

function Stop-AllBotProcesses {
    foreach ($p in (Get-BotProcesses)) {
        Write-Log "Stopping bot process PID $($p.ProcessId)."
        Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue
    }
    $script:BotProcess = $null
    Remove-Item $BotPidFile -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
}

function Start-Bot([string]$ExePath) {
    if (-not (Test-Path $ExePath)) { throw "Executable not found: $ExePath" }

    Remove-Item $StdoutLog, $StderrLog -Force -ErrorAction SilentlyContinue
    Write-Log "Starting bot from $ExePath."

    $script:BotProcess = Start-Process -FilePath $ExePath `
        -WorkingDirectory $RepoPath -PassThru -WindowStyle Hidden `
        -RedirectStandardOutput $StdoutLog -RedirectStandardError $StderrLog

    Set-Content -Path $BotPidFile -Value $script:BotProcess.Id
    Write-Log "Bot process created (PID $($script:BotProcess.Id))."

    for ($i = 0; $i -lt 45; $i++) {
        Start-Sleep -Seconds 2
        $script:BotProcess.Refresh()

        if ($script:BotProcess.HasExited) {
            $exitCode = $script:BotProcess.ExitCode
            $script:BotProcess = $null
            throw "Bot exited during startup. Exit code: $exitCode"
        }

        if ((Test-Path $StdoutLog) -and
            (Select-String -Path $StdoutLog -SimpleMatch "bot terhubung" -Quiet)) {
            Write-Log "Bot successfully connected to Discord."
            return
        }
    }

    Stop-Process -Id $script:BotProcess.Id -Force -ErrorAction SilentlyContinue
    $script:BotProcess = $null
    throw "Discord connection verification timed out."
}

function Adopt-RunningBot {
    $processes = Get-BotProcesses
    if ($processes.Count -gt 0) {
        $p = $processes[0]
        $proc = Get-Process -Id $p.ProcessId -ErrorAction SilentlyContinue
        if ($proc -and -not $proc.HasExited) {
            $script:BotProcess = $proc
            Set-Content -Path $BotPidFile -Value $p.ProcessId
            Write-Log "Adopted existing bot process PID $($p.ProcessId)."
            return $true
        }
    }
    return $false
}

function Test-BotAlive {
    if (-not $script:BotProcess) { return $false }
    try {
        $script:BotProcess.Refresh()
        if ($script:BotProcess.HasExited) {
            $script:BotProcess = $null
            return $false
        }
        return $true
    }
    catch {
        $script:BotProcess = $null
        return $false
    }
}

function Deploy-Commit([string]$Commit) {
    $deployedCommit = Read-TrimmedFile $DeployedCommitFile
    $deployedFeCommit = Read-TrimmedFile $DeployedFrontendCommitFile
    $frontendReady = Test-Path (Join-Path $FrontendCurrentDir "index.html")

    if ($Commit -eq $deployedCommit -and $Commit -eq $deployedFeCommit -and $frontendReady) {
        return $false
    }

    $oldExe = Read-TrimmedFile $DeployedExeFile
    if (-not $oldExe -or -not (Test-Path $oldExe)) {
        $oldProcesses = Get-BotProcesses
        if ($oldProcesses.Count -gt 0) { $oldExe = $oldProcesses[0].ExecutablePath }
    }
    $oldBackendHash = Read-TrimmedFile $DeployedBackendHashFile

    Write-Log "New deployment requested: $Commit"
    $candidate = Get-VerifiedCandidate $Commit
    if (-not $candidate) { return $false }

    $botAlive = Test-BotAlive
    $backendUnchanged = $false
    if ($botAlive) {
        if ($candidate.BackendTreeHash -and $oldBackendHash -and ($candidate.BackendTreeHash -eq $oldBackendHash)) {
            $backendUnchanged = $true
        }
        elseif ($oldExe -and (Test-Sha256 $oldExe $candidate.ExeHash)) {
            $backendUnchanged = $true
        }
    }

    if ($backendUnchanged) {
        Write-Log "Backend unchanged for commit $($Commit.Substring(0,7)). Updating frontend only without stopping bot."
        Publish-StagedFrontend $candidate.FrontendZip $Commit
        Set-Content -Path $DeployedCommitFile -Value $Commit
        Set-Content -Path $DeployedFrontendCommitFile -Value $Commit
        if ($candidate.BackendTreeHash) {
            Set-Content -Path $DeployedBackendHashFile -Value $candidate.BackendTreeHash
        }
        Write-Log "Frontend-only deployment successful: $Commit"
        Send-DiscordNotification "🎨 **URBANIAC FRONTEND UPDATE**`nActivity frontend updated atomically without restarting the bot.`nCommit: $($Commit.Substring(0,7))"
        return $true
    }

    # Deploy staged frontend before switching the backend executable so both share the exact same commit.
    Publish-StagedFrontend $candidate.FrontendZip $Commit

    # Keep the currently running bot online throughout download and verification.
    Stop-AllBotProcesses
    try {
        Start-Bot $candidate.ExePath
        Set-Content -Path $DeployedCommitFile -Value $Commit
        Set-Content -Path $DeployedExeFile -Value $candidate.ExePath
        Set-Content -Path $DeployedFrontendCommitFile -Value $Commit
        if ($candidate.BackendTreeHash) {
            Set-Content -Path $DeployedBackendHashFile -Value $candidate.BackendTreeHash
        }
        Write-Log "Deployment successful (backend + frontend): $Commit"
        Send-DiscordNotification "✅ **URBANIAC UPDATE SUCCESS**`nPrebuilt Windows executable and Activity frontend verified and deployed.`nCommit: $($Commit.Substring(0,7))"
        return $true
    }
    catch {
        Write-Log "New version startup failed: $($_.Exception.Message)"
        Stop-AllBotProcesses
        $feRestored = Restore-PreviousFrontend
        if ($oldExe -and (Test-Path $oldExe)) {
            try {
                Write-Log "Starting previous executable as fallback (frontend restored: $feRestored)."
                Start-Bot $oldExe
                Send-DiscordNotification "⚠️ **URBANIAC FALLBACK**`nNew version failed. Previous executable and frontend restored.`nAttempted commit: $($Commit.Substring(0,7))"
            }
            catch {
                Write-Log "Fallback startup failed: $($_.Exception.Message)"
                Send-DiscordNotification "🔴 **URBANIAC OFFLINE**`nNew version and fallback both failed. Inspect laptop logs."
            }
        }
        else {
            Write-Log "No previous executable available for fallback."
            Send-DiscordNotification "🔴 **URBANIAC OFFLINE**`nNew version failed and no previous executable is available."
        }
        return $false
    }
}

function Invoke-FallbackBotRecovery {
    $exe = Read-TrimmedFile $DeployedExeFile
    if (-not $exe -or -not (Test-Path $exe)) {
        $exe = Join-Path $DeployRoot "release\urbaniac.exe"
    }
    if ($exe -and (Test-Path $exe)) {
        Start-Bot $exe
        return $true
    }
    return $false
}

# STARTUP
if (-not (Test-Path $RepoPath)) {
    Write-Log "Repository working directory not found: $RepoPath"
    throw "Repository working directory not found: $RepoPath"
}
Write-Log "Urbaniac runtime-only supervisor starting. Cargo and Node builds are disabled."

try {
    $hasBot = Adopt-RunningBot
    $remoteCommit = Get-RemoteCommit
    Write-Log "GitHub main commit: $remoteCommit"

    $deployed = Deploy-Commit $remoteCommit
    if (-not $deployed -and -not (Test-BotAlive)) {
        if (-not (Invoke-FallbackBotRecovery) -and -not $hasBot) {
            Write-Log "No running bot or previously deployed executable is available yet. Waiting in supervisor loop."
        }
    }

    if (Test-BotAlive) {
        Send-DiscordNotification "🟢 **URBANIAC ONLINE**`nRuntime-only deployment supervisor is active."
    }
}
catch {
    Write-Log "Startup cycle error (supervisor remains active): $($_.Exception.Message)"
    Send-DiscordNotification "⚠️ **URBANIAC STARTUP WARNING**`n$($_.Exception.Message)`nSupervisor remains active and will retry."
    if (-not (Test-BotAlive)) {
        try {
            if (Invoke-FallbackBotRecovery) {
                Send-DiscordNotification "🟢 **URBANIAC ONLINE**`nRecovered previous executable after startup warning."
            }
        }
        catch {
            Write-Log "Startup fallback recovery failed: $($_.Exception.Message)"
        }
    }
}

# SUPERVISOR LOOP
while ($true) {
    Start-Sleep -Seconds $CheckInterval
    try {
        if (-not (Test-BotAlive)) {
            Write-Log "Bot process is not running."
        }

        $remoteCommit = Get-RemoteCommit
        Write-Log "Checked GitHub main: $remoteCommit"
        $deployed = Deploy-Commit $remoteCommit

        if (-not $deployed -and -not (Test-BotAlive)) {
            Write-Log "Attempting bot recovery."
            if (Invoke-FallbackBotRecovery) {
                Send-DiscordNotification "🟢 **URBANIAC ONLINE**`nBot recovered successfully."
            }
            else {
                Write-Log "No deployed executable available for recovery. Waiting for release."
            }
        }
    }
    catch {
        Write-Log "Supervisor cycle failed: $($_.Exception.Message)"
        Send-DiscordNotification "⚠️ **URBANIAC SUPERVISOR WARNING**`n$($_.Exception.Message)"
    }
}
