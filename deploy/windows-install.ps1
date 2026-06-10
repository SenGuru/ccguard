<#
.SYNOPSIS
    Deploy the CCGuard managed-settings.json policy and schedule the agent.
.DESCRIPTION
    Idempotent. Copies the generated managed-settings.json to the enterprise
    path, hardens its ACL so non-admins cannot modify it, and registers a SYSTEM
    scheduled task that attests hourly and captures at logon.
.PARAMETER PolicyJson
    Path to the managed-settings.json produced by `ccguard-agent gen-policy`.
.PARAMETER ServerUrl
    CCGuard server base URL the agent attests/captures to.
.PARAMETER AgentPath
    Path to the ccguard-agent executable (default: assumes it's on PATH).
.EXAMPLE
    ./windows-install.ps1 -PolicyJson .\managed-settings.json -ServerUrl https://ccguard.corp.example
#>
param(
    [Parameter(Mandatory = $true)] [string]$PolicyJson,
    [Parameter(Mandatory = $true)] [string]$ServerUrl,
    [string]$AgentPath = "ccguard-agent"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $PolicyJson)) { throw "Policy file not found: $PolicyJson" }

$Dir  = "C:\ProgramData\ClaudeCode"
$Dest = Join-Path $Dir "managed-settings.json"

# 1. Install the managed settings (highest-precedence enterprise policy).
New-Item -ItemType Directory -Force -Path $Dir | Out-Null
Copy-Item -Path $PolicyJson -Destination $Dest -Force
Write-Host "Installed $Dest"

# 2. Harden the ACL: SYSTEM + Administrators full control, Users read-only.
#    Non-admins can READ the policy (Claude Code must load it) but cannot MODIFY
#    it, so the policy cannot be tampered with from a standard user account.
icacls $Dest /inheritance:r `
    /grant:r "SYSTEM:(F)" `
    /grant:r "BUILTIN\Administrators:(F)" `
    /grant:r "BUILTIN\Users:(RX)" | Out-Null
Write-Host "ACL hardened: Users have read-only access"

# Registry alternative (instead of the JSON file) — write the same policy under:
#   HKLM\SOFTWARE\Policies\ClaudeCode  (REG_SZ value 'Settings' = the JSON blob)
# Useful when pushing via Group Policy Preferences instead of a file drop.

# 3. Register the SYSTEM scheduled task: attest hourly + capture at logon.
$Token = $env:CCGUARD_TOKEN
if ([string]::IsNullOrEmpty($Token)) {
    Write-Warning "CCGUARD_TOKEN not set in this session; the task uses the machine env var at runtime."
}

$TaskName = "CCGuard-Agent"
$Principal = New-ScheduledTaskPrincipal -UserId "SYSTEM" -LogonType ServiceAccount -RunLevel Highest

$AttestAction = New-ScheduledTaskAction -Execute $AgentPath `
    -Argument "--server $ServerUrl --token `$env:CCGUARD_TOKEN --attest"
$CaptureAction = New-ScheduledTaskAction -Execute $AgentPath `
    -Argument "--server $ServerUrl --token `$env:CCGUARD_TOKEN --capture"

$Hourly = New-ScheduledTaskTrigger -Once -At (Get-Date) `
    -RepetitionInterval (New-TimeSpan -Hours 1) -RepetitionDuration ([TimeSpan]::MaxValue)
$AtLogon = New-ScheduledTaskTrigger -AtLogOn

Register-ScheduledTask -TaskName $TaskName -Force `
    -Principal $Principal `
    -Action @($AttestAction, $CaptureAction) `
    -Trigger @($Hourly, $AtLogon) | Out-Null

Write-Host "Scheduled task '$TaskName' registered (attest hourly, capture at logon)."
Write-Host "Verify: run 'claude' then '/status' -> should show 'Enterprise managed settings'."
