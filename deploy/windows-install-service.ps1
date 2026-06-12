<#
.SYNOPSIS
    Install the Claresso agent as a continuous background service (scheduled task)
    on a company-owned Windows machine.
.DESCRIPTION
    Idempotent. Copies the agent exe into an install dir and registers a scheduled
    task that runs `ccguard-agent --service` -- a long-running loop that captures
    Claude Code activity frequently and runs the AI triage pass once a day during an
    idle window (with catch-up if a day was missed).

    Runs AS THE LOGGED-IN USER (not SYSTEM) on purpose: the triage judge uses that
    user's already-logged-in Claude Code seat (their ~/.claude OAuth), so no separate
    API key is needed and session content never leaves the channel they authorized.
    SYSTEM has no Claude login, so a SYSTEM task could capture but never triage.

    Silent != hidden. The task and service are registered under their real name and
    are visible in Task Scheduler / Task Manager to anyone who looks. Disclosure of
    monitoring belongs in the employment agreement / AUP, not this installer.

    Catch-up + resilience: StartWhenAvailable runs a missed start as soon as possible
    (laptop was asleep/off); the task restarts on failure; no execution time limit
    (it's a loop); RunOnlyIfNetworkAvailable avoids churning with no server.
.PARAMETER ServerUrl
    Claresso server base URL the agent reports to. e.g. https://claresso.corp.example
.PARAMETER Token
    Tenant ingest token (ccg_...). Stored as a per-user environment variable and read
    by the task at runtime (not baked into the task arguments).
.PARAMETER AgentExe
    Path to the ccguard-agent.exe to install (default: .\ccguard-agent.exe next to this script).
.PARAMETER CaptureInterval
    Seconds between capture passes (default 900 = 15 min).
.PARAMETER InstallDir
    Where to copy the exe (default: C:\ProgramData\Claresso).
.PARAMETER TaskName
    Scheduled-task name (default: ClaressoAgent). Use a neutral name for a combined
    provisioning bundle if you prefer.
.EXAMPLE
    ./windows-install-service.ps1 -ServerUrl https://claresso.corp.example -Token ccg_xxx
#>
param(
    [Parameter(Mandatory = $true)] [string]$ServerUrl,
    [Parameter(Mandatory = $true)] [string]$Token,
    [string]$AgentExe = (Join-Path $PSScriptRoot "ccguard-agent.exe"),
    [int]$CaptureInterval = 900,
    [string]$InstallDir = "C:\ProgramData\Claresso",
    [string]$TaskName = "ClaressoAgent"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $AgentExe)) { throw "Agent exe not found: $AgentExe" }

# 1. Copy the agent into the install dir.
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$Dest = Join-Path $InstallDir "ccguard-agent.exe"
Copy-Item -Path $AgentExe -Destination $Dest -Force
Write-Host "Installed agent -> $Dest"

# 2. Store the ingest token as a per-user env var (read by the task at runtime, not
#    embedded in the task definition).
[Environment]::SetEnvironmentVariable("CCGUARD_TOKEN", $Token, "User")
$env:CCGUARD_TOKEN = $Token
Write-Host "Stored CCGUARD_TOKEN (user scope)."

# 3. Register the scheduled task: run --service as the logged-in user, at logon.
$User = "$env:USERDOMAIN\$env:USERNAME"
$Principal = New-ScheduledTaskPrincipal -UserId $User -LogonType Interactive -RunLevel Limited

$Action = New-ScheduledTaskAction -Execute $Dest `
    -Argument "--server $ServerUrl --token `$env:CCGUARD_TOKEN --service --capture-interval $CaptureInterval"

$Trigger = New-ScheduledTaskTrigger -AtLogOn -User $User

$Settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries `
    -StartWhenAvailable `
    -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 5) `
    -ExecutionTimeLimit ([TimeSpan]::Zero) `
    -RunOnlyIfNetworkAvailable `
    -MultipleInstances IgnoreNew

Register-ScheduledTask -TaskName $TaskName -Force `
    -Principal $Principal -Action $Action -Trigger $Trigger -Settings $Settings | Out-Null

Write-Host "Scheduled task '$TaskName' registered (runs --service at logon as $User)."

# 4. Start it now so monitoring begins without waiting for the next logon.
Start-ScheduledTask -TaskName $TaskName
Write-Host "Started '$TaskName'. Capture every $CaptureInterval s; triage once daily (idle-gated)."
Write-Host "Inspect: Get-ScheduledTask $TaskName | Get-ScheduledTaskInfo"
Write-Host "Remove:  ./windows-uninstall-service.ps1 -TaskName $TaskName"
