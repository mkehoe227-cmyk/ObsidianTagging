---
description: Tag and link a note using the local vector engine
argument-hint: <path/to/note.md>
---

You are tagging the Obsidian note at `$ARGUMENTS`. Follow these steps **exactly and in order**. Do not skip steps. Do not summarize or rewrite the user's prose.

## Step 1 — Run the vector engine

Execute the tagger binary to upsert the note's embedding and retrieve the 10 most similar notes:

```bash
./tools/tagger/target/release/tagger "$ARGUMENTS"
```

The output is a newline-separated list of note titles (or file paths if titles are unavailable). Capture this list as SIMILAR_NOTES.

If the binary does not exist, output this error and stop:
> Error: Rust tagger binary not found. Run `cargo build --release` in tools/tagger/ first.

## Step 2 — Read the note

Read the full content of `$ARGUMENTS`. Identify:
- The YAML frontmatter block (between the first and second `---` lines). Extract the existing `tags:` array.
- The body text (everything after the closing `---`).

## Step 3 — Read the tag registry

Read `tags.json` from the vault root. Extract the `tags` array. Count total tags as TAG_COUNT.

## Step 4 — Select tags to apply

Rules (apply ALL of them):

**4a. Cold Start (TAG_COUNT < 25):**
- You MAY generate up to 3 new tags that don't exist in the `tags` array.
- New tags must represent distinct, reusable concepts (not one-off proper nouns).
- Each new tag must be lowercase, hyphenated, no spaces (e.g. `knowledge-management`).
- Append every new tag you create to the `tags` array in `tags.json`.

**4b. Mature Vault (TAG_COUNT >= 25):**
- Prioritize tags that already exist in the `tags` array.
- You MAY create at most 1 new tag only if the note introduces a concept genuinely absent from the existing taxonomy.
- Append it to `tags.json` if created.

**4c. Merge Rule (always):**
- PRESERVE all tags already in the note's frontmatter `tags:` array.
- APPEND 1–4 non-duplicate tags from your selected set.
- The final `tags:` array = original tags UNION new tags (no duplicates, no removals).

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

**6a.** Rewrite `$ARGUMENTS` with:
- Updated YAML frontmatter `tags:` array (merged per Step 4c)
- Body text with `[[links]]` injected (per Step 5)
- All other content byte-for-byte identical

**6b.** If new tags were created, write the updated `tags.json` with the new tags appended to the array.

## Step 7 — Report

Output a brief summary:
- Tags added: list them
- Links injected: list them with the line they appear on
- New tags added to tags.json: list them (or "none")
