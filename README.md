# ObsidianTagging

Automated tag and `[[WikiLink]]` injection for Obsidian vaults. A local Rust vector engine finds semantically similar notes; Claude Code reasons over the results and edits your markdown — without rewriting your prose.

---

## Architecture

```
/tag 00-Inbox/my-note.md
        │
        ▼
┌─────────────────────────┐
│  Rust tagger binary     │  generates embedding (MiniLM-L6-v2)
│  tools/tagger/          │  upserts into local flat-file index
│                         │  returns top-10 similar note titles
└────────────┬────────────┘
             │ stdout (note titles)
             ▼
┌─────────────────────────┐
│  Claude Code CLI        │  reads note + tags.json
│  .claude/commands/tag   │  selects tags (adaptive rules)
│                         │  injects [[links]] into body
│                         │  writes changes back to disk
└─────────────────────────┘
```

---

## Prerequisites

- **Rust** ≥ 1.75 (`rustup` recommended)
- **Claude Code** CLI — [install](https://claude.ai/code)
- Internet access on first run (model weights auto-download from HuggingFace ~90 MB, cached after that)

---

## Setup

```bash
# 1. Clone
git clone https://github.com/mkehoe227-cmyk/ObsidianTagging.git
cd ObsidianTagging

# 2. Build the tagger binary
cd tools/tagger
cargo build --release
cd ../..
```

---

## Usage

Open the vault root in Claude Code, then run:

```
/tag 00-Inbox/my-note.md
```

Claude will:
1. Run the Rust binary to embed the note and retrieve the 10 most similar notes
2. Read the note and `tags.json`
3. Merge new tags into YAML frontmatter
4. Inject `[[WikiLinks]]` where similar note titles appear verbatim in your prose
5. Write the updated file and report what changed

---

## Tag Rules

| Condition | Behaviour |
|---|---|
| `tags.json` has < 25 tags (cold start) | May create up to **3 new tags** per run; appends them to `tags.json` |
| `tags.json` has ≥ 25 tags (mature vault) | Prioritises existing tags; at most **1 new tag** if concept is genuinely absent |
| Always | Preserves all existing frontmatter tags; appends 1–4 non-duplicates |

---

## Link Injection Rules

- Only wraps text **already present** in the note body — never adds words or rephrases
- Skips text already inside `[[ ]]`
- Maximum **3 links injected** per run
- Uses pipe syntax when case differs: `[[Target|display text]]`

---

## Vault Structure

```
ObsidianTagging/
├── 00-Inbox/        # unsorted new notes
├── 01-Projects/     # active project notes
├── 02-Areas/        # ongoing responsibilities
├── 03-Resources/    # reference material
├── 04-Archive/      # inactive notes
├── tags.json        # shared tag registry
└── tools/tagger/    # Rust vector engine
```

---

## Key Files

| File | Role |
|---|---|
| `tools/tagger/src/main.rs` | CLI entry point; wires extract → embed → index |
| `tools/tagger/src/embed.rs` | HuggingFace MiniLM-L6-v2 embedding via candle |
| `tools/tagger/src/index.rs` | Flat-file vector store; upsert + cosine similarity |
| `tools/tagger/src/extract.rs` | Markdown parser; extracts title and body |
| `.claude/commands/tag.md` | Claude Code `/tag` slash command definition |
| `tags.json` | Vault-level tag registry with version field |

---

## How the Index Works

Vectors are stored as raw `f32` bytes in `.tagger/index/vectors.bin` (384 dims × 4 bytes per entry). A `manifest.json` alongside it maps each row to a note path and title. The tagger uses SHA-256 of the file path as a stable ID for upsert — re-tagging an updated note replaces its vector in-place.

The index directory is gitignored; it lives only on your local machine.
