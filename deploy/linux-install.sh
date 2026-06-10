#!/usr/bin/env bash
# Deploy the CCGuard managed-settings.json policy and schedule the agent (Linux).
# Idempotent. Run with sudo.
#
# Usage: sudo ./linux-install.sh <managed-settings.json> <server-url> [agent-path]
set -euo pipefail

POLICY_JSON="${1:?usage: linux-install.sh <managed-settings.json> <server-url> [agent-path]}"
SERVER_URL="${2:?usage: linux-install.sh <managed-settings.json> <server-url> [agent-path]}"
AGENT="${3:-/usr/local/bin/ccguard-agent}"

[ -f "$POLICY_JSON" ] || { echo "Policy file not found: $POLICY_JSON" >&2; exit 1; }

DIR="/etc/claude-code"
DEST="$DIR/managed-settings.json"

# 1. Install the managed settings (highest-precedence enterprise policy).
sudo mkdir -p "$DIR"
sudo cp "$POLICY_JSON" "$DEST"
# root-owned, world-readable, not world-writable: users load but cannot tamper.
sudo chown root:root "$DEST"
sudo chmod 644 "$DEST"
echo "Installed $DEST (root:root 644)"

# 2. systemd service + timer: attest hourly.
#    CCGUARD_TOKEN is read from /etc/ccguard/token.env (key=value, root 600).
sudo tee /etc/systemd/system/ccguard-attest.service >/dev/null <<SERVICE_EOF
[Unit]
Description=CCGuard agent attestation
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
EnvironmentFile=-/etc/ccguard/token.env
ExecStart=$AGENT --server $SERVER_URL --token \${CCGUARD_TOKEN} --attest
SERVICE_EOF

sudo tee /etc/systemd/system/ccguard-attest.timer >/dev/null <<'TIMER_EOF'
[Unit]
Description=Run CCGuard attestation hourly

[Timer]
OnBootSec=2min
OnUnitActiveSec=1h
Persistent=true

[Install]
WantedBy=timers.target
TIMER_EOF

sudo systemctl daemon-reload
sudo systemctl enable --now ccguard-attest.timer
echo "systemd timer installed (attest hourly)."
echo "Verify: run 'claude' then '/status' -> should show 'Enterprise managed settings'."
