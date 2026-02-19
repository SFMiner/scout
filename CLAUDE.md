# Scout — Writing App

## Project Overview
Scout is a desktop-first structured writing app for long-form fiction. It is built with Tauri + SvelteKit + TipTap. It is a personal tool intended to replace Atticus for the author's fiction writing workflow.

## Stack
- **Desktop wrapper**: Tauri 2.x (Rust backend)
- **Frontend framework**: SvelteKit (SPA mode, SSR disabled)
- **Editor**: TipTap (ProseMirror-based)
- **Package manager**: npm
- **Language**: TypeScript + Svelte

## Project Structure
```
scout/
├── src/
│   ├── lib/
│   │   ├── types.ts                      # Shared TypeScript types
│   │   ├── stores.ts                     # Svelte stores (project, chapters, font, etc.)
│   │   ├── fileIO.ts                     # Frontend → Tauri command bridge
│   │   ├── customDictionaryExtension.ts  # ProseMirror plugin for custom spellcheck
│   │   ├── StartupModal.svelte           # Project open/create screen
│   │   ├── ExportModal.svelte            # RTF export UI
│   │   ├── ImportModal.svelte            # Text/Markdown import UI
│   │   ├── FontModal.svelte              # Font selection UI
│   │   └── FindReplaceModal.svelte       # Find & replace UI
│   └── routes/
│       ├── +layout.ts                    # SPA mode config, do not modify
│       └── +page.svelte                  # Main app UI (single page)
├── src-tauri/
│   ├── src/
│   │   ├── main.rs                       # Entry point only — calls lib::run()
│   │   └── lib.rs                        # All Tauri commands and backend logic
│   └── tauri.conf.json                   # App identifier: com.seanm.scout
└── CLAUDE.md
```

## Internal Storage Format
- Projects are stored as a folder on disk
- `project.json`: project metadata (title, author, chapter order, export dir, font)
- `chapters/<id>.json`: one TipTap JSON document per chapter (numeric IDs)
- `custom_dictionary.json`: project-level custom spellcheck words
- `assets/`: images referenced by chapters (not yet used)

TipTap JSON is the canonical internal format. Do not use Markdown as internal storage.

**App-level config** is stored at `{APP_CONFIG_DIR}/Scout/`:
- `config.json`: last opened project path, global font preference
- `custom_dictionary.json`: global custom spellcheck dictionary (shared across all projects)

## Editor Features

### Implemented
- Two-panel layout: sidebar (chapter list) + editor surface
- TipTap editor with Bold, Italic, H2–H6, Blockquote toolbar
- Placeholder text ("Start writing...")
- In-memory chapter switching with auto-save (1 second debounce)
- Chapter rename (double-click in sidebar)
- Chapter multi-select mode for export
- Font selection (app-level and project-level, persisted)
- Find & replace across current chapter or all chapters
- RTF export (single or multiple chapters)
- Text and Markdown import (with chapter splitting by delimiter)
- **Two-tier custom dictionary**: right-click any word → "Add to Dictionary (Global)" or "Add to Dictionary (Project)". Words are stored in JSON files and suppressed from spellcheck immediately via ProseMirror decorations (`spellcheck="false"`). Dictionary words are loaded on project open.

### Next priorities
- EPUB, DOCX, Markdown export via Pandoc
- DOCX and Markdown import via Pandoc
- Image insertion (full-bleed)

### Stretch goals (do not implement unless asked)
- Footnotes
- Callout boxes
- Custom chapter themes
- Volumes and Parts
- View/manage dictionary words modal

## Key Conventions
- All indentation uses **tabs**, not spaces
- Svelte component styles use scoped `<style>` blocks; global TipTap styles use `:global()`
- All Tauri commands (Rust) live in `src-tauri/src/lib.rs` — `main.rs` is entry point only
- Frontend-to-backend calls go through `src/lib/fileIO.ts`
- Do not add features not listed above without confirming with the user first
- Do not refactor working code unless asked

## Custom Dictionary Architecture
The custom dictionary bypasses Chromium's built-in spellchecker (which ignores external files) using a ProseMirror decoration plugin:

- `customDictionaryExtension.ts` maintains a module-level `Set<string>` of lowercased custom words
- On every doc change or when a word is added, it scans the document and applies `Decoration.inline({ spellcheck: 'false' })` to matching spans
- The browser respects `spellcheck="false"` on inline elements, removing the red underline immediately
- `getDictionaryWords(projectPath)` (Tauri command) loads both global + project dictionaries on project open
- `add_to_dictionary` (Tauri command) saves to the appropriate JSON file; the frontend then calls `addDictionaryWord()` + dispatches a meta-transaction to trigger immediate re-decoration

## Export Pipeline
Currently exports to RTF only (implemented directly in Rust via `export_project` command). Pandoc-based export (EPUB, DOCX, Markdown) is planned but not yet implemented.

## Out of Scope
- PDF export (handled externally if ever needed)
- Mobile or web deployment
- Cloud storage (user handles this via Git)
- Volumes and Parts (not needed for current projects)
