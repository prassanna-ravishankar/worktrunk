# Demo Recording

## Running the Demo

```bash
./dev/wt-demo-build
```

Creates:
- `dev/wt-demo/out/wt-demo.gif` - Animated demo (~2 MB)
- `dev/wt-demo/out/run.txt` - Text transcript
- `dev/wt-demo/out/record.log` - Recording log

The script creates a fresh temp repo under `dev/wt-demo/out/.demo-*/`, seeds 4 worktrees + 2 extra branches, shows `wt list`, creates a worktree, edits a file, merges with `wt merge`, then shows `wt list --branches --full`.

## Publishing

Demo GIFs are hosted in [worktrunk-assets](https://github.com/max-sixty/worktrunk-assets) to avoid bloating the main repo.

**Build → Preview → Publish:**

```bash
# 1. Build the demo (auto-copies to docs/static/assets/ for local preview)
./dev/wt-demo-build

# 2. Preview inline (zola serve should be running)
# View at http://127.0.0.1:1111/quickstart/

# 3. Publish when satisfied
./scripts/publish-assets
```

The script auto-clones the assets repo if needed.

### How the Paths Connect

**File locations:**
```
worktrunk/
├── dev/wt-demo/out/wt-demo.gif      # Build output (gitignored)
├── docs/static/assets/wt-demo.gif   # Local preview (gitignored)
└── docs/content/quickstart.md       # References: /assets/wt-demo.gif

../worktrunk-assets/
└── demos/wt-demo.gif                # Published asset
```

**Path 1: Local Preview**
```
./dev/wt-demo-build
    → creates dev/wt-demo/out/wt-demo.gif
    → copies to docs/static/assets/wt-demo.gif

zola serve (in docs/)
    → serves docs/static/assets/* at /assets/*
    → quickstart.md ![](/assets/wt-demo.gif) resolves to local file
```

**Path 2: Publish**
```
./scripts/publish-assets
    → copies dev/wt-demo/out/wt-demo.gif
    → to ../worktrunk-assets/demos/wt-demo.gif
    → pushes to GitHub
```

**Path 3: CI Build** (`.github/workflows/build-docs.yaml`)
```
curl https://raw.githubusercontent.com/max-sixty/worktrunk-assets/main/demos/wt-demo.gif
    → saves to docs/static/assets/wt-demo.gif

zola build
    → copies docs/static/assets/* to docs/public/assets/*
    → quickstart.md ![](/assets/wt-demo.gif) resolves correctly
```

**Path 4: README on GitHub.com**
```
README.md references:
    https://cdn.jsdelivr.net/gh/max-sixty/worktrunk-assets@main/demos/wt-demo.gif
    → jsDelivr fetches from worktrunk-assets repo
    → GitHub renders the image inline
```

**Key invariant:** All paths end with the GIF at `/assets/wt-demo.gif` (local, CI) or CDN (README).

## Viewing Results

**Do NOT use `open` on the GIF** - that's for the user to do manually.

Inline viewing options:
- Quick Look: `qlmanage -p dev/wt-demo/out/wt-demo.gif`
- iTerm2: `imgcat dev/wt-demo/out/wt-demo.gif`

For Claude Code: read `dev/wt-demo/out/run.txt` to see text output (cannot view GIFs directly).

## Prerequisites

- `wt` (worktrunk) installed and in PATH
- `vhs` for recording
- `starship` for prompt
- `llm` CLI with Claude model configured (for commit message generation)
- `cargo-nextest` for running tests
- Python 3

## Files

- `demo.tape` - VHS tape file with recording script
- `fixtures/` - Extracted content files (README, lib.rs, etc.)
- `out/` - Output directory (gitignored)

## Defaults

Light theme (GitHub-light-inspired palette), starship prompt, 1600x720 window.
