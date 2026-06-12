<#
.SYNOPSIS
    Remove the Claresso agent service (scheduled task) installed by
    windows-install-service.ps1.
.PARAMETER TaskName
    Scheduled-task name to remove (default: ClaressoAgent).
.PARAMETER InstallDir
    Install dir to delete (default: C:\ProgramData\Claresso).
.PARAMETER KeepToken
    Keep the CCGUARD_TOKEN user env var (default: remove it).
#>
param(
    [string]$TaskName = "ClaressoAgent",
    [string]$InstallDir = "C:\ProgramData\Claresso",
    [switch]$KeepToken
)

$ErrorActionPreference = "SilentlyContinue"

Stop-ScheduledTask -TaskName $TaskName
Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
Write-Host "Removed scheduled task '$TaskName'."

if (Test-Path $InstallDir) {
    Remove-Item -Recurse -Force $InstallDir
    Write-Host "Removed $InstallDir."
}

if (-not $KeepToken) {
    [Environment]::SetEnvironmentVariable("CCGUARD_TOKEN", $null, "User")
    Write-Host "Removed CCGUARD_TOKEN (user scope)."
}
