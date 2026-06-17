#!/usr/bin/env python3
r"""Generate a believable fake Claude Code machine for CCGuard demos / testing.

Why this is more than transcripts
----------------------------------
The CCGuard agent does NOT just tail JSONL. Per session it fingerprints the whole
machine (see crates/ccguard-agent/src/{main,signals,repo,paths}.rs):

  1. ~/.claude.json            -> oauthAccount.emailAddress + organization uuid (seat identity)
  2. ~/.claude/.credentials.json -> claudeAiOauth.subscriptionType ("max"/"pro")
  3. ~/.claude/projects/<encoded-cwd>/<uuid>.jsonl -> the session transcripts
  4. LIVE `git` commands run against each session's `cwd`  <-- the classification moat
        git remote -v, @{u} (pushed?), log -1 %ce / %G?, config user.email,
        rev-parse --show-toplevel (monorepo), plus .npmrc/package.json/go.mod scan

If the cwd paths are not real git repos, every session lands UNCLASSIFIED and the
work/personal donut is empty -- which is exactly the "looks fake" failure we want
to avoid. So this script materialises real on-disk repos engineered to land in each
provenance bucket, then writes transcripts whose `cwd` points at them.

What it builds
--------------
Under  %USERPROFILE%\.claude  (override with --claude-dir):
  .claude.json, .claude/.credentials.json
  .claude/projects/<encoded>/<uuid>.jsonl   (one per session)
Under  <workroot>  (default %USERPROFILE%\dev, override with --work-root):
  real git repos, one per project, with remotes/upstream/commits crafted per bucket.

Provenance buckets (verified against crates/ccguard-core/src/provenance.rs):
  WORK (Tier-G, auto 0.95) : corp remote on allowlist + pushed upstream (W-PUSH)
  WORK-PROVISIONAL (0.6)   : corp remote present, NOT pushed, + corp git email / @scope registry
  PERSONAL-hint -> UNCLASS : a personal (github.com/<you>) remote, unsigned -> stays UNCLASSIFIED
                             (PERSONAL needs 2 independent signals; the AI judge is what
                              actually calls these personal -- which is the product thesis)
  SCRATCH -> UNCLASSIFIED  : no git at all (a plain dir)

The AI judge (agent --triage via local `claude -p`) is what turns the pending/UNKNOWN
sessions into work/personal on the dashboard; the git layer just supplies the
deterministic corroborators + the Tier-G freebies, exactly like a real fleet.

Usage
-----
  python tools/gen_fake_machine.py                 # build everything (idempotent-ish)
  python tools/gen_fake_machine.py --days 21 --sessions 40
  python tools/gen_fake_machine.py --email dev@acme-corp.com --org-uuid <uuid>
  python tools/gen_fake_machine.py --clean         # remove what a previous run created
  python tools/gen_fake_machine.py --dry-run       # print the plan, write nothing

Then point the agent at it:
  ccguard-agent --server http://localhost:8080 --token ccg_... --capture
  ccguard-agent --server http://localhost:8080 --token ccg_... --triage --force
"""
from __future__ import annotations

import argparse
import json
import os
import random
import shutil
import subprocess
import sys
import uuid
from datetime import datetime, timedelta, timezone
from pathlib import Path

# --------------------------------------------------------------------------- #
# Tenant defaults. These match the conventions in spec/mock_anthropic.py and the
# provenance unit tests (corp host github.com, corp org acme-corp, corp domain
# acme-corp.com). Change with flags to match whatever tenant you set up in the
# dashboard's work-definition / allowlist.
# --------------------------------------------------------------------------- #
CORP_HOST = "github.com"
CORP_ORG = "acme-corp"
CORP_DOMAIN = "acme-corp.com"
DEFAULT_EMAIL = f"dev@{CORP_DOMAIN}"
PERSONAL_HANDLE = "gsenthil-dev"  # a personal github org/owner (NOT on the allowlist)

MARKER = ".ccguard-fake"  # dropped in each created dir so --clean knows what we made

# A model mix that looks like a real Max user (Opus for big turns, Sonnet/Haiku otherwise).
MODELS = ["claude-opus-4-8", "claude-sonnet-4-8", "claude-haiku-4-5"]


# --------------------------------------------------------------------------- #
# Project catalogue. Each entry becomes one repo + a handful of sessions.
#   bucket: how the on-disk git repo is crafted (drives provenance)
#   tasks : a pool of realistic, DISTINCT prompts so repeated sessions on the same
#           repo don't read like copy-paste (the dead giveaway of fake data)
#   files : candidate files the session reads/edits
# --------------------------------------------------------------------------- #
PROJECTS = [
    # --- company work: Tier-G (corp remote + pushed) ----------------------- #
    dict(name="billing-svc", bucket="work_pushed", lang="rust",
         files=["src/webhook.rs", "src/idempotency.rs", "src/refund.rs",
                "src/invoice.rs", "tests/webhook_test.rs"],
         tasks=[
             "Add idempotency keys to the Stripe webhook handler",
             "Handle the partial-refund case in the invoice reconciler",
             "Fix the off-by-one in the proration calc for mid-cycle upgrades",
             "Add retry with backoff to the dunning email job",
             "Write tests for the failed-payment state machine",
             "Investigate why some webhooks are processed twice",
         ]),
    dict(name="checkout-web", bucket="work_pushed", lang="ts",
         files=["src/cart/merge.ts", "src/cart/merge.test.ts",
                "src/checkout/form.tsx", "src/checkout/validate.ts"],
         tasks=[
             "Fix a race in the cart-merge reducer on login",
             "Add inline validation to the shipping-address form",
             "Debounce the promo-code lookup so it stops spamming the API",
             "Make the checkout button disabled state accessible",
             "Persist the cart to localStorage across refreshes",
         ]),
    dict(name="fleet-api", bucket="work_pushed", lang="go",
         files=["internal/devices/handler.go", "internal/devices/page.go",
                "internal/auth/middleware.go", "internal/devices/handler_test.go"],
         tasks=[
             "Add pagination to the GET /devices endpoint",
             "Add a rate-limit middleware keyed by API token",
             "Return 409 instead of 500 on a duplicate enroll",
             "Add structured logging to the device-attest path",
             "Cache the org lookup so every request stops hitting Postgres",
         ]),

    # --- company work: WORK-PROVISIONAL (corp remote, not pushed, corp email/@scope) #
    dict(name="ops-runbook-tools", bucket="work_provisional", lang="ts",
         files=["src/rotate.ts", "src/pager.ts", "package.json"],
         tasks=[
             "Script to rotate the on-call PagerDuty schedule from a CSV",
             "Add a dry-run flag to the schedule-rotation script",
             "Generate the weekly on-call summary as markdown",
             "Wire the runbook tool to read secrets from the vault, not env",
         ]),
    dict(name="data-migration", bucket="work_provisional", lang="rust",
         files=["src/backfill.rs", "src/verify.rs"],
         tasks=[
             "Backfill the new tenant_id column on the events table",
             "Add a checkpoint so the backfill can resume after a crash",
             "Write a verifier that diffs old vs new rows after backfill",
         ]),

    # --- the employee's own / personal projects (AI judge calls these personal) #
    dict(name="my-portfolio", bucket="personal", lang="ts",
         files=["app/page.tsx", "app/hero.tsx", "app/projects.tsx", "app/blog/[slug].tsx"],
         tasks=[
             "Restyle the hero section of my personal portfolio site",
             "Add a dark-mode toggle to my portfolio",
             "Set up an MDX blog on my personal site",
             "Make my portfolio projects grid responsive on mobile",
         ]),
    dict(name="leetcode-grind", bucket="personal", lang="py",
         files=["week_412/two_sum.py", "week_412/notes.md", "graphs/dijkstra.py"],
         tasks=[
             "Solve this week's LeetCode contest problems and explain them",
             "Explain the time complexity of my Dijkstra implementation",
             "Refactor my sliding-window solutions into a template",
             "Help me understand why my DP solution TLEs",
         ]),
    dict(name="wedding-site", bucket="personal", lang="ts",
         files=["src/rsvp.tsx", "src/registry.tsx", "src/gallery.tsx"],
         tasks=[
             "Build an RSVP form for my wedding website",
             "Add a photo gallery to the wedding site",
             "Hook the RSVP form up to a Google Sheet",
             "Add a countdown timer to the wedding date",
         ]),

    # --- scratch / no-git (stays UNCLASSIFIED until the judge looks) -------- #
    dict(name="scratch", bucket="scratch", lang="py",
         files=["rename.py", "convert.py"],
         tasks=[
             "Quick one-off script to rename a folder of photos by EXIF date",
             "Convert a folder of HEIC images to JPG",
             "One-liner to find the biggest files under a directory",
         ]),
]


# --------------------------------------------------------------------------- #
# git helpers (all local; no network ever)
# --------------------------------------------------------------------------- #
def git(repo: Path, *args: str, env: dict | None = None, check: bool = True) -> str:
    full = ["git", "-C", str(repo), *args]
    res = subprocess.run(full, capture_output=True, text=True, env=env)
    if check and res.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)} failed in {repo}:\n{res.stderr}")
    return res.stdout.strip()


def git_commit_env(base: dict, name: str, email: str, when: datetime) -> dict:
    """Author+committer identity & date for a deterministic, unsigned commit."""
    iso = when.strftime("%Y-%m-%dT%H:%M:%S")
    e = dict(base)
    e.update(
        GIT_AUTHOR_NAME=name, GIT_AUTHOR_EMAIL=email, GIT_AUTHOR_DATE=iso,
        GIT_COMMITTER_NAME=name, GIT_COMMITTER_EMAIL=email, GIT_COMMITTER_DATE=iso,
    )
    return e


def make_repo(repo: Path, *, remote_url: str | None, pushed: bool,
              commit_email: str, commit_name: str, when: datetime,
              scope_registry: str | None) -> None:
    """Create a real git repo on disk crafted to drive a specific provenance bucket.

    pushed=True  -> we fabricate an upstream tracking ref with NO network:
                    git update-ref refs/remotes/origin/<b> HEAD + set-upstream.
                    This makes `git rev-parse @{u}` succeed => RawSignals.pushed=true
                    => with a corp remote that's W-PUSH (Tier-G WORK 0.95).
    """
    repo.mkdir(parents=True, exist_ok=True)
    (repo / MARKER).write_text("created by gen_fake_machine.py\n", encoding="utf-8")
    git(repo, "init", "-q")
    # Force a deterministic default branch name.
    git(repo, "symbolic-ref", "HEAD", "refs/heads/main")
    # Local identity drives RawSignals.config_email (a corroborator).
    git(repo, "config", "user.email", commit_email)
    git(repo, "config", "user.name", commit_name)
    git(repo, "config", "commit.gpgsign", "false")  # keep commits UNSIGNED on purpose

    if remote_url:
        git(repo, "remote", "add", "origin", remote_url)
    if scope_registry:
        # A private-registry fingerprint -> C-REGISTRY corroborator.
        (repo / ".npmrc").write_text(
            f"@{CORP_ORG}:registry=https://{scope_registry}/api/npm/\n", encoding="utf-8")

    (repo / "README.md").write_text(f"# {repo.name}\n", encoding="utf-8")
    git(repo, "add", "-A")
    env = git_commit_env(os.environ.copy(), commit_name, commit_email, when)
    git(repo, "commit", "-q", "-m", "chore: initial commit", env=env)

    if pushed and remote_url:
        branch = git(repo, "rev-parse", "--abbrev-ref", "HEAD")
        head = git(repo, "rev-parse", "HEAD")
        # Fabricate the remote-tracking ref + upstream with no network at all.
        git(repo, "update-ref", f"refs/remotes/origin/{branch}", head)
        git(repo, "config", f"branch.{branch}.remote", "origin")
        git(repo, "config", f"branch.{branch}.merge", f"refs/heads/{branch}")


# --------------------------------------------------------------------------- #
# transcript synthesis
# --------------------------------------------------------------------------- #
def encode_cwd(cwd: str) -> str:
    """Claude Code's project-folder encoding: non-alnum -> '-'. The server reads
    cwd from the transcript body, so this only affects the on-disk folder name."""
    out = []
    for ch in cwd:
        out.append(ch if (ch.isalnum()) else "-")
    return "".join(out)


def jline(obj: dict) -> str:
    return json.dumps(obj, separators=(",", ":"), ensure_ascii=False)


def rand_tokens(big: bool) -> tuple[int, int]:
    if big:
        return random.randint(8000, 60000), random.randint(400, 4000)
    return random.randint(1200, 9000), random.randint(80, 900)


def tool_call(tool: str, **inp) -> dict:
    return {"type": "tool_use", "id": f"toolu_{uuid.uuid4().hex[:16]}", "name": tool, "input": inp}


def build_session(proj: dict, cwd: str, start: datetime, sid: str) -> tuple[list[str], str]:
    """Return (jsonl_lines, title). A session = a small, believable CC conversation:
    user prompt -> thinking -> tool calls (Read/Edit/Bash) -> tool results -> summary.
    """
    lines: list[str] = []
    t = start
    branch = "main"
    model = random.choice(MODELS)
    # Pick a distinct task + a believable 1-3 file subset for THIS session so
    # repeated sessions on the same repo don't read like copy-paste.
    topic = random.choice(proj["tasks"])
    k = min(len(proj["files"]), random.randint(1, 3))
    files = random.sample(proj["files"], k)

    def emit(obj: dict, dt_advance=(2, 40)):
        nonlocal t
        obj.setdefault("sessionId", sid)
        obj.setdefault("timestamp", t.astimezone(timezone.utc).isoformat().replace("+00:00", "Z"))
        obj.setdefault("cwd", cwd)
        obj.setdefault("gitBranch", branch)
        lines.append(jline(obj))
        t = t + timedelta(seconds=random.randint(*dt_advance))

    # 1) opening user prompt
    emit({"type": "user", "message": {"role": "user",
          "content": [{"type": "text", "text": topic}]}})

    # 2) assistant thinking + a plan
    ti, to = rand_tokens(big=True)
    emit({"type": "assistant", "message": {"role": "assistant", "model": model,
          "usage": {"input_tokens": ti, "output_tokens": to},
          "content": [{"type": "thinking",
                       "thinking": f"Let me look at the relevant files for: {topic}. "
                                   f"I'll read {files[0]} first, then make the change."}]}})

    # 3) read a file
    emit({"type": "assistant", "message": {"role": "assistant", "model": model,
          "usage": {"input_tokens": 0, "output_tokens": 0},
          "content": [tool_call("Read", file_path=f"{cwd}\\{files[0]}")]}})
    emit({"type": "user", "message": {"role": "user", "content": [
          {"type": "tool_result", "tool_use_id": f"toolu_{uuid.uuid4().hex[:16]}",
           "content": f"// {files[0]}\n// ... existing code ...\n"}]}})

    # 4) a couple of edits
    for f in files[: max(1, len(files) - 1)]:
        ti, to = rand_tokens(big=True)
        emit({"type": "assistant", "message": {"role": "assistant", "model": model,
              "usage": {"input_tokens": ti, "output_tokens": to},
              "content": [tool_call("Edit", file_path=f"{cwd}\\{f}",
                                    old_string="// ... existing code ...",
                                    new_string=f"// {topic}\n// implemented")]}})

    # 5) run a build/test via Bash
    test_cmd = {"rust": "cargo test", "ts": "npm test", "go": "go test ./...",
                "py": "pytest -q"}[proj["lang"]]
    emit({"type": "assistant", "message": {"role": "assistant", "model": model,
          "usage": {"input_tokens": 0, "output_tokens": 0},
          "content": [tool_call("Bash", command=test_cmd)]}})
    emit({"type": "user", "message": {"role": "user", "content": [
          {"type": "tool_result", "tool_use_id": f"toolu_{uuid.uuid4().hex[:16]}",
           "content": "ok. test result: ok. 0 failed"}]}})

    # 6) closing assistant summary
    ti, to = rand_tokens(big=False)
    emit({"type": "assistant", "message": {"role": "assistant", "model": model,
          "usage": {"input_tokens": ti, "output_tokens": to},
          "content": [{"type": "text",
                       "text": f"Done: {topic}. Edited {len(files)} file(s); tests pass."}]}})

    # 7) ai-title line (CC writes one once it has a summary)
    title = topic if len(topic) <= 60 else topic[:57] + "..."
    lines.append(jline({"type": "ai-title", "sessionId": sid, "aiTitle": title}))

    return lines, title


# --------------------------------------------------------------------------- #
# orchestration
# --------------------------------------------------------------------------- #
def bucket_repo_spec(proj: dict, email: str, name: str):
    """Map a project's bucket to its make_repo() kwargs."""
    b = proj["bucket"]
    corp_remote = f"git@{CORP_HOST}:{CORP_ORG}/{proj['name']}.git"
    pers_remote = f"https://github.com/{PERSONAL_HANDLE}/{proj['name']}.git"
    if b == "work_pushed":
        return dict(remote_url=corp_remote, pushed=True,
                    commit_email=email, commit_name=name, scope_registry=None)
    if b == "work_provisional":
        return dict(remote_url=corp_remote, pushed=False,
                    commit_email=email, commit_name=name,
                    scope_registry=f"artifactory.{CORP_DOMAIN}")
    if b == "personal":
        # personal remote + a personal-looking commit email; unsigned -> not enough
        # for a deterministic PERSONAL, so it stays UNCLASSIFIED until the AI judge.
        return dict(remote_url=pers_remote, pushed=True,
                    commit_email=f"{PERSONAL_HANDLE}@gmail.com", commit_name=name,
                    scope_registry=None)
    return None  # scratch: no git at all


def plan_summary(projects, days, sessions) -> str:
    from collections import Counter
    c = Counter(p["bucket"] for p in projects)
    return (f"{sessions} sessions across {len(projects)} repos over {days} days\n"
            f"  buckets: {dict(c)}")


def main() -> int:
    ap = argparse.ArgumentParser(description="Generate a fake Claude Code machine for CCGuard.")
    ap.add_argument("--claude-dir", default=str(Path.home() / ".claude"),
                    help="target ~/.claude dir (default: %(default)s)")
    ap.add_argument("--work-root", default=str(Path.home() / "dev"),
                    help="where to materialise the fake git repos (default: %(default)s)")
    ap.add_argument("--email", default=DEFAULT_EMAIL, help="corp seat email")
    ap.add_argument("--name", default="Dev Example", help="git author/committer name")
    ap.add_argument("--org-uuid", default=str(uuid.uuid4()), help="corp Claude org uuid")
    ap.add_argument("--plan", default="max", choices=["max", "pro", "team"], help="subscription plan")
    ap.add_argument("--days", type=int, default=14, help="spread sessions over the last N days")
    ap.add_argument("--sessions", type=int, default=28, help="total number of sessions")
    ap.add_argument("--seed", type=int, default=42, help="RNG seed for reproducibility")
    ap.add_argument("--clean", action="store_true", help="remove what a previous run created, then exit")
    ap.add_argument("--dry-run", action="store_true", help="print the plan; write nothing")
    args = ap.parse_args()

    random.seed(args.seed)
    claude_dir = Path(args.claude_dir)
    work_root = Path(args.work_root)
    projects_dir = claude_dir / "projects"

    if args.clean:
        return do_clean(claude_dir, work_root)

    print("CCGuard fake-machine generator")
    print(f"  claude dir : {claude_dir}")
    print(f"  work root  : {work_root}")
    print(f"  seat       : {args.email}  (org {args.org_uuid}, plan {args.plan})")
    print(f"  plan       : {plan_summary(PROJECTS, args.days, args.sessions)}")
    if args.dry_run:
        print("\n--dry-run: nothing written.")
        return 0

    # 1) identity + credentials --------------------------------------------- #
    claude_dir.mkdir(parents=True, exist_ok=True)
    projects_dir.mkdir(parents=True, exist_ok=True)
    dot_json = claude_dir.parent / ".claude.json"
    write_identity(dot_json, args.email, args.org_uuid)
    write_credentials(claude_dir / ".credentials.json", args.plan)
    print(f"  wrote {dot_json}")
    print(f"  wrote {claude_dir / '.credentials.json'}")

    # 2) real git repos per project ----------------------------------------- #
    now = datetime.now(timezone.utc)
    repo_cwd: dict[str, str] = {}
    for proj in PROJECTS:
        repo = work_root / proj["name"]
        cwd = str(repo).replace("/", "\\")  # CC stores Windows-style cwds
        repo_cwd[proj["name"]] = cwd
        spec = bucket_repo_spec(proj, args.email, args.name)
        if spec is None:
            # scratch: just a plain dir with a file, no git.
            repo.mkdir(parents=True, exist_ok=True)
            (repo / MARKER).write_text("created by gen_fake_machine.py\n", encoding="utf-8")
            (repo / proj["files"][0]).parent.mkdir(parents=True, exist_ok=True)
            (repo / proj["files"][0]).write_text("# scratch\n", encoding="utf-8")
            print(f"  repo  {proj['name']:<18} bucket=scratch (no git)")
            continue
        when = now - timedelta(days=args.days, hours=random.randint(0, 12))
        make_repo(repo, when=when, **spec)
        # materialise the referenced files so Read/Edit targets exist on disk too.
        for f in proj["files"]:
            fp = repo / f
            fp.parent.mkdir(parents=True, exist_ok=True)
            if not fp.exists():
                fp.write_text(f"// {f}\n// ... existing code ...\n", encoding="utf-8")
        pushed = "pushed" if spec["pushed"] else "local"
        print(f"  repo  {proj['name']:<18} bucket={proj['bucket']} ({pushed}, {spec['remote_url']})")

    # 3) transcripts -------------------------------------------------------- #
    n = args.sessions
    # weight session counts toward work repos (a realistic corp machine)
    weights = {"work_pushed": 4, "work_provisional": 2, "personal": 2, "scratch": 1}
    pool = []
    for proj in PROJECTS:
        pool += [proj] * weights[proj["bucket"]]
    total_written = 0
    for i in range(n):
        proj = random.choice(pool)
        cwd = repo_cwd[proj["name"]]
        # spread start times across the window, business-hours-ish
        day_offset = random.randint(0, args.days - 1) if args.days > 1 else 0
        start = (now - timedelta(days=day_offset)).replace(
            hour=random.randint(9, 18), minute=random.randint(0, 59),
            second=random.randint(0, 59), microsecond=0)
        sid = str(uuid.uuid4())
        lines, title = build_session(proj, cwd, start, sid)
        folder = projects_dir / encode_cwd(cwd)
        folder.mkdir(parents=True, exist_ok=True)
        (folder / f"{sid}.jsonl").write_text("\n".join(lines) + "\n", encoding="utf-8")
        total_written += 1
    print(f"  wrote {total_written} transcript(s) under {projects_dir}")

    print("\nNext steps:")
    print("  1) In the dashboard, set the tenant allowlist/work-definition so the")
    print(f"     corp host/org/domain match: host={CORP_HOST} org={CORP_ORG} domain={CORP_DOMAIN}")
    print("  2) Capture:  ccguard-agent --server <url> --token ccg_... --capture")
    print("  3) Triage :  ccguard-agent --server <url> --token ccg_... --triage --force")
    print("     (the local `claude -p` judge resolves the work/personal donut)")
    print("\nUndo everything with:  python tools/gen_fake_machine.py --clean")
    return 0


def write_identity(path: Path, email: str, org_uuid: str) -> None:
    # Mirror the shapes read_active_account() probes (oauthAccount.emailAddress +
    # organizationUuid / organization.uuid).
    data = {
        "oauthAccount": {
            "emailAddress": email,
            "organizationUuid": org_uuid,
            "organization": {"uuid": org_uuid, "name": "Acme Corp"},
        },
        "numStartups": random.randint(20, 400),
        "installMethod": "npm",
    }
    path.write_text(json.dumps(data, indent=2), encoding="utf-8")


def write_credentials(path: Path, plan: str) -> None:
    # read_subscription_plan() reads claudeAiOauth.subscriptionType.
    data = {"claudeAiOauth": {"subscriptionType": plan, "scopes": ["user:inference"]}}
    path.write_text(json.dumps(data, indent=2), encoding="utf-8")


def do_clean(claude_dir: Path, work_root: Path) -> int:
    removed = 0
    # transcripts + identity/credentials we wrote
    projects = claude_dir / "projects"
    for p in (projects, claude_dir / ".credentials.json", claude_dir.parent / ".claude.json"):
        if p.exists():
            if p.is_dir():
                shutil.rmtree(p, ignore_errors=True)
            else:
                p.unlink()
            removed += 1
            print(f"  removed {p}")
    # only remove repos we created (marker file present)
    if work_root.exists():
        for child in work_root.iterdir():
            if child.is_dir() and (child / MARKER).exists():
                shutil.rmtree(child, ignore_errors=True)
                removed += 1
                print(f"  removed {child}")
    print(f"clean: removed {removed} path(s).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
