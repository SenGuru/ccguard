#!/usr/bin/env bash
# Deploy the CCGuard managed-settings.json policy and schedule the agent (macOS).
# Idempotent. Run with sudo.
#
# Usage: sudo ./macos-install.sh <managed-settings.json> <server-url> [agent-path]
set -euo pipefail

POLICY_JSON="${1:?usage: macos-install.sh <managed-settings.json> <server-url> [agent-path]}"
SERVER_URL="${2:?usage: macos-install.sh <managed-settings.json> <server-url> [agent-path]}"
AGENT="${3:-/usr/local/bin/ccguard-agent}"

[ -f "$POLICY_JSON" ] || { echo "Policy file not found: $POLICY_JSON" >&2; exit 1; }

DIR="/Library/Application Support/ClaudeCode"
DEST="$DIR/managed-settings.json"

# 1. Install the managed settings (highest-precedence enterprise policy).
sudo mkdir -p "$DIR"
sudo cp "$POLICY_JSON" "$DEST"
# root-owned, world-readable, not world-writable: users load but cannot tamper.
sudo chown root:wheel "$DEST"
sudo chmod 644 "$DEST"
echo "Installed $DEST (root:wheel 644)"

# 2. launchd job: attest hourly. CCGUARD_TOKEN is read from the machine env;
#    set it in /etc/launchd.conf or inject via your MDM payload.
PLIST="/Library/LaunchDaemons/com.ccguard.agent.plist"
sudo tee "$PLIST" >/dev/null <<PLIST_EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>            <string>com.ccguard.agent</string>
  <key>ProgramArguments</key>
  <array>
    <string>$AGENT</string>
    <string>--server</string> <string>$SERVER_URL</string>
    <string>--token</string>  <string>\$CCGUARD_TOKEN</string>
    <string>--attest</string>
  </array>
  <key>StartInterval</key>    <integer>3600</integer>
  <key>RunAtLoad</key>        <true/>
</dict>
</plist>
PLIST_EOF

sudo chown root:wheel "$PLIST"
sudo chmod 644 "$PLIST"
sudo launchctl bootout system "$PLIST" 2>/dev/null || true
sudo launchctl bootstrap system "$PLIST"
echo "launchd job installed (attest hourly)."
echo "Verify: run 'claude' then '/status' -> should show 'Enterprise managed settings'."
