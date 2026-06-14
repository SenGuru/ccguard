#!/usr/bin/env python
"""CCGuard task runner. Usage: python do.py <alias> [args...]

Dev:
  r            cargo run -p ccguard-server (local, binds 0.0.0.0:8080)
  db           docker compose up -d db (local Postgres on :5432)
  t            cargo test --workspace
  c            cargo check --workspace

Build & deploy (QA = the shared landessware.com box):
  build [env]      cross-compile the Linux server binary in Docker (musl static)
  deploy [env]     build -> upload binary + env file -> restart the systemd service
                   (add --skip-build to reuse the last built binary)
  provision [env]  one-time server bootstrap: Postgres role+db, systemd unit,
                   nginx vhost for the subdomain, TLS (self-signed -> Let's Encrypt)

  <env> defaults to qa. Server IP + ssh key live in _DEPLOY_TARGETS below; the
  rest of each env's layout comes from deploy/<env>config.json, and runtime
  secrets from deploy/<env>.env (gitignored; see deploy/qa.env.example).

The ccguard-server binary runs its own sqlx migrations at startup, so there is
no separate migrate step — restarting the service after a deploy migrates the DB.
"""
from __future__ import annotations

import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING
from urllib.parse import urlsplit, unquote

if TYPE_CHECKING:
    import paramiko

ROOT = Path(__file__).resolve().parent
DEPLOY_DIR = ROOT / "deploy"
DIST_DIR = DEPLOY_DIR / "dist"

# The server crate's release binary, cross-compiled for Linux into its own target
# dir so it never collides with the Windows host's target/release artifacts.
LINUX_TARGET_DIR = ROOT / "target" / "linux-musl"
LINUX_BINARY = LINUX_TARGET_DIR / "release" / "ccguard-server"


# --------------------------------------------------------------------------- #
# Deployment targets — server IP + local ssh key per environment.
# qa is the shared landessware.com box (also hosts attend / PoolApp); ccguard
# coexists as a name-based nginx vhost on the same 80/443, proxying to :7070.
# Drop the box's ssh private key at deploy/keys/id_ed25519 (gitignored).
# --------------------------------------------------------------------------- #
@dataclass
class DeployTarget:
    server_ip: str
    ssh_key: Path


_DEPLOY_KEY = DEPLOY_DIR / "keys" / "id_ed25519"
_DEPLOY_TARGETS: dict[str, DeployTarget] = {
    "qa": DeployTarget("168.144.121.191", _DEPLOY_KEY),
}
DEPLOY_ENVS = tuple(_DEPLOY_TARGETS)


@dataclass
class DeployConfig:
    """Target (from _DEPLOY_TARGETS) + layout (from deploy/<env>config.json)."""

    env_name: str
    server_ip: str
    ssh_user: str
    ssh_key: Path | None
    env_file: Path | None        # pushed to <remote_path>/<env>.env (systemd EnvironmentFile)
    config_file: Path | None     # pushed to <remote_path>/configuration/ccg.json
    remote_path: str
    service_name: str
    subdomain: str
    internal_bind: str           # host:port the app listens on, behind nginx
    public_https_port: int
    max_body_mb: int             # nginx client_max_body_size (matches /v1/capture's 64MB limit)
    lets_encrypt: bool
    lets_encrypt_email: str

    @property
    def internal_host(self) -> str:
        return self.internal_bind.rsplit(":", 1)[0]

    @property
    def internal_port(self) -> str:
        return self.internal_bind.rsplit(":", 1)[1]

    @property
    def remote_binary(self) -> str:
        return f"{self.remote_path}/bin/{self.service_name}-server"

    @property
    def remote_env(self) -> str:
        return f"{self.remote_path}/{self.env_name}.env"

    @property
    def remote_config(self) -> str:
        return f"{self.remote_path}/configuration/ccg.json"

    @property
    def ssl_certfile(self) -> str:
        return f"{self.remote_path}/certs/selfsigned.crt"

    @property
    def ssl_keyfile(self) -> str:
        return f"{self.remote_path}/certs/selfsigned.key"

    @property
    def ssl_current_certfile(self) -> str:
        """Symlink nginx serves — self-signed at bootstrap, Let's Encrypt once issued."""
        return f"{self.remote_path}/certs/current.crt"

    @property
    def ssl_current_keyfile(self) -> str:
        return f"{self.remote_path}/certs/current.key"


def _deploy_path(value: str) -> Path | None:
    if not value:
        return None
    p = Path(value)
    return p if p.is_absolute() else DEPLOY_DIR / p


def _load_deploy_config(env_name: str) -> DeployConfig:
    target = _DEPLOY_TARGETS[env_name]
    cfg_path = DEPLOY_DIR / f"{env_name}config.json"
    raw = json.loads(cfg_path.read_text(encoding="utf-8")) if cfg_path.is_file() else {}
    deploy = raw.get("deploy", {})
    return DeployConfig(
        env_name=env_name,
        server_ip=target.server_ip,
        ssh_user=deploy.get("sshUser", "root"),
        ssh_key=target.ssh_key,
        env_file=_deploy_path(deploy.get("envFile", f"{env_name}.env")),
        config_file=_deploy_path(deploy.get("configFile", f"{env_name}-ccg.json")),
        remote_path=deploy.get("remotePath", "/opt/ccguard"),
        service_name=deploy.get("serviceName", "ccguard"),
        subdomain=deploy.get("subdomain", ""),
        internal_bind=deploy.get("internalBind", "127.0.0.1:7070"),
        public_https_port=int(deploy.get("publicHttpsPort", 443)),
        max_body_mb=int(deploy.get("maxBodyMb", 64)),
        lets_encrypt=bool(deploy.get("letsEncrypt", False)),
        lets_encrypt_email=deploy.get("letsEncryptEmail", ""),
    )


def _run(cmd: list[str], cwd: Path | None = None) -> int:
    print(f"$ {' '.join(cmd)}")
    return subprocess.call(cmd, cwd=str(cwd or ROOT))


# --------------------------------------------------------------------------- #
# Local dev helpers
# --------------------------------------------------------------------------- #
def cmd_run(*args: str) -> int:
    return _run(["cargo", "run", "-p", "ccguard-server", *args])


def cmd_db() -> int:
    return _run(["docker", "compose", "up", "-d", "db"])


def cmd_test(*args: str) -> int:
    return _run(["cargo", "test", "--workspace", *args])


def cmd_check(*args: str) -> int:
    return _run(["cargo", "check", "--workspace", *args])


# --------------------------------------------------------------------------- #
# Build — cross-compile the Linux server binary in Docker.
# rust:alpine is musl-native, so a plain release build is a static x86_64-musl
# binary that runs on the QA box with zero shared-lib / glibc-version surprises.
# A named volume caches the crate registry across builds.
# --------------------------------------------------------------------------- #
def cmd_build(*args: str) -> int:
    """Cross-compile ccguard-server for Linux (musl static) via Docker."""
    if not _docker_available():
        print("Docker is required to cross-compile the Linux binary.\n"
              "   Install Docker Desktop and ensure `docker` is on PATH.")
        return 2

    work = str(ROOT).replace("\\", "/")
    build = (
        "apk add --no-cache build-base >/dev/null && "
        "cargo build --release --bin ccguard-server --target-dir target/linux-musl"
    )
    cmd = [
        "docker", "run", "--rm",
        "-v", f"{work}:/work", "-w", "/work",
        "-v", "ccguard-cargo-registry:/usr/local/cargo/registry",
        "rust:alpine", "sh", "-c", build,
    ]
    rc = _run(cmd)
    if rc == 0:
        if not LINUX_BINARY.is_file():
            print(f"build reported success but {LINUX_BINARY} is missing")
            return 1
        size_mb = LINUX_BINARY.stat().st_size / (1024 * 1024)
        print(f"built {LINUX_BINARY} ({size_mb:.1f} MB, linux x86_64-musl)")
    return rc


def _docker_available() -> bool:
    try:
        return subprocess.call(
            ["docker", "version"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        ) == 0
    except OSError:
        return False


# --------------------------------------------------------------------------- #
# Remote helpers (SSH via paramiko)
# --------------------------------------------------------------------------- #
def _connect_ssh(cfg: DeployConfig) -> paramiko.SSHClient:
    try:
        import paramiko as _paramiko
    except ImportError:
        print("paramiko is required for deploy/provision — run: pip install paramiko")
        raise SystemExit(2)
    client = _paramiko.SSHClient()
    client.load_system_host_keys()
    client.set_missing_host_key_policy(_paramiko.AutoAddPolicy())
    key_file: str | None = None
    if cfg.ssh_key is not None and cfg.ssh_key.is_file():
        key_file = str(cfg.ssh_key)
    else:
        print(f"warning: ssh key {cfg.ssh_key} not found; using default ssh identity")
    print(f"connecting to {cfg.ssh_user}@{cfg.server_ip} ...")
    client.connect(cfg.server_ip, username=cfg.ssh_user, key_filename=key_file, timeout=15)
    return client


def _ssh_exec(client: paramiko.SSHClient, command: str) -> int:
    """Run a remote command, streaming combined stdout+stderr; returns exit code."""
    print(f"[remote] $ {command}")
    transport = client.get_transport()
    if transport is None:
        print("ssh transport unavailable")
        return 1
    channel = transport.open_session()
    channel.set_combine_stderr(True)
    channel.exec_command(command)
    while True:
        data = channel.recv(4096)
        if not data:
            break
        text = data.decode(errors="replace")
        encoding = sys.stdout.encoding or "utf-8"
        sys.stdout.write(text.encode(encoding, errors="replace").decode(encoding))
        sys.stdout.flush()
    return channel.recv_exit_status()


def _sftp_put(client: paramiko.SSHClient, local: Path, remote: str) -> None:
    print(f"[sftp] {local} -> {remote}")
    sftp = client.open_sftp()
    try:
        sftp.put(str(local), remote)
    finally:
        sftp.close()


def _validated_deploy_config(env_name: str, command: str) -> DeployConfig | None:
    if env_name not in DEPLOY_ENVS:
        print(f"usage: python do.py {command} <{'|'.join(DEPLOY_ENVS)}>")
        return None
    cfg = _load_deploy_config(env_name)
    if not cfg.server_ip:
        print(f"no server ip for '{env_name}' — fill in _DEPLOY_TARGETS in do.py")
        return None
    if not cfg.subdomain:
        print(f"no subdomain for '{env_name}' — set deploy.subdomain in {env_name}config.json")
        return None
    return cfg


def _resolve_env_file(cfg: DeployConfig) -> Path:
    """Copy the env file to dist/, forcing CCGUARD_BIND to the configured internal bind."""
    assert cfg.env_file is not None
    lines = cfg.env_file.read_text(encoding="utf-8").splitlines()
    want = f"CCGUARD_BIND={cfg.internal_bind}"
    for i, line in enumerate(lines):
        if line.strip().startswith("CCGUARD_BIND="):
            if line.strip() != want:
                print(f"note: forcing {want} in shipped env (was {line.strip()})")
            lines[i] = want
            break
    else:
        lines.append(want)
    DIST_DIR.mkdir(parents=True, exist_ok=True)
    resolved = DIST_DIR / f"{cfg.env_name}.resolved.env"
    resolved.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    return resolved


# --------------------------------------------------------------------------- #
# Deploy (assumes the env has already been provisioned)
# --------------------------------------------------------------------------- #
def cmd_deploy(*args: str) -> int:
    """Build -> upload binary + env file -> restart the service.

    Usage: python do.py deploy [qa] [--skip-build]

    The binary runs its own migrations at startup, so restarting the service
    after the new binary lands is what migrates the database.
    """
    skip_build = "--skip-build" in args
    positional = [a for a in args if not a.startswith("--")]
    env_name = positional[0] if positional else "qa"
    cfg = _validated_deploy_config(env_name, "deploy")
    if cfg is None:
        return 2
    if cfg.env_file is None or not cfg.env_file.is_file():
        print(f"env file {cfg.env_file} not found — copy deploy/{env_name}.env.example "
              f"to deploy/{env_name}.env and fill it in")
        return 2
    if cfg.config_file is None or not cfg.config_file.is_file():
        print(f"config file {cfg.config_file} not found — create deploy/{env_name}-ccg.json")
        return 2

    if not skip_build:
        rc = cmd_build()
        if rc:
            return rc
    if not LINUX_BINARY.is_file():
        print(f"no binary at {LINUX_BINARY} — run without --skip-build first")
        return 2

    resolved_env = _resolve_env_file(cfg)
    remote_tmp_bin = f"/tmp/{cfg.service_name}-server"
    remote_tmp_env = f"/tmp/{cfg.service_name}-{env_name}.env"
    remote_tmp_cfg = f"/tmp/{cfg.service_name}-ccg.json"

    client = _connect_ssh(cfg)
    try:
        _sftp_put(client, LINUX_BINARY, remote_tmp_bin)
        _sftp_put(client, resolved_env, remote_tmp_env)
        _sftp_put(client, cfg.config_file, remote_tmp_cfg)
        steps = [
            f"systemctl stop {cfg.service_name} || true",
            f"mkdir -p {cfg.remote_path}/bin",
            f"mv {remote_tmp_bin} {cfg.remote_binary}",
            f"chmod +x {cfg.remote_binary}",
            f"mv {remote_tmp_env} {cfg.remote_env}",
            f"chmod 600 {cfg.remote_env}",
            f"mkdir -p {cfg.remote_path}/configuration {cfg.remote_path}/data/logs",
            f"mv {remote_tmp_cfg} {cfg.remote_config}",
            f"chmod 600 {cfg.remote_config}",
            f"systemctl start {cfg.service_name}",
            "sleep 2",
            f"systemctl status {cfg.service_name} --no-pager -l | head -n 8",
        ]
        rc = _ssh_exec(client, " && ".join(steps))
        if rc == 0:
            print(f"\ndeployed — https://{cfg.subdomain}/ (app on {cfg.internal_bind})")
        return rc
    finally:
        client.close()


# --------------------------------------------------------------------------- #
# Provision (one-time server bootstrap)
# --------------------------------------------------------------------------- #
_SYSTEMD_UNIT_TEMPLATE = """\
[Unit]
Description=ccguard server ({env})
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
WorkingDirectory={remote_path}
EnvironmentFile=-{remote_path}/{env}.env
ExecStart={remote_binary}
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
"""

# Name-based vhost: NOT default_server, so it coexists with the box's other
# sites (attend/PoolApp own the default 80/443). nginx routes by Host/SNI.
_NGINX_SITE_TEMPLATE = """\
server {{
    listen 80;
    server_name {subdomain};
    location /.well-known/acme-challenge/ {{ root /var/www/html; }}
    location / {{ return 301 https://$host$request_uri; }}
}}

server {{
    listen {https_port} ssl;
    server_name {subdomain};
    ssl_certificate     {cert_file};
    ssl_certificate_key {key_file};

    client_max_body_size {max_body}m;

    location / {{
        proxy_pass http://{internal_host}:{internal_port};
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 120;
    }}
}}
"""


def _db_steps(cfg: DeployConfig) -> list[str]:
    """Idempotent CREATE DATABASE from the database url in deploy/<env>-ccg.json.

    Connects to whatever host the URL names. The QA box uses the shared remote
    Postgres (same server as attend), so this shells `psql` over the network as
    the URL's user; for a localhost URL it drops to the local postgres superuser
    instead. A separate role is only created when the user isn't `postgres`.
    """
    if cfg.config_file is None or not cfg.config_file.is_file():
        print("warning: no ccg.json config — skipping db creation")
        return []
    raw = json.loads(cfg.config_file.read_text(encoding="utf-8"))
    url = (raw.get("database", {}) or {}).get("url", "")
    if not url:
        print("warning: no database.url in ccg.json — skipping db creation")
        return []
    parts = urlsplit(url)
    user = unquote(parts.username or "postgres")
    password = unquote(parts.password or "")
    host = parts.hostname or "localhost"
    port = parts.port or 5432
    dbname = parts.path.lstrip("/") or "ccguard"

    if host in ("localhost", "127.0.0.1", "::1"):
        # Local box Postgres: act as the OS postgres superuser.
        pg = "runuser -u postgres -- psql"
        pw_lit = password.replace("'", "''")
        steps = ["systemctl enable --now postgresql"]
        if user != "postgres":
            steps.append(
                f"{pg} -tAc \"SELECT 1 FROM pg_roles WHERE rolname='{user}'\" | grep -q 1 || "
                f"{pg} -c \"CREATE ROLE \\\"{user}\\\" LOGIN PASSWORD '{pw_lit}'\"")
        steps.append(
            f"{pg} -tAc \"SELECT 1 FROM pg_database WHERE datname='{dbname}'\" | grep -q 1 || "
            f"{pg} -c \"CREATE DATABASE \\\"{dbname}\\\" OWNER \\\"{user}\\\"\"")
        return steps

    # Remote Postgres: connect over the network as the URL's user (must be able to
    # create databases — the shared box connects as the postgres superuser). The
    # hyphenated db name needs SQL double-quoting; psql -c carries it single-quoted.
    pg = f"PGPASSWORD='{password}' psql -h {host} -p {port} -U {user} -d postgres"
    return [
        (f"( {pg} -tAc \"SELECT 1 FROM pg_database WHERE datname='{dbname}'\" | grep -q 1 || "
         f"{pg} -c 'CREATE DATABASE \"{dbname}\"' )"),
    ]


def cmd_provision(*args: str) -> int:
    """One-time bootstrap: packages -> Postgres role+db -> systemd unit -> nginx vhost -> TLS.

    Usage: python do.py provision [qa]

    Installs nginx + Postgres + certbot, creates the app's db/role, writes the
    systemd unit and a name-based nginx vhost for the subdomain (TLS terminated
    by nginx, proxying to the localhost app port), bootstraps a self-signed cert,
    then issues a real Let's Encrypt cert if letsEncrypt is on and the subdomain
    resolves to this box. The service is enabled; `deploy` lands the binary and
    starts it. Idempotent — safe to re-run.
    """
    env_name = args[0] if args else "qa"
    cfg = _validated_deploy_config(env_name, "provision")
    if cfg is None:
        return 2
    if cfg.env_file is None or not cfg.env_file.is_file():
        print(f"env file {cfg.env_file} not found — copy deploy/{env_name}.env.example "
              f"to deploy/{env_name}.env and fill it in")
        return 2

    DIST_DIR.mkdir(parents=True, exist_ok=True)
    unit_path = DIST_DIR / f"{cfg.service_name}-{env_name}.service"
    unit_path.write_text(
        _SYSTEMD_UNIT_TEMPLATE.format(
            env=env_name,
            remote_path=cfg.remote_path,
            remote_binary=cfg.remote_binary,
        ),
        encoding="utf-8", newline="\n",  # systemd units need LF, not Windows CRLF
    )
    nginx_path = DIST_DIR / f"{cfg.service_name}-{env_name}.nginx"
    nginx_path.write_text(
        _NGINX_SITE_TEMPLATE.format(
            subdomain=cfg.subdomain,
            https_port=cfg.public_https_port,
            cert_file=cfg.ssl_current_certfile,
            key_file=cfg.ssl_current_keyfile,
            max_body=cfg.max_body_mb,
            internal_host=cfg.internal_host,
            internal_port=cfg.internal_port,
        ),
        encoding="utf-8", newline="\n",
    )
    print(f"built {unit_path}\nbuilt {nginx_path} (server_name: {cfg.subdomain})")

    remote_unit = f"/tmp/{cfg.service_name}.service"
    remote_nginx = f"/tmp/{cfg.service_name}.nginx"
    client = _connect_ssh(cfg)
    try:
        _sftp_put(client, unit_path, remote_unit)
        _sftp_put(client, nginx_path, remote_nginx)

        # postgresql-client gives us psql to create the DB on the remote server;
        # no local postgresql server is needed (the QA DB lives on a shared box).
        packages = "nginx postgresql-client openssl"
        if cfg.lets_encrypt:
            packages += " certbot python3-certbot-nginx"

        cn = cfg.subdomain
        san = f"DNS:{cfg.subdomain},IP:{cfg.server_ip}"
        san_marker = f"{cfg.remote_path}/certs/.san"

        steps = [
            "export DEBIAN_FRONTEND=noninteractive",
            "apt-get update -qq",
            f"apt-get install -y -qq {packages}",
            f"mkdir -p {cfg.remote_path}/bin {cfg.remote_path}/certs "
            f"{cfg.remote_path}/configuration {cfg.remote_path}/data/logs",
        ]
        steps += _db_steps(cfg)
        steps += [
            # Self-signed cert (regenerated only if missing or the SAN set changed).
            (f"( test -f {cfg.ssl_certfile} && test -f {cfg.ssl_keyfile} "
             f"&& [ \"$(cat {san_marker} 2>/dev/null)\" = '{san}' ] ) || "
             f"( openssl req -x509 -nodes -newkey rsa:2048 -days 825 "
             f"-keyout {cfg.ssl_keyfile} -out {cfg.ssl_certfile} "
             f"-subj '/CN={cn}' -addext 'subjectAltName={san}' "
             f"&& echo '{san}' > {san_marker} )"),
            f"chmod 600 {cfg.ssl_keyfile}",
            # nginx serves the current.* symlinks; bootstrap to self-signed
            # (a previous Let's Encrypt flip is preserved).
            f"test -e {cfg.ssl_current_certfile} || ln -sf {cfg.ssl_certfile} {cfg.ssl_current_certfile}",
            f"test -e {cfg.ssl_current_keyfile} || ln -sf {cfg.ssl_keyfile} {cfg.ssl_current_keyfile}",
            f"mv {remote_nginx} /etc/nginx/sites-available/{cfg.service_name}",
            (f"ln -sf /etc/nginx/sites-available/{cfg.service_name} "
             f"/etc/nginx/sites-enabled/{cfg.service_name}"),
            "nginx -t",
            "systemctl enable --now nginx",
            "systemctl reload nginx",
        ]
        if cfg.lets_encrypt:
            le_live = f"/etc/letsencrypt/live/{cfg.subdomain}"
            email_arg = (f"-m {cfg.lets_encrypt_email}" if cfg.lets_encrypt_email
                         else "--register-unsafely-without-email")
            steps += [
                # Issue (or keep) the real cert through the running nginx, then flip
                # the symlinks. Fault-tolerant: on failure the site keeps self-signed.
                (f"( certbot certonly --nginx -d {cfg.subdomain} "
                 f"--non-interactive --agree-tos {email_arg} --keep-until-expiring "
                 f"--deploy-hook 'systemctl reload nginx' "
                 f"&& ln -sf {le_live}/fullchain.pem {cfg.ssl_current_certfile} "
                 f"&& ln -sf {le_live}/privkey.pem {cfg.ssl_current_keyfile} "
                 f"&& systemctl reload nginx && echo 'lets encrypt cert active' ) "
                 f"|| echo 'lets encrypt issuance failed — still serving self-signed'"),
                ("echo '23 3 * * * root certbot renew --quiet "
                 "--deploy-hook \"systemctl reload nginx\"' > /etc/cron.d/certbot-renew"),
                "chmod 644 /etc/cron.d/certbot-renew",
            ]
        steps += [
            f"mv {remote_unit} /etc/systemd/system/{cfg.service_name}.service",
            "systemctl daemon-reload",
            f"systemctl enable {cfg.service_name}",
            # If a binary is already deployed, (re)start it; otherwise deploy starts it.
            f"test -x {cfg.remote_binary} && systemctl restart {cfg.service_name} || true",
            f"echo 'provisioned — now run: python do.py deploy {env_name}'",
        ]
        return _ssh_exec(client, " && ".join(steps))
    finally:
        client.close()


_ALIASES = {
    "r": cmd_run, "db": cmd_db, "t": cmd_test, "c": cmd_check,
    "build": cmd_build, "deploy": cmd_deploy, "provision": cmd_provision,
}


def main() -> int:
    if len(sys.argv) < 2 or sys.argv[1] in ("/?", "-h", "--help"):
        print(__doc__)
        return 0
    alias = sys.argv[1]
    fn = _ALIASES.get(alias)
    if fn is None:
        print(f"unknown alias: {alias}\n{__doc__}")
        return 2
    return fn(*sys.argv[2:])


if __name__ == "__main__":
    sys.exit(main())
