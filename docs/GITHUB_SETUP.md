# Publish to GitHub

Target repository (empty, already matches `Cargo.toml` metadata):

**https://github.com/alexdepasquale2-jpg/hades-engine**

Alternative empty repo: `alexdepasquale2-jpg/bigmmo` (change `GITHUB_REPO` in the script).

## Option A — Cursor Origin sync (recommended)

1. Connect the **Cursor GitHub app** to your account (Cursor Settings → Integrations).
2. Open [cursor.com/codebase](https://cursor.com/codebase) → **Sync from GitHub**.
3. Select **`alexdepasquale2-jpg/hades-engine`** and confirm.
4. Push this project to the Origin remote (pushes pass through to GitHub):

   ```bash
   git remote add origin-github https://origin.cursor.com/unfaithful/alexdepasquale2-jpg-hades-engine.git
   # or use the green **Code** URL from the synced repo page
   git push -u origin-github main
   ```

5. Attach future cloud agents to that synced repo so pushes use the right token scope.

## Option B — Personal access token (one-shot)

1. Create a fine-grained or classic PAT with **Contents: Read and write** on `hades-engine` (classic: `repo` scope).
   Fine-grained: select repository **hades-engine** and set **Contents** to Read and write.
2. Export it and run from this repo root:

   ```bash
   export GH_TOKEN=github_pat_...
   chmod +x scripts/push-to-github.sh
   ./scripts/push-to-github.sh
   ```

3. For **GitHub Actions** releases, add repo secret `CARGO_REGISTRY_TOKEN` when you publish to crates.io.

## Option C — Git bundle (no token on this machine)

A bundle of `main` may be attached to the agent run. On your laptop:

```bash
git clone https://github.com/alexdepasquale2-jpg/hades-engine.git
cd hades-engine
git pull /path/to/hades-engine-main.bundle main
git push origin main
```

## After the first push

- CI runs from `.github/workflows/ci.yml` on `main`.
- Tag `v0.1.0` to trigger `.github/workflows/release.yml` (needs `CARGO_REGISTRY_TOKEN` for crates.io).
- Set the GitHub repo description: *MBT-native MMORPG simulation engine (Rust)*.
