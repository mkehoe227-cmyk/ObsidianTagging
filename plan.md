I am a software engineer building an automated, highly scalable note tagging and linking system for my Obsidian vault (which uses a PARA/Zettelkasten structure). I will be using the `claude-code` CLI to execute the tagging via a custom `/tag` command, but I need your help to develop the supporting architecture and codebase.

Because I want to ensure we build this correctly, please use your "ask user question" tool at any point if you are missing information regarding my operating system, specific Rust crate preferences, or file structures before writing the code.

Please break this project down into a multi-step implementation process. We will tackle one phase at a time.

### System Architecture & Workflow
The system is divided into two decoupled parts to ensure it scales flawlessly from 10 notes to 10,000 notes: A local Rust vector engine (for search/RAG) and the Claude CLI (for reasoning/text editing).

When I run `/tag [filename]` in my terminal, the following must happen:
1.  **The Rust Engine:** A local Rust executable processes the specified markdown file, generates a vector embedding, and updates a local index using Upsert logic. It then runs a cosine similarity search against the index and outputs the titles of the top 10 most related notes.
2.  **The Handoff:** The Claude CLI reads the outputted list of 10 note titles.
3.  **The Claude CLI:** Claude reads the note's text and a `tags.json` reference file. It appends relevant tags to the YAML frontmatter and weaves `[[Internal Links]]` into the body text based on the 10 titles provided by the Rust engine.

### Core Requirements & Edge Cases
You must account for the following strict rules in your implementation:

**1. The Reference State (`tags.json`) & Adaptive Tag Generation**
Maintain a `tags.json` file in the vault root. The LLM must pull tags primarily from this list to prevent hallucination, but must follow adaptive rules for creating new tags:
* **Cold Start (Under 25 Total Tags):** If the `tags.json` list is small, the vault is still establishing its taxonomy. You may generate up to **3 new, distinct tags** per note to help build out the classification system, and append them to the JSON file.
* **Mature Vault (25+ Total Tags):** If the taxonomy is established, prioritize using existing tags. If a fundamentally new concept is introduced, you may create a maximum of **1 new tag** per note and append it to the JSON file.

**2. The Tag Merger Rule**
When updating the YAML frontmatter, the system must act as a merger. It must preserve all existing tags (e.g., if `#journal` is already there, leave it) and only append 1-4 new, non-duplicate tags from the reference list. 

**3. The Link Exclusion Rule & Strict Prose Preservation**
When inserting inline links based on the provided note titles, the system must ONLY add `[[` and `]]` brackets to existing text. It is strictly forbidden from rewriting or summarizing my original prose. Furthermore, it must completely ignore text that is already wrapped in `[[` `]]` to avoid double-linking.

**4. The Upsert Logic (Handling Re-runs)**
I will frequently re-run `/tag` on older, updated notes. The Rust engine must hash the file path. If the file already exists in the local vector index, it must overwrite/replace the old vector with the new one (Upsert). It must not append a duplicate entry.

### The Implementation Plan
Please acknowledge these requirements and propose a step-by-step plan to build this, starting with:
* **Phase 1:** Setting up the vault structure, the `tags.json` state, and the `.claude/commands/tag.md` instruction file. 
* **Phase 2:** Scaffolding the Rust binary (crate selection for embeddings like `candle-core` or `ort`, text extraction, and local storage formats like `.safetensors` or `.bin`).
* **Phase 3:** Implementing the Rust Upsert and Cosine Similarity logic.
* **Phase 4:** End-to-end testing of the `/tag` handoff.

Are you ready to begin Phase 1? If you need any clarification on my environment or preferences, ask now.