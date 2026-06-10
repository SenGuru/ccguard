# CCGuard MDM deploy

This directory contains the installers that lock a fleet of Claude Code installs
onto CCGuard by deploying an **enterprise managed-settings.json** to each
machine and scheduling the CCGuard agent to attest + capture.

## What managed settings are

`managed-settings.json` is Claude Code's enterprise policy file. Generate it with:

```sh
ccguard-agent gen-policy \
  --server   https://ccguard.corp.example \
  --org-uuid org-1234-5678 \
  --otel     https://otel.corp.example:4318 \
  --min-version 2.1.38 \
  > managed-settings.json
```

(Or download the exact file from the CCGuard **Policy** page:
`/dashboard/policy/managed-settings.json`.)

The generated file forces telemetry on, pins the corp login org
(`forceLoginOrgUUID`), restricts managed hooks to the CCGuard server, wires the
`ccguard-agent --capture` SessionEnd hook, disables bypass-permissions mode, and
requires a minimum Claude Code version.

## Precedence (highest wins)

```
managed (this file)  >  CLI flags  >  local project (.claude)  >  project (.claude)  >  user (~/.claude)
```

Because it is the **highest precedence** layer, a user cannot override any key it
sets — that is the whole point of deploying it via MDM to a path only an admin
can write.

## Per-OS install path

| OS      | Path                                                          |
|---------|--------------------------------------------------------------|
| Windows | `C:\ProgramData\ClaudeCode\managed-settings.json`            |
| macOS   | `/Library/Application Support/ClaudeCode/managed-settings.json` |
| Linux   | `/etc/claude-code/managed-settings.json`                     |

## Verify

After install, run `claude` and type `/status`. The settings panel should show
**"Enterprise managed settings"** as an active source. If it doesn't, the file is
in the wrong path or not readable by the user.

## Attest cadence

The installers register an hourly task that runs:

```sh
ccguard-agent --server <url> --token $CCGUARD_TOKEN --attest
```

Hourly attestation is the right balance: the fleet page treats any device whose
`last_seen` is older than **15 minutes** as `stale`, so an hourly cadence keeps
healthy machines green while surfacing a machine that has gone dark within one
attest interval. The `--capture` hook (wired by the managed settings itself) runs
at SessionEnd and uploads the full transcript; the Windows installer additionally
runs `--capture` at logon as a backstop.

## Install

```powershell
# Windows (elevated)
./windows-install.ps1 -PolicyJson .\managed-settings.json -ServerUrl https://ccguard.corp.example
```

```sh
# macOS (sudo)
sudo ./macos-install.sh ./managed-settings.json https://ccguard.corp.example

# Linux (sudo)
sudo ./linux-install.sh ./managed-settings.json https://ccguard.corp.example
```
