---
description: Tag and link a note using the local vector engine
argument-hint: <path/to/note.md>
---

You are tagging the Obsidian note at `$ARGUMENTS`. Follow these steps **exactly and in order**. Do not skip steps. Do not summarize or rewrite the user's prose.

## Step 1 — Run the vector engine

Execute the tagger binary to upsert the note's embedding and retrieve the 10 most similar notes:

```bash
/Users/mitchkehoe/Desktop/ClaudeTest/ObsidianTagging/tools/tagger/target/release/tagger \
  --index-dir /Users/mitchkehoe/Desktop/ClaudeTest/ObsidianTagging/.tagger/index \
  "$ARGUMENTS"
```

The output is a newline-separated list of note titles (or file paths if titles are unavailable). Capture this list as SIMILAR_NOTES.

If the binary does not exist, output this error and stop:
> Error: Rust tagger binary not found. Run `cargo build --release` in `/Users/mitchkehoe/Desktop/ClaudeTest/ObsidianTagging/tools/tagger/` first.

## Step 2 — Read the note

Read the full content of `$ARGUMENTS`. Determine the note's tag format:

**Case A — YAML frontmatter** (the very first line of the file is exactly `---`):
- Extract the existing `tags:` array from the frontmatter block (between the first and second `---`).
- Set TAG_MODE = YAML.
- Body = everything after the closing `---`.

**Case B — Inline hashtags** (first line is NOT `---`):
- Find the line that starts with `Tags:` anywhere in the file. Extract any existing `#word` tokens from that line.
- Set TAG_MODE = INLINE.
- Body = the entire file content.

## Step 3 — Read the tag registry

Read `/Users/mitchkehoe/Desktop/ClaudeTest/ObsidianTagging/tags.json`. Extract the `tags` array. Count total tags as TAG_COUNT.

## Step 4 — Select tags to apply

Rules (apply ALL of them):

**4a. Cold Start (TAG_COUNT < 50):**
- You MAY generate up to 3 new tags that don't exist in the `tags` array.
- New tags must represent distinct, reusable concepts (not one-off proper nouns).
- Each new tag must be lowercase, hyphenated, no spaces (e.g. `knowledge-management`).
- Append every new tag you create to the `tags` array in `tags.json`.

**4b. Mature Vault (TAG_COUNT >= 50):**
- Prioritize tags that already exist in the `tags` array.
- You MAY create at most 2 new tags only if the note introduces concepts genuinely absent from the existing taxonomy.
- Append them to `tags.json` if created.

**4c. Merge Rule (always):**
- PRESERVE all tags already on the note (YAML array entries or inline `#hashtag` tokens).
- APPEND 1–4 non-duplicate tags from your selected set.
- Final tag set = original UNION new (no duplicates, no removals).

## Step 5 — Select links to inject

From SIMILAR_NOTES, identify titles that appear verbatim (or near-verbatim) as phrases in the note's body text.

Rules:
- ONLY wrap existing text. Never add words, insert new sentences, or rephrase.
- Ignore text already wrapped in `[[` `]]` — do not double-link.
- Maximum 3 links injected per run.
- Match case-insensitively but preserve the original capitalization inside the brackets.

Example — if SIMILAR_NOTES contains "Zettelkasten Method" and the body contains:
  `The zettelkasten method is a system...`
→ Replace with: `The [[Zettelkasten Method|zettelkasten method]] is a system...`

Use the pipe syntax `[[Target|display text]]` when the casing differs.

## Step 6 — Write changes

**6a.** Rewrite `$ARGUMENTS` with the tag changes and `[[links]]` injected (per Step 5):

**If TAG_MODE = YAML:**
- Update the `tags:` array in the YAML frontmatter (merged per Step 4c).
- Do NOT alter any other frontmatter keys or the body structure.

**If TAG_MODE = INLINE:**
- Find the `Tags:` line. Append new tags as space-separated `#tag` tokens after the existing ones.
- Do NOT create YAML frontmatter. Do NOT add `---` delimiters.
- All other lines byte-for-byte identical.

**6b.** If new tags were created, write the updated `/Users/mitchkehoe/Desktop/ClaudeTest/ObsidianTagging/tags.json` with the new tags appended to the array.

## Step 7 — Report

Output a brief summary:
- Tags added: list them
- Links injected: list them with the line they appear on
- New tags added to tags.json: list them (or "none")
