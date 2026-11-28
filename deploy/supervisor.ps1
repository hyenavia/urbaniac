$ErrorActionPreference = "Stop"

# Urbaniac runtime-only supervisor (extended for backend + Discord Activity frontend).
# The laptop downloads prebuilt Windows executables and prebuilt frontend-dist.zip from GitHub Releases.
# It never runs Cargo, Node.js, or builds Rust/frontend locally.

$Owner = "hyenavia"
$Repository = "urbaniac"
$Branch = "main"
$CheckInterval = 300
$RestartDelay = 10
$ActivityRecoveryBaseCooldownSec = 900
$ActivityRecoveryMaxCooldownSec = 3600

$RepoPath = "D:\Projects\urbaniac"
$EnvFile = Join-Path $RepoPath ".env"
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
$FailedCommitFile = "C:\UrbaniacDeploy\failed-commit.txt"
$BotPidFile = "C:\UrbaniacDeploy\bot.pid"

$StdoutLog = Join-Path $LogDirectory "bot-stdout.log"
$StderrLog = Join-Path $LogDirectory "bot-stderr.log"
$SupervisorLog = Join-Path $LogDirectory "supervisor.log"

$GitExe = (Get-Command git -ErrorAction Stop).Source
$script:BotProcess = $null
$script:ActivityDegradedAlerted = $false
$script:ActivityRecoveryAttempts = 0
$script:NextActivityRecoveryAt = [datetime]::MinValue
$script:ApiHeaders = @{
    "User-Agent" = "Urbaniac-Deployment-Supervisor"
    "Accept" = "application/vnd.github+json"
}

$IconOk = [char]::ConvertFromUtf32(0x2705)
$IconArt = [char]::ConvertFromUtf32(0x1F3A8)
$IconWarn = [char]::ConvertFromUtf32(0x26A0) + [char]::ConvertFromUtf32(0xFE0F)
$IconRed = [char]::ConvertFromUtf32(0x1F534)
$IconGreen = [char]::ConvertFromUtf32(0x1F7E2)

function Write-Log([string]$Message) {
    $line = "[{0}] {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $Message
    try {
        Add-Content -Path $SupervisorLog -Value $line
    }
    catch {
        # Never crash the supervisor loop if log file write encounters transient contention.
    }
}

function Get-CurrentUtc {
    return [datetime]::UtcNow
}

function Reset-ActivityRecoveryState {
    $script:ActivityDegradedAlerted = $false
    $script:ActivityRecoveryAttempts = 0
    $script:NextActivityRecoveryAt = [datetime]::MinValue
}

function Schedule-NextActivityRecovery {
    $script:ActivityRecoveryAttempts++
    $exp = [Math]::Min($script:ActivityRecoveryAttempts - 1, 4)
    $cooldownSec = [Math]::Min(
        $ActivityRecoveryMaxCooldownSec,
        [int]($ActivityRecoveryBaseCooldownSec * [Math]::Pow(2, $exp))
    )
    $nextAt = (Get-CurrentUtc).AddSeconds($cooldownSec)
    $script:NextActivityRecoveryAt = $nextAt
    Write-Log "Scheduled next controlled Activity recovery attempt #$($script:ActivityRecoveryAttempts + 1) after $cooldownSec seconds (at $($nextAt.ToString('yyyy-MM-dd HH:mm:ss')) UTC)."
}

function Send-DiscordNotification([string]$Message) {
    try {
        if (-not (Test-Path $WebhookFile)) { return }
        $url = (Get-Content $WebhookFile -Raw).Trim()
        if ([string]::IsNullOrWhiteSpace($url)) { return }
        $payload = @{ content = $Message } | ConvertTo-Json -Compress
        $utf8Body = [System.Text.Encoding]::UTF8.GetBytes($payload)
        Invoke-RestMethod -Uri $url -Method Post `
            -ContentType "application/json; charset=utf-8" `
            -Body $utf8Body -TimeoutSec 15 | Out-Null
        Write-Log "Discord notification sent."
    }
    catch {
        Write-Log "Discord notification failed: $($_.Exception.Message)"
    }
}

function Read-TrimmedFile([string]$Path) {
    if (-not (Test-Path $Path)) { return "" }
    $raw = Get-Content $Path -Raw -ErrorAction SilentlyContinue
    if ($null -eq $raw) { return "" }
    return $raw.Trim()
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

function Write-StateFile([string]$Path, [string]$Value) {
    $tmpPath = "$Path.tmp"
    Remove-Item -Path $tmpPath -Force -ErrorAction SilentlyContinue | Out-Null
    try {
        Invoke-WithRetry {
            Set-Content -Path $tmpPath -Value $Value
            Move-Item -Path $tmpPath -Destination $Path -Force
        } | Out-Null
    }
    catch {
        Remove-Item -Path $tmpPath -Force -ErrorAction SilentlyContinue | Out-Null
        throw
    }
}

function Get-DeploymentStateSnapshot {
    $paths = @(
        $DeployedCommitFile,
        $DeployedExeFile,
        $DeployedFrontendCommitFile,
        $DeployedBackendHashFile
    )
    $entries = @()
    foreach ($p in $paths) {
        $item = $null
        try {
            $item = Get-Item -LiteralPath $p -Force -ErrorAction Stop
        }
        catch [System.Management.Automation.ItemNotFoundException], [System.IO.FileNotFoundException], [System.IO.DirectoryNotFoundException] {
            $entries += [PSCustomObject]@{
                Path    = $p
                Exists  = $false
                Content = ""
            }
            continue
        }
        catch {
            throw "Failed to inspect deployment state file '$p': $($_.Exception.Message)"
        }

        if ($item.PSIsContainer) {
            throw "Deployment state path '$p' is a directory, expected a file."
        }

        try {
            $content = [System.IO.File]::ReadAllText($item.FullName)
        }
        catch {
            throw "Failed to read deployment state file '$p': $($_.Exception.Message)"
        }

        $entries += [PSCustomObject]@{
            Path    = $p
            Exists  = $true
            Content = $content
        }
    }
    return $entries
}

function Restore-DeploymentStateSnapshot($Snapshot) {
    if (-not $Snapshot) { return }
    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    foreach ($entry in $Snapshot) {
        try {
            if ($entry.Exists) {
                $needsRestore = $true
                if (Test-Path -LiteralPath $entry.Path -PathType Leaf) {
                    try {
                        $current = [System.IO.File]::ReadAllText($entry.Path)
                        if ($current -ceq $entry.Content) {
                            $needsRestore = $false
                        }
                    }
                    catch {
                        $needsRestore = $true
                    }
                }
                if ($needsRestore) {
                    $tmpPath = "$($entry.Path).tmp"
                    Remove-Item -Path $tmpPath -Force -ErrorAction SilentlyContinue | Out-Null
                    try {
                        Invoke-WithRetry {
                            [System.IO.File]::WriteAllText($tmpPath, $entry.Content, $utf8NoBom)
                            Move-Item -LiteralPath $tmpPath -Destination $entry.Path -Force
                        } | Out-Null
                    }
                    catch {
                        Remove-Item -Path $tmpPath -Force -ErrorAction SilentlyContinue | Out-Null
                        throw
                    }
                }
            }
            elseif (Test-Path -LiteralPath $entry.Path) {
                Remove-Item -LiteralPath $entry.Path -Force -ErrorAction SilentlyContinue | Out-Null
            }
        }
        catch {
            Write-Log "Warning: failed to restore state file $($entry.Path): $($_.Exception.Message)"
        }
    }
}

function Download-FileAtomic([string]$Uri, [string]$DestPath, [int]$TimeoutSec) {
    $tmpPath = "$DestPath.tmp"
    Remove-Item $tmpPath -Force -ErrorAction SilentlyContinue | Out-Null
    try {
        Invoke-WebRequest -Uri $Uri -OutFile $tmpPath -UseBasicParsing -TimeoutSec $TimeoutSec | Out-Null
        Move-Item -Path $tmpPath -Destination $DestPath -Force | Out-Null
    }
    catch {
        Remove-Item $tmpPath -Force -ErrorAction SilentlyContinue | Out-Null
        throw
    }
}

function Get-ExpectedSha256([string]$SumFile, [string]$FileName) {
    if (-not (Test-Path $SumFile)) { return $null }
    $escaped = [regex]::Escape($FileName)
    $pattern = "^\s*([0-9a-fA-F]{64})\s+\*?$escaped\s*$"
    $sumLine = Get-Content $SumFile |
        Where-Object { $_ -match $pattern } |
        Select-Object -First 1
    if (-not $sumLine -or $sumLine -notmatch $pattern) {
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

function Get-DotEnvMap([string]$Path) {
    $map = @{}
    if (-not (Test-Path -Path $Path -PathType Leaf)) {
        return $map
    }
    foreach ($rawLine in (Get-Content -Path $Path -ErrorAction SilentlyContinue)) {
        $line = $rawLine.Trim()
        if ([string]::IsNullOrWhiteSpace($line) -or $line.StartsWith("#")) {
            continue
        }
        if ($line -match "^(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)$") {
            $key = $Matches[1]
            $val = $Matches[2].Trim()
            if (($val.Length -ge 2) -and (
                ($val.StartsWith('"') -and $val.EndsWith('"')) -or
                ($val.StartsWith("'") -and $val.EndsWith("'"))
            )) {
                $val = $val.Substring(1, $val.Length - 2)
            }
            $map[$key] = $val
        }
    }
    return $map
}

function Get-ActivityHealthConfig {
    $dotEnv = Get-DotEnvMap $EnvFile

    $rawEnabled = $env:ACTIVITY_ENABLED
    if ([string]::IsNullOrWhiteSpace($rawEnabled) -and $dotEnv.ContainsKey("ACTIVITY_ENABLED")) {
        $rawEnabled = $dotEnv["ACTIVITY_ENABLED"]
    }
    $enabled = $false
    if (-not [string]::IsNullOrWhiteSpace($rawEnabled)) {
        $normEnabled = $rawEnabled.Trim().ToLowerInvariant()
        $enabled = ($normEnabled -eq "true" -or $normEnabled -eq "1")
    }

    $bindAddr = $env:ACTIVITY_BIND_ADDR
    if ([string]::IsNullOrWhiteSpace($bindAddr) -and $dotEnv.ContainsKey("ACTIVITY_BIND_ADDR")) {
        $bindAddr = $dotEnv["ACTIVITY_BIND_ADDR"]
    }
    if ([string]::IsNullOrWhiteSpace($bindAddr)) {
        $bindAddr = "127.0.0.1:8080"
    }
    else {
        $bindAddr = $bindAddr.Trim()
    }

    $probeAddr = $bindAddr
    if ($probeAddr -match "^0\.0\.0\.0:(\d+)$") {
        $probeAddr = "127.0.0.1:$($Matches[1])"
    }
    elseif ($probeAddr -match "^\[::\]:(\d+)$") {
        $probeAddr = "[::1]:$($Matches[1])"
    }
    elseif ($probeAddr -match "^\*:(\d+)$") {
        $probeAddr = "127.0.0.1:$($Matches[1])"
    }

    return [PSCustomObject]@{
        Enabled   = $enabled
        BindAddr  = $bindAddr
        HealthUrl = "http://$probeAddr/api/health"
    }
}

function Test-ActivityHealth([string]$HealthUrl, [int]$Attempts = 1, [int]$DelayMs = 1000) {
    for ($i = 1; $i -le $Attempts; $i++) {
        try {
            $resp = Invoke-RestMethod -Uri $HealthUrl -Method Get -TimeoutSec 5
            if ($resp -and ([string]$resp.status -eq "ok")) {
                return $true
            }
        }
        catch {
            # Ignore transient startup error until attempts are exhausted.
        }
        if ($i -lt $Attempts) {
            Start-Sleep -Milliseconds $DelayMs
        }
    }
    return $false
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
    $downloadDir = Join-Path $DownloadRoot $Commit

    New-Item -ItemType Directory -Force -Path $downloadDir | Out-Null

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

    if (-not $exeUrl -or -not $sumUrl -or -not $feUrl -or -not $feSumUrl -or -not $manifestUrl) {
        Write-Log "Release $short does not have all 5 required assets yet (urbaniac.exe, SHA256SUMS.txt, frontend-dist.zip, frontend-dist.zip.sha256, release-manifest.json). Keeping current deployment."
        return $null
    }

    $downloadExe = Join-Path $downloadDir "urbaniac.exe"
    $downloadSum = Join-Path $downloadDir "SHA256SUMS.txt"
    $downloadFeZip = Join-Path $downloadDir "frontend-dist.zip"
    $downloadFeSum = Join-Path $downloadDir "frontend-dist.zip.sha256"
    $downloadManifest = Join-Path $downloadDir "release-manifest.json"

    Write-Log "Downloading checksums, manifest, and release assets for commit $short."
    Download-FileAtomic $sumUrl $downloadSum 60
    Download-FileAtomic $feSumUrl $downloadFeSum 60
    Download-FileAtomic $manifestUrl $downloadManifest 60

    $expectedExeHash = Get-ExpectedSha256 $downloadSum "urbaniac.exe"

    if (-not $expectedExeHash) {
        throw [System.IO.InvalidDataException]::new("SHA256SUMS.txt has an invalid format for urbaniac.exe ($short).")
    }

    # Immutable executable directory: never overwrite a running executable.
    $stageDir = Join-Path $DeployRoot ("deploy-" + $short + "-" + $expectedExeHash.Substring(0, 12))
    $releaseDir = Join-Path $stageDir "release"
    $candidateExe = Join-Path $releaseDir "urbaniac.exe"

    $expectedFeHash = Get-ExpectedSha256 $downloadFeSum "frontend-dist.zip"
    if (-not $expectedFeHash) {
        throw [System.IO.InvalidDataException]::new("frontend-dist.zip.sha256 has an invalid format ($short).")
    }
    $sumFeHash = Get-ExpectedSha256 $downloadSum "frontend-dist.zip"
    if ($sumFeHash -and ($sumFeHash -ne $expectedFeHash)) {
        throw [System.IO.InvalidDataException]::new("Checksum mismatch between SHA256SUMS.txt and frontend-dist.zip.sha256 ($short).")
    }

    try {
        $manifest = Get-Content $downloadManifest -Raw | ConvertFrom-Json
    }
    catch {
        throw [System.IO.InvalidDataException]::new("release-manifest.json is invalid JSON for commit $short : $($_.Exception.Message)")
    }

    $manifestCommit = ([string]$manifest.commit).Trim().ToLowerInvariant()
    $manifestExeHash = ([string]$manifest.exe_sha256).Trim().ToLowerInvariant()
    $manifestFeHash = ([string]$manifest.frontend_sha256).Trim().ToLowerInvariant()
    $backendTreeHash = ([string]$manifest.backend_tree_hash).Trim().ToLowerInvariant()

    if ($manifestCommit -ne $Commit.ToLowerInvariant()) {
        throw [System.IO.InvalidDataException]::new("release-manifest.json commit mismatch for $short (got '$manifestCommit').")
    }
    if ($manifestExeHash -ne $expectedExeHash) {
        throw [System.IO.InvalidDataException]::new("release-manifest.json exe_sha256 mismatch against SHA256SUMS.txt ($short).")
    }
    if ($manifestFeHash -ne $expectedFeHash) {
        throw [System.IO.InvalidDataException]::new("release-manifest.json frontend_sha256 mismatch against frontend-dist.zip.sha256 ($short).")
    }

    if (-not (Test-Sha256 $downloadExe $expectedExeHash)) {
        Write-Log "Downloading prebuilt executable for commit $short."
        Download-FileAtomic $exeUrl $downloadExe 300
        if (-not (Test-Sha256 $downloadExe $expectedExeHash)) {
            Remove-Item $downloadExe -Force -ErrorAction SilentlyContinue | Out-Null
            throw [System.IO.InvalidDataException]::new("SHA-256 verification failed for urbaniac.exe ($short).")
        }
    }

    if (-not (Test-Sha256 $downloadFeZip $expectedFeHash)) {
        Write-Log "Downloading prebuilt frontend-dist.zip for commit $short."
        Download-FileAtomic $feUrl $downloadFeZip 180
        if (-not (Test-Sha256 $downloadFeZip $expectedFeHash)) {
            Remove-Item $downloadFeZip -Force -ErrorAction SilentlyContinue | Out-Null
            throw [System.IO.InvalidDataException]::new("SHA-256 verification failed for frontend-dist.zip ($short).")
        }
    }

    New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null
    if (-not (Test-Sha256 $candidateExe $expectedExeHash)) {
        $candidateTemp = "$candidateExe.download"
        Remove-Item $candidateTemp -Force -ErrorAction SilentlyContinue | Out-Null
        try {
            Copy-Item $downloadExe $candidateTemp -Force | Out-Null

            if (-not (Test-Sha256 $candidateTemp $expectedExeHash)) {
                throw [System.IO.InvalidDataException]::new("Candidate executable verification failed ($short).")
            }

            $candidateFullPath = [System.IO.Path]::GetFullPath($candidateExe)
            $runningAtCandidate = @(Get-BotProcesses | Where-Object {
                $_.ExecutablePath -and
                [string]::Equals(
                    [System.IO.Path]::GetFullPath($_.ExecutablePath),
                    $candidateFullPath,
                    [System.StringComparison]::OrdinalIgnoreCase
                )
            })
            if ($runningAtCandidate.Count -gt 0) {
                throw "Refusing to overwrite running executable at $candidateExe."
            }

            Invoke-WithRetry {
                Move-Item -Path $candidateTemp -Destination $candidateExe -Force | Out-Null
            } | Out-Null
        }
        catch {
            Remove-Item $candidateTemp -Force -ErrorAction SilentlyContinue | Out-Null
            throw
        }
    }

    Write-Log "Assets downloaded, manifest cross-checked, and SHA-256 verified for commit $short."

    return [PSCustomObject]@{
        Commit          = $Commit
        ExePath         = $candidateExe
        ExeHash         = $expectedExeHash
        FrontendZip     = $downloadFeZip
        FrontendHash    = $expectedFeHash
        BackendTreeHash = $backendTreeHash
    }
}

function Cleanup-FrontendStaging([string]$StagePath) {
    if ($StagePath -and (Test-Path $StagePath)) {
        Remove-Item -Path $StagePath -Recurse -Force -ErrorAction SilentlyContinue | Out-Null
    }
    Get-ChildItem -Path $FrontendStagingRoot -Directory -ErrorAction SilentlyContinue |
        ForEach-Object { Remove-Item $_.FullName -Recurse -Force -ErrorAction SilentlyContinue | Out-Null }
}

function Stage-FrontendArchive([string]$ZipPath, [string]$Commit) {
    $short = $Commit.Substring(0, 7)
    $stagePath = Join-Path $FrontendStagingRoot ("stage-" + $short + "-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
    New-Item -ItemType Directory -Force -Path $stagePath | Out-Null

    try {
        try {
            Expand-Archive -Path $ZipPath -DestinationPath $stagePath -Force | Out-Null
        }
        catch {
            throw [System.IO.InvalidDataException]::new("Failed to extract frontend-dist.zip ($short): $($_.Exception.Message)")
        }

        $stagedIndex = Join-Path $stagePath "index.html"
        if (-not (Test-Path $stagedIndex)) {
            throw [System.IO.InvalidDataException]::new("Staged frontend is missing index.html ($short).")
        }

        return $stagePath
    }
    catch {
        Cleanup-FrontendStaging -StagePath $stagePath
        throw
    }
}

function Switch-StagedFrontend([string]$StagePath, [string]$Commit) {
    $short = $Commit.Substring(0, 7)
    if (Test-Path $FrontendPreviousDir) {
        Invoke-WithRetry { Remove-Item -Path $FrontendPreviousDir -Recurse -Force | Out-Null } | Out-Null
    }

    $hadCurrent = [bool](Test-Path $FrontendCurrentDir)
    if ($hadCurrent) {
        Invoke-WithRetry { Move-Item -Path $FrontendCurrentDir -Destination $FrontendPreviousDir -Force | Out-Null } | Out-Null
    }

    try {
        Invoke-WithRetry { Move-Item -Path $StagePath -Destination $FrontendCurrentDir -Force | Out-Null } | Out-Null
    }
    catch {
        if ($hadCurrent -and (Test-Path $FrontendPreviousDir) -and -not (Test-Path $FrontendCurrentDir)) {
            try {
                Invoke-WithRetry { Move-Item -Path $FrontendPreviousDir -Destination $FrontendCurrentDir -Force | Out-Null } | Out-Null
            }
            catch {
                Write-Log "Failed to restore previous frontend after swap error: $($_.Exception.Message)"
            }
        }
        elseif ((-not $hadCurrent) -and (Test-Path $FrontendCurrentDir)) {
            Remove-Item -Path $FrontendCurrentDir -Recurse -Force -ErrorAction SilentlyContinue | Out-Null
        }
        throw
    }

    Write-Log "Frontend atomically deployed to $FrontendCurrentDir for commit $short (hadPrevious: $hadCurrent)."
    return [bool]$hadCurrent
}

function Publish-StagedFrontend([string]$ZipPath, [string]$Commit) {
    $stagePath = Stage-FrontendArchive -ZipPath $ZipPath -Commit $Commit
    try {
        return [bool](Switch-StagedFrontend -StagePath $stagePath -Commit $Commit)
    }
    finally {
        Cleanup-FrontendStaging -StagePath $stagePath
    }
}

function Restore-FrontendAfterFailure([bool]$HadCurrentBefore) {
    try {
        if ($HadCurrentBefore) {
            if (-not (Test-Path $FrontendPreviousDir)) {
                Write-Log "Cannot restore previous frontend because $FrontendPreviousDir is missing."
                return $false
            }
            if (Test-Path $FrontendCurrentDir) {
                $failedTmp = Join-Path $FrontendStagingRoot ("failed-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
                Invoke-WithRetry { Move-Item -Path $FrontendCurrentDir -Destination $failedTmp -Force | Out-Null } | Out-Null
                Remove-Item -Path $failedTmp -Recurse -Force -ErrorAction SilentlyContinue | Out-Null
            }
            Invoke-WithRetry { Move-Item -Path $FrontendPreviousDir -Destination $FrontendCurrentDir -Force | Out-Null } | Out-Null
            Write-Log "Previous frontend directory restored."
            return $true
        }

        if (Test-Path $FrontendCurrentDir) {
            $failedTmp = Join-Path $FrontendStagingRoot ("failed-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
            Invoke-WithRetry { Move-Item -Path $FrontendCurrentDir -Destination $failedTmp -Force | Out-Null } | Out-Null
            Remove-Item -Path $failedTmp -Recurse -Force -ErrorAction SilentlyContinue | Out-Null
        }
        Remove-Item -Path $DeployedFrontendCommitFile -Force -ErrorAction SilentlyContinue | Out-Null
        Write-Log "Removed first-time frontend directory to match rolled-back backend state."
        return $true
    }
    catch {
        Write-Log "Failed to roll back frontend directory: $($_.Exception.Message)"
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
    $initial = @(Get-BotProcesses)
    $hadWork = ($initial.Count -gt 0) -or ($null -ne $script:BotProcess)

    foreach ($p in $initial) {
        Write-Log "Stopping bot process PID $($p.ProcessId)."
        Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue
    }
    if ($script:BotProcess) {
        try {
            $script:BotProcess.Refresh()
            if (-not $script:BotProcess.HasExited) {
                Write-Log "Stopping tracked bot process PID $($script:BotProcess.Id)."
                Stop-Process -Id $script:BotProcess.Id -Force -ErrorAction SilentlyContinue
            }
        }
        catch {
            # Ignore already exited process handle errors.
        }
    }

    $deadline = (Get-CurrentUtc).AddSeconds(10)
    while ((Get-CurrentUtc) -lt $deadline) {
        $remaining = @(Get-BotProcesses)
        if ($remaining.Count -eq 0) {
            break
        }
        Start-Sleep -Milliseconds 500
        foreach ($p in $remaining) {
            Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue
        }
    }

    $stillAlive = @(Get-BotProcesses)
    if ($stillAlive.Count -gt 0) {
        $pids = ($stillAlive | ForEach-Object { $_.ProcessId }) -join ", "
        throw "Failed to stop existing bot process(es): PID $pids"
    }

    $script:BotProcess = $null
    Remove-Item $BotPidFile -Force -ErrorAction SilentlyContinue | Out-Null
    if ($hadWork) {
        Start-Sleep -Seconds 1
    }
}

function Start-Bot([string]$ExePath, [switch]$AllowDegradedActivity) {
    if (-not (Test-Path $ExePath)) { throw "Executable not found: $ExePath" }

    Remove-Item $StdoutLog, $StderrLog -Force -ErrorAction SilentlyContinue | Out-Null
    Write-Log "Starting bot from $ExePath."

    $script:BotProcess = Start-Process -FilePath $ExePath `
        -WorkingDirectory $RepoPath -PassThru -WindowStyle Hidden `
        -RedirectStandardOutput $StdoutLog -RedirectStandardError $StderrLog

    Set-Content -Path $BotPidFile -Value $script:BotProcess.Id
    Write-Log "Bot process created (PID $($script:BotProcess.Id))."

    $discordConnected = $false
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
            $discordConnected = $true
            break
        }
    }

    if (-not $discordConnected) {
        Stop-Process -Id $script:BotProcess.Id -Force -ErrorAction SilentlyContinue
        $script:BotProcess = $null
        throw "Discord connection verification timed out."
    }

    $activityCfg = Get-ActivityHealthConfig
    if (-not $activityCfg.Enabled) {
        Reset-ActivityRecoveryState
        return "Healthy"
    }

    $bindErrorLogged = (Test-Path $StdoutLog) -and (
        (Select-String -Path $StdoutLog -SimpleMatch "gagal bind port web Activity API" -Quiet) -or
        (Select-String -Path $StdoutLog -SimpleMatch "web Activity API berhenti dengan error" -Quiet)
    )
    $activityHealthy = $false
    if (-not $bindErrorLogged) {
        $activityHealthy = Test-ActivityHealth -HealthUrl $activityCfg.HealthUrl -Attempts 10 -DelayMs 1000
    }

    if ($activityHealthy) {
        Reset-ActivityRecoveryState
        Write-Log "Activity Web API health check passed at $($activityCfg.HealthUrl)."
        return "Healthy"
    }

    if ($AllowDegradedActivity) {
        Write-Log "WARNING: Bot connected to Discord, but Activity Web API failed at $($activityCfg.HealthUrl). Keeping bot online in degraded mode."
        if (-not $script:ActivityDegradedAlerted) {
            Send-DiscordNotification "$IconWarn **URBANIAC ACTIVITY DEGRADED**`nBot is online on Discord, but Activity Web API failed ``/api/health`` at ``$($activityCfg.BindAddr)``."
            $script:ActivityDegradedAlerted = $true
        }
        if ((Get-CurrentUtc) -ge $script:NextActivityRecoveryAt) {
            Schedule-NextActivityRecovery
        }
        return "Degraded"
    }

    Stop-Process -Id $script:BotProcess.Id -Force -ErrorAction SilentlyContinue
    $script:BotProcess = $null
    throw "Activity Web API health check failed at $($activityCfg.HealthUrl)."
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

function Test-BotAndActivityHealthy {
    if (-not (Test-BotAlive)) { return $false }

    $activityCfg = Get-ActivityHealthConfig
    if (-not $activityCfg.Enabled) {
        Reset-ActivityRecoveryState
        return $true
    }

    if (Test-ActivityHealth -HealthUrl $activityCfg.HealthUrl -Attempts 3 -DelayMs 1000) {
        if ($script:ActivityDegradedAlerted) {
            Write-Log "Activity Web API recovered at $($activityCfg.HealthUrl)."
        }
        Reset-ActivityRecoveryState
        return $true
    }

    Write-Log "Bot process is alive, but Activity Web API failed health check at $($activityCfg.HealthUrl)."
    return $false
}

function Invoke-FallbackBotRecovery {
    $exe = Read-TrimmedFile $DeployedExeFile
    if (-not $exe -or -not (Test-Path $exe)) {
        $exe = Join-Path $DeployRoot "release\urbaniac.exe"
    }
    if (-not $exe -or -not (Test-Path $exe)) {
        return "Unavailable"
    }

    try {
        return [string](Start-Bot $exe)
    }
    catch {
        Write-Log "Strict recovery failed ($($_.Exception.Message)). Retrying recovery allowing degraded Activity state."
        try {
            Stop-AllBotProcesses
        }
        catch {
            Write-Log "Warning during pre-degraded Stop-AllBotProcesses: $($_.Exception.Message)"
        }
        return [string](Start-Bot $exe -AllowDegradedActivity)
    }
}

function Invoke-ControlledHealthRecovery([switch]$IsStartup) {
    $botAlive = Test-BotAlive
    $wasDegraded = $script:ActivityDegradedAlerted
    $now = Get-CurrentUtc

    if ($botAlive) {
        if ($now -lt $script:NextActivityRecoveryAt) {
            Write-Log "Discord bot remains online in degraded mode; next controlled Activity recovery attempt at $($script:NextActivityRecoveryAt.ToString('yyyy-MM-dd HH:mm:ss')) UTC."
            return "Cooldown"
        }

        $attemptNum = $script:ActivityRecoveryAttempts + 1
        Write-Log "Activity Web API unhealthy; attempting controlled recovery restart #$attemptNum."
        try {
            Stop-AllBotProcesses
        }
        catch {
            Write-Log "Controlled recovery could not stop running bot cleanly: $($_.Exception.Message)"
            $activityCfg = Get-ActivityHealthConfig
            if (-not $script:ActivityDegradedAlerted) {
                Send-DiscordNotification "$IconWarn **URBANIAC ACTIVITY DEGRADED**`nBot is online on Discord, but Activity Web API failed ``/api/health`` at ``$($activityCfg.BindAddr)``."
                $script:ActivityDegradedAlerted = $true
            }
            if ((Get-CurrentUtc) -ge $script:NextActivityRecoveryAt) {
                Schedule-NextActivityRecovery
            }
            return "Degraded"
        }
    }
    else {
        Write-Log "Bot process is not running. Attempting recovery."
    }

    $result = Invoke-FallbackBotRecovery
    if ($result -eq "Healthy") {
        Reset-ActivityRecoveryState
        if (-not $IsStartup) {
            if ($wasDegraded -or $botAlive) {
                Send-DiscordNotification "$IconGreen **URBANIAC ONLINE**`nActivity Web API and Discord bot recovered successfully."
            }
            else {
                Send-DiscordNotification "$IconGreen **URBANIAC ONLINE**`nBot recovered successfully."
            }
        }
        return "Healthy"
    }

    if ($result -eq "Degraded") {
        if ((Get-CurrentUtc) -ge $script:NextActivityRecoveryAt) {
            Schedule-NextActivityRecovery
        }
        return "Degraded"
    }

    Write-Log "No deployed executable available for recovery. Waiting for release."
    return "Unavailable"
}

function Deploy-Commit([string]$Commit) {
    $short = $Commit.Substring(0, [Math]::Min(7, $Commit.Length))
    $failedCommit = Read-TrimmedFile $FailedCommitFile
    if ($failedCommit -and ($failedCommit -ne $Commit)) {
        Remove-Item $FailedCommitFile -Force -ErrorAction SilentlyContinue | Out-Null
        $failedShort = $failedCommit.Substring(0, [Math]::Min(7, $failedCommit.Length))
        Write-Log "Remote main advanced from failed commit $failedShort to $short; clearing failed-commit marker."
    }
    elseif ($failedCommit -eq $Commit) {
        $existingExe = Read-TrimmedFile $DeployedExeFile
        if (-not $existingExe -or -not (Test-Path $existingExe)) {
            $oldProcs = Get-BotProcesses
            if ($oldProcs.Count -gt 0) { $existingExe = $oldProcs[0].ExecutablePath }
        }
        if ((Test-BotAlive) -or ($existingExe -and (Test-Path $existingExe))) {
            Write-Log "Skipping commit $short because previous deployment attempt failed. Waiting for a new commit on $Branch."
            return $false
        }
    }

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
    $candidate = $null
    try {
        $candidate = Get-VerifiedCandidate $Commit
    }
    catch [System.IO.InvalidDataException] {
        Write-Log "Artifact validation failed for commit $short : $($_.Exception.Message)"
        Set-Content -Path $FailedCommitFile -Value $Commit
        Send-DiscordNotification "$IconWarn **URBANIAC DEPLOY SKIPPED**`nRelease artifact validation failed for commit $short.`n$($_.Exception.Message)"
        return $false
    }
    if (-not $candidate) { return $false }

    $botAlive = Test-BotAlive
    $activityCfg = Get-ActivityHealthConfig
    $activityOk = (-not $activityCfg.Enabled) -or (Test-ActivityHealth -HealthUrl $activityCfg.HealthUrl -Attempts 2 -DelayMs 500)
    $backendUnchanged = $false
    if ($botAlive -and $activityOk) {
        if ($candidate.BackendTreeHash -and $oldBackendHash -and ($candidate.BackendTreeHash -eq $oldBackendHash)) {
            $backendUnchanged = $true
        }
        elseif ($oldExe -and (Test-Sha256 $oldExe $candidate.ExeHash)) {
            $backendUnchanged = $true
        }
    }

    if ($backendUnchanged) {
        Write-Log "Backend unchanged for commit $short. Updating frontend only without stopping bot."
        $stateSnapshot = Get-DeploymentStateSnapshot
        $hadFrontendBefore = $false
        $frontendSwitched = $false
        try {
            $hadFrontendBefore = [bool](Publish-StagedFrontend $candidate.FrontendZip $Commit)
            $frontendSwitched = $true
            Write-StateFile $DeployedCommitFile $Commit
            Write-StateFile $DeployedFrontendCommitFile $Commit
            if ($candidate.BackendTreeHash) {
                Write-StateFile $DeployedBackendHashFile $candidate.BackendTreeHash
            }
            Remove-Item $FailedCommitFile -Force -ErrorAction SilentlyContinue | Out-Null
            Write-Log "Frontend-only deployment successful: $Commit"
            Send-DiscordNotification "$IconArt **URBANIAC FRONTEND UPDATE**`nActivity frontend updated atomically without restarting the bot.`nCommit: $short"
            return $true
        }
        catch [System.IO.InvalidDataException] {
            Write-Log "Frontend package validation failed for commit $short : $($_.Exception.Message)"
            Set-Content -Path $FailedCommitFile -Value $Commit
            Send-DiscordNotification "$IconWarn **URBANIAC DEPLOY SKIPPED**`nFrontend artifact validation failed for commit $short.`n$($_.Exception.Message)"
            return $false
        }
        catch {
            Write-Log "Frontend-only update failed for commit $short : $($_.Exception.Message)"
            $feRolledBack = $true
            if ($frontendSwitched) {
                $feRolledBack = Restore-FrontendAfterFailure -HadCurrentBefore $hadFrontendBefore
            }
            Restore-DeploymentStateSnapshot $stateSnapshot
            Send-DiscordNotification "$IconWarn **URBANIAC FRONTEND ROLLBACK**`nFrontend update failed after swap (rolled back: $feRolledBack). Bot stays online.`nCommit: $short"
            return $false
        }
    }

    # Stage and verify the frontend archive first, while keeping both the running bot and current frontend untouched.
    $stagePath = $null
    try {
        $stagePath = Stage-FrontendArchive -ZipPath $candidate.FrontendZip -Commit $Commit
    }
    catch [System.IO.InvalidDataException] {
        Write-Log "Frontend package validation failed for commit $short : $($_.Exception.Message)"
        Set-Content -Path $FailedCommitFile -Value $Commit
        Send-DiscordNotification "$IconWarn **URBANIAC DEPLOY SKIPPED**`nFrontend artifact validation failed for commit $short.`n$($_.Exception.Message)"
        return $false
    }

    try {
        # Capture state snapshot before stopping the running bot so snapshot read errors never stop the live bot.
        $stateSnapshot = Get-DeploymentStateSnapshot

        # Stop the old backend before activating the new frontend so a stop failure never pairs new frontend with old backend.
        try {
            Stop-AllBotProcesses
        }
        catch {
            Write-Log "Aborting deployment of commit $short because existing bot process could not be stopped cleanly: $($_.Exception.Message)"
            $reAdopted = Adopt-RunningBot
            if (-not $reAdopted -and -not (Test-BotAlive) -and $oldExe -and (Test-Path $oldExe)) {
                try {
                    $null = Start-Bot $oldExe -AllowDegradedActivity
                }
                catch {
                    Write-Log "Recovery start after failed stop also failed: $($_.Exception.Message)"
                }
            }
            Send-DiscordNotification "$IconWarn **URBANIAC DEPLOY ABORTED**`nExisting bot process could not be stopped cleanly for commit $short; existing frontend was kept untouched."
            return $false
        }

        $hadFrontendBefore = $false
        $frontendSwitched = $false
        $candidateStarted = $false
        try {
            $hadFrontendBefore = [bool](Switch-StagedFrontend -StagePath $stagePath -Commit $Commit)
            $frontendSwitched = $true
            $null = Start-Bot $candidate.ExePath
            $candidateStarted = $true

            Write-StateFile $DeployedCommitFile $Commit
            Write-StateFile $DeployedExeFile $candidate.ExePath
            Write-StateFile $DeployedFrontendCommitFile $Commit
            if ($candidate.BackendTreeHash) {
                Write-StateFile $DeployedBackendHashFile $candidate.BackendTreeHash
            }
            Remove-Item $FailedCommitFile -Force -ErrorAction SilentlyContinue | Out-Null
            Write-Log "Deployment successful (backend + frontend): $Commit"
            Send-DiscordNotification "$IconOk **URBANIAC UPDATE SUCCESS**`nPrebuilt Windows executable and Activity frontend verified and deployed.`nCommit: $short"
            return $true
        }
        catch {
            Write-Log "Deployment step failed for commit $short : $($_.Exception.Message)"
            try {
                Stop-AllBotProcesses
            }
            catch {
                Write-Log "Warning: failed to stop process during rollback: $($_.Exception.Message)"
            }

            $feRestored = $true
            if ($frontendSwitched) {
                $feRestored = Restore-FrontendAfterFailure -HadCurrentBefore $hadFrontendBefore
            }
            Restore-DeploymentStateSnapshot $stateSnapshot

            if (-not $candidateStarted) {
                Set-Content -Path $FailedCommitFile -Value $Commit
            }

            if ($oldExe -and (Test-Path $oldExe)) {
                try {
                    Write-Log "Starting previous executable as fallback (frontend rolled back: $feRestored)."
                    $null = Start-Bot $oldExe -AllowDegradedActivity
                    Send-DiscordNotification "$IconWarn **URBANIAC FALLBACK**`nNew version failed. Previous executable and frontend state restored.`nSkipped commit: $short"
                }
                catch {
                    Write-Log "Fallback startup failed: $($_.Exception.Message)"
                    Send-DiscordNotification "$IconRed **URBANIAC OFFLINE**`nNew version and fallback both failed. Inspect laptop logs."
                }
            }
            else {
                Write-Log "No previous executable available for fallback."
                Send-DiscordNotification "$IconRed **URBANIAC OFFLINE**`nNew version failed and no previous executable is available."
            }
            return $false
        }
    }
    finally {
        Cleanup-FrontendStaging -StagePath $stagePath
    }
}

# STARTUP
New-Item -ItemType Directory -Force -Path $LogDirectory, $DownloadRoot, $FrontendRoot, $FrontendStagingRoot | Out-Null
if (-not (Test-Path -Path $RepoPath -PathType Container)) {
    Write-Log "Repository working directory not found: $RepoPath"
    Send-DiscordNotification "$IconRed **URBANIAC STARTUP ERROR**`nRepository working directory not found: $RepoPath"
    throw "Repository working directory not found: $RepoPath"
}
New-Item -ItemType Directory -Force -Path $DeployRoot | Out-Null

Write-Log "Urbaniac runtime-only supervisor starting. Cargo and Node builds are disabled."

try {
    $hasBot = Adopt-RunningBot
    $remoteCommit = Get-RemoteCommit
    Write-Log "GitHub main commit: $remoteCommit"

    $deployed = Deploy-Commit $remoteCommit
    $startupHealthy = Test-BotAndActivityHealthy
    if (-not $deployed -and -not $startupHealthy) {
        $rec = Invoke-ControlledHealthRecovery -IsStartup
        $startupHealthy = ($rec -eq "Healthy")
        if ($rec -eq "Unavailable" -and -not $hasBot) {
            Write-Log "No running bot or previously deployed executable is available yet. Waiting in supervisor loop."
        }
    }

    if ($startupHealthy) {
        Send-DiscordNotification "$IconGreen **URBANIAC ONLINE**`nRuntime-only deployment supervisor is active."
    }
}
catch {
    Write-Log "Startup cycle error (supervisor remains active): $($_.Exception.Message)"
    Send-DiscordNotification "$IconWarn **URBANIAC STARTUP WARNING**`n$($_.Exception.Message)`nSupervisor remains active and will retry."
    if (-not (Test-BotAndActivityHealthy)) {
        try {
            $rec = Invoke-ControlledHealthRecovery -IsStartup
            if ($rec -eq "Healthy") {
                Send-DiscordNotification "$IconGreen **URBANIAC ONLINE**`nRecovered previous executable after startup warning."
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
        $deployed = $false
        try {
            $remoteCommit = Get-RemoteCommit
            Write-Log "Checked GitHub main: $remoteCommit"
            $deployed = Deploy-Commit $remoteCommit
        }
        catch {
            Write-Log "GitHub check/deploy failed: $($_.Exception.Message)"
            Send-DiscordNotification "$IconWarn **URBANIAC SUPERVISOR WARNING**`n$($_.Exception.Message)"
        }

        if (-not $deployed -and -not (Test-BotAndActivityHealthy)) {
            $null = Invoke-ControlledHealthRecovery
        }
    }
    catch {
        Write-Log "Supervisor cycle failed: $($_.Exception.Message)"
        Send-DiscordNotification "$IconWarn **URBANIAC SUPERVISOR WARNING**`n$($_.Exception.Message)"
    }
}
