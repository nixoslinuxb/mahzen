# Mahzen

[![CI](https://github.com/nixoslinuxb/mahzen/actions/workflows/ci.yml/badge.svg)](https://github.com/nixoslinuxb/mahzen/actions/workflows/ci.yml)

A native, Obsidian-like markdown note-taking app written in Rust. It is
**Wayland-only** — it refuses to start under X11 or XWayland — and targets Arch
Linux.

Notes are stored as **plain Markdown files** inside a vault folder you choose:
no login screen, no password, no encryption layer. Open the app and your notes
are there; the same files open in Obsidian, VS Code or `grep`.

## Contents

- [Features](#features) · [GPU rendering](#gpu-rendering) ·
  [Editing experience](#editing-experience) ·
  [Obsidian-parity features](#obsidian-parity-features)
- [Theme](#theme) · [Attachments](#attachments) · [Icons & UX](#icons--ux) ·
  [Supported markdown](#supported-markdown)
- [Install](#install-arch-linux) · [Run](#run) ·
  [Security hardening](#security-hardening) ·
  [Keyboard shortcuts](#keyboard-shortcuts)
- [Where things are stored](#where-things-are-stored) · [Layout](#layout) ·
  [Development](#development) · [License](#license)

## Your data

- Notes are ordinary `.md` files under your vault folder, written atomically
  (temp file + rename) so a crash cannot corrupt them.
- Attachments live in `<vault>/assets/` and are ordinary files too.
- The app keeps no copy of your notes; the vault folder *is* the source of
  truth, so any other editor or tool can read and write it.

> **Migration note:** earlier versions of Mahzen (then "Encrypted Note")
  encrypted notes with AES-256-GCM. The encryption layer has been removed on
  request. If the app opens an `ENCD1` vault it shows a **one-time conversion
  screen**: enter the old master password once and every note and attachment
  is rewritten as a plain file (nothing is deleted). The AES/Argon2 code now
  exists only in `src/legacy.rs`, which can read the old format and never
  writes it.

## Features

- **Vaults** — a folder of markdown notes, with last-vault and recent-vaults
  memory, opened automatically on launch.
- **Sub-notes** — **hover** any note in the sidebar to reveal inline icons: **＋ New
  sub-note**, **✎ Rename**, **🗑 Delete** (no right-click menu). Sub-notes nest under
  the note using Obsidian's convention: `Parent.md` + `Parent/Child.md`.
- **File tree sidebar** with per-folder note counts.
- **Editor + live preview** in `Edit`, `Split`, and `Preview` modes per tab, with
  **undo/redo** (toolbar buttons, `Ctrl+Z` / `Ctrl+Shift+Z` / `Ctrl+Y`, and palette commands).
- **Tabs** with dirty indicators and autosave.
- **Wikilinks** — `[[Note]]` / `[[Note|alias]]` as clickable preview links.
- **Backlinks** and **Outline** panels.
- **Full-text search** across decrypted notes.
- A modern, rounded dark theme matching Obsidian's default palette, with a
  violet accent (see [Theme](#theme)).

## GPU rendering

All drawing is GPU-accelerated. The app renders through **`wgpu`**, which on
Wayland uses **Vulkan** by default, and it asks for a **high-performance**
(hardware) adapter. On launch it prints the chosen device once, e.g.:

```
Mahzen: GPU rendering — backend=Vulkan, device='NVIDIA GeForce RTX 3070', driver='NVIDIA 615.78.08'
```

If Vulkan isn't available, wgpu transparently falls back to an OpenGL backend.
You can influence adapter selection with the standard wgpu environment variables:
`WGPU_BACKEND` (`vulkan`, `gl`, …), `WGPU_POWER_PREF` (`high`/`low`), and
`WGPU_ADAPTER_NAME`.

On Arch, make sure a Vulkan driver is installed for the GPU in use (e.g.
`vulkan-radeon`, `vulkan-intel`, or `nvidia-utils`).

## Editing experience

- **Live Preview** — a fourth view mode (`Edit` / `Split` / `Preview` / `Live`).
  Non-active lines are rendered as markdown (with inline markup hidden); the **active
  line is a real character-accurate `TextEdit`**. Caveat: unlike Obsidian, the active
  line shows its raw markup in full rather than hiding the non-caret spans, so it is a
  *line-accurate* hybrid, not character-accurate WYSIWYG. True WYSIWYG needs a custom
  rich-text engine (not built).
- **Concealed markdown (WYSIWYG editing)** — the default editor hides inline
  markup (`**`, `*`, `~~`, backticks, `#` heading markers, `> `, wikilink
  brackets and link URLs) and styles the content instead: headings are enlarged,
  bold/italic/strikethrough are rendered, code gets a background, wikilinks look
  like links. The caret and selection stay character-accurate because the layout
  keeps the original bytes (hidden markers collapse to near-zero width).
  **Element-level reveal**: the markers of the construct the caret or selection
  is inside become visible again (`**bold**`, `[[link]]`, `[text](url)`, heading
  and quote prefixes), so editing feels like Obsidian's live preview while the
  rest of the note stays concealed. Toggle it with the palette command *Toggle
  concealed markdown*.
- **Multi-cursor editing** — `Ctrl+D` selects the word and adds the next
  occurrence as an extra caret; `Ctrl+Alt+Up` / `Ctrl+Alt+Down` add a caret on
  the line above/below; `Alt+click` places a caret anywhere. Typing, Enter, Tab,
  Backspace/Delete, word/line/arrow navigation and shift-selection apply at
  every caret at once, secondary carets and selections are drawn in the accent
  colour, and `Esc` (or a plain click) collapses back to a single caret.
- **Find / replace in the current note** — `Ctrl+F` (or `Ctrl+H`) opens an
  inline find bar with match highlighting, case toggle, next/previous, and
  **Replace** / **Replace all**. `Ctrl+Shift+F` still runs the vault-wide search.
- **Split note pane** — *Toggle second note pane* (palette) shows a second note
  beside the active one; `Ctrl+click` any tab to load it into that pane. Each
  pane keeps its own view mode and scroll position.
- **Column edit / multi-cursor helper** — *Column edit* inserts the same text at
  a column (or at the end of every line) over a line range, for the common
  "edit every selected line at once" workflow.
- **Spell check** — *Toggle spell check* underlines misspelled words using an
  offline system dictionary (`/usr/share/dict/*`, cracklib, or Hunspell `.dic`),
  skipping code, URLs, wikilink targets and frontmatter.
- **Properties editor** — the right panel lists every YAML frontmatter property
  (not just `title`/`aliases`/`tags`) and lets you add, edit (scalar or list),
  rename and delete arbitrary properties.
- **Tag management** — the tag panel is now hierarchical (`work`, `work/rust`,
  …); *Manage tags* renames, merges or deletes a tag across every note, updating
  both the frontmatter `tags` lists and inline `#tags` (subtags included).
- **Vim mode** — toggled from the command palette; a full modal editing layer
  (`src/vim.rs`): Normal/Insert/Visual, motions, operators, `p`/`P`, `r`, `u`,
  **text objects** (`iw aw i( a( i" i' i[ i{ ip` with `d`/`c`/`y`), and **`.`-repeat**.
- **Clipboard** — `Ctrl+C` / `Ctrl+X` / `Ctrl+V` (and `Ctrl+A`) work in the editor
  and every text field. On KDE/Wayland this needed a workaround: eframe enables
  egui-winit's `smithay-clipboard`, which speaks the older **wlr-data-control**
  protocol that **KWin does not implement** (KWin uses **ext-data-control**), so
  paste silently failed. A small **patched `egui-winit`** (vendored under
  `vendor/egui-winit`) now falls back to an **in-app clipboard** when the platform
  backend errors, so copy/paste works within the app everywhere. **Cross-app**
  copy/paste also works on KWin: `src/system_clipboard.rs` implements a
  **`ext-data-control`** bridge (a reader thread caches the current selection, and
  each copy serves a data source), which the vendored egui-winit consults after its
  built-in backends fail.
- **Single instance** — only one copy runs at a time; a per-user Unix socket
  (`$XDG_RUNTIME_DIR/mahzen.sock`) is bound at startup, and a second
  launch prints a message and exits instead of opening another window.

## Obsidian-parity features

- **YAML frontmatter** — `title`, `aliases`, and `tags` are parsed; the frontmatter
  block is hidden from the rendered preview, and `title` overrides the file name.
- **Tags** — inline `#tags` and frontmatter tags are indexed. The right panel lists
  every tag with a count; clicking one filters the sidebar to that tag.
- **Aliases** — `[[alias]]` resolves to the note that declares it.
- **Anchors** — `[[Note#Heading]]` and `[[Note#^block]]` resolve to the note.
- **Embeds / transclusion** — `![[Note]]` renders the other note inline (depth-limited).
- **Command palette / quick switcher** — `Ctrl+P` to fuzzy-jump to any note or run a
  command (new note, open vault, daily note, new-from-template, **open canvas**,
  cycle view, toggle panel, toggle graph, close tab, quit).
- **Daily notes** — *Open today's / previous / next daily note* commands, plus a
  monthly **calendar sidebar** for arbitrary dates. Obsidian-style: configurable
  **moment date format**, an optional **template note**, and a **Daily notes settings**
  dialog. Templates support `{{title}}`, `{{date}}`, `{{time}}` **and inline format
  tokens** `{{date:FORMAT}}` / `{{time:FORMAT}}` (e.g. `{{date:dddd, MMMM D}}`).
- **Templates** — *New note from template* builds a note from any file in
  `Templates/`, substituting `{{date}}` and `{{title}}`.
- **Graph view** — force-directed graph of notes and links, with a **local graph**
  (depth-controlled, anchored on the active note) and **filters** (tag, orphans).
  Obsidian-like: every node shows its **note title** beside it (labels are hidden
  when zoomed out and appear on hover/zoom-in or for the active note), node size
  scales with **link count**, links are **straight lines** meeting at shared
  junctions, and you can **drag to pan** and **scroll to zoom** (with a reset
  button).
- **Canvas** — a full-featured `.canvas` board editor (Obsidian's JSON format).
  **New canvas** (command palette) creates an empty board in the vault; **Open canvas
  board** opens one — or shows a **picker** when there are several. Cards:
  text/file/group/link; pan (middle/right-drag),
  zoom-to-cursor, rubber-band multi-select, drag to move, corner-resize, double-click
  to edit text in place, drag the ◉ handle to connect cards, right-click for a
  background/node context menu (new card, colour, delete), `Del` to delete, keys
  `1`–`6` to recolour, and **undo/redo** (toolbar, `Ctrl+Z` / `Ctrl+Shift+Z`). **Saved on
  close and on exit** (plus the Save button).
- **Mermaid (built-in)** — ` ```mermaid ` fences render **without any external tool**:
  a native renderer for flowcharts (`graph`/`flowchart` TD/LR, node shapes, `-->`,
  `---`, `-.->`, `==>`, edge labels, chained arrows) parses, lays out, and draws the
  diagram. Sequence diagrams and pie charts are also native (see below); any
  other Mermaid type falls back to `mmdc` (if installed) then to a source block.
- **Richer backlinks** — the right panel shows **Backlinks** (with line numbers and
  context) plus **Unlinked mentions** (title/alias appearing as plain text).
- **Workspace persistence** — open tabs, active tab, and panel state are restored.
- **Markdown extensions** — GitHub tables, callouts, footnotes, strikethrough,
  `[text](url)` links (via `xdg-open`), **`==highlight==`**, **`%%comments%%`**
  (multi-line too), inline HTML (tags are stripped, `<br>` becomes a line
  break), **nested callouts**, **sized embeds** (`![[image.png|300]]` or
  `![[image.png|300x200]]`), hover previews on wikilinks, and block references
  (`^block-id` + the *Copy block reference* command).
- **Native LaTeX math** — a real math typesetter replaced the old Unicode
  prettifier: `\frac`, `\sqrt` (with index), `^`/`_` scripts, `\sum`/`\int`
  with limits, `\left…\right` scalable delimiters, matrices
  (`pmatrix`/`bmatrix`/`vmatrix`/`matrix`), `aligned`/`cases`, `\text`,
  accents (`\overline`, `\hat`, `\vec`, `\dot`) and the usual Greek/operator
  symbols are laid out and drawn natively.
- **More Mermaid, natively** — besides flowcharts, **sequence diagrams**,
  **pie charts**, **state diagrams**, **class diagrams**, **ER diagrams**,
  **gantt**, **user journeys**, **mind maps** and **timelines** render without
  `mmdc`; anything else still falls back to the CLI when installed.
- **Visual table editing** — *Edit table* opens a grid editor for the table in
  the note: edit cells, add/remove rows and columns, then write it back.
- **Query blocks (Dataview/Bases style)** — fenced `query` blocks (triple-backtick + `query`, or `dataview`) run a
  small query language against the vault: `table`/`list`/`task`, `from #tag`
  or `"Folder"`, `where field op value` (`=`, `!=`, `>`, `<`, `contains`,
  truthiness), `sort field asc|desc` and `limit N`. Fields include
  `file.name`, `file.path`, `file.folder`, `tags`, `aliases`, `todo`, `done`
  and any frontmatter property. Expressions support `length`, `upper`,
  `lower`, `contains`, `startswith`, `endswith`, `round`, `default`,
  `dateformat`, `sum`, `min`, `max` and `expr as "alias"`; `flatten tags` and
  `group by field` (with `length(rows)`) aggregate rows.
- **Richer graph** — edges now show **direction arrows**, nodes can be coloured
  by tag via **colour groups** (persisted), and the force layout exposes
  **repulsion**/**attraction** sliders with a recompute button.
- **LaTeX extras** — `\eqref{…}` / `\autoref{…}` (parenthesised cross
  references), `\begin{array}{lcr}` column alignment (left/center/right),
  `\color{name}` (rest of the group) and `\textcolor{name}{…}` with the
  standard xcolor names, `\cancel` / `\bcancel` / `\xcancel` / `\sout`
  strike-out decorations and `\boxed{…}` frames.
- **LaTeX environments** — `\begin{equation}`/`align`/`gather`/`split`/
  `multline`/`matrix` environments, `\[ … \]`, automatic **equation
  numbering**, `\tag{…}`, `\label{…}` + `\ref{…}` cross-references, explicit
  `\left.`/`\right.` delimiters and a wider symbol set.
- **Richer canvas cards** — link cards fetch their page **title and
  description** in the background, and **PDF cards** (context menu → *New PDF
  card…*) render a **first-page preview** through `pdftoppm` when available.
- **Search power** — plain terms, `"quoted phrases"`, `tag:`, `path:`, `file:`,
  `property:key=value`, `task:todo|done`, `section:"Heading"`, `line:N` /
  `line:>N` / `line:10-20`, `/regex/`, `-` negation and **`OR`** (all other
  terms are ANDed). Find/replace in the current note is covered above.

## Workspace extras

- **Settings window** — palette command *Settings* (or the pencil icon in the
  top bar) edits font size, concealed markdown, spell check, Vim mode, default
  split view, linked pane, **UI language**, daily-notes
  folder/format/template, templates folder and the startup window mode, then
  saves everything at once.
- **UI language** — English and **Türkçe**. The interface is translated live
  from the Settings window (stored as `language` in `config.json`); missing
  translations fall back to English, so the UI never breaks.
- **Bookmarks** — star the active note (*Bookmark / unbookmark current note* or
  the star icon). The sidebar lists them and can open a **random bookmark**;
  *Open a random note* picks from the whole vault.
- **Note composer** — merge the active note into another (content appended, the
  original file removed) or split it below the caret into a new note.
- **Voice notes** — *Record a voice note* captures the default PulseAudio input
  with `ffmpeg` into `<vault>/assets/voice-<timestamp>.ogg`; run the command
  again to stop, and the embed is inserted into the active note. A red `● REC`
  timer shows in the status bar.
- **Linked second pane** — with *Toggle linked second pane*, wikilinks clicked
  in the first pane open in the second one (Obsidian's linked view).

## Portability & interop

- **Plain-Markdown export** — *Export vault as plain Markdown…* writes every
  decrypted note as an ordinary `.md` file (same folder layout) plus its
  `assets/` attachments into a folder you choose. Your vault is not
  modified, so the notes become readable by Obsidian, VS Code, `grep`, Git…
- **Markdown / Obsidian import** — *Import Markdown / Obsidian vault…* scans a
  folder and copies every `.md` and attachment into the vault. It is
  **additive and non-destructive**: the source folder is never modified, and
  name clashes become `Note (1).md`.
- **HTML / PDF / print export** — *Export current note (PDF / HTML / print)…*
  writes a self-contained HTML file (styled, with tables, code, callouts,
  highlights and images) and opens it in the browser for printing or "Save as
  PDF"; a `.pdf` target is converted with `wkhtmltopdf` when available.

## Live vault

- **External change watching** — the vault is polled (mtime + size, no inotify
  dependency) every ~1.2 s. Notes changed by another program are reloaded
  automatically; new/removed notes refresh the tree and index. If a note has
  **unsaved edits** the app keeps your buffer, flags a conflict and shows a
  status hint — `Ctrl+Shift+R` reloads from disk. Toggle in Settings.

## Free panel layout

- **Stacked tabs** — a vertical column of full-width tab rows (Settings or the
  *Toggle stacked tabs* command) instead of the horizontal strip.
- **Independent views** — *Open current note in a new view* opens the same note
  twice, each with its own view mode, scroll position and caret.
- **Drag tabs between panes** — drag a tab in the strip to move it into the
  second pane; drag a pane tab back to return it to the main pane.

## Advanced math

`\colorbox{…}{…}` / `\fcolorbox`, `\underbrace{…}_{…}`, `\overbrace{…}^{…}`,
`\stackrel{…}{…}`, `\binom`, and the `gathered`/`alignat`/`flalign`/
`subequations`/`smallmatrix`/`Vmatrix` environments. Cleveref-style references:
`\cref{a,b}` → `(1, 2)`, `\Cref{a}` → `Eq. (1)` (localised), `\crefrange{a}{b}`
→ `(1)–(2)`, `\labelcref`, `\cpageref`.

## Dataview & Bases

- **Wider function set** — `date`, `year`, `month`, `day`, `weekday`, `reldate`,
  `dateformat`/`format`, `number`, `floor`, `ceil`, `abs`, `mod`, `replace`,
  `substring`, `split`, `join`, `unique`, `count`, `truncate`, `pad`,
  `striptime`, `coalesce`/`nonnull`, `length`, `upper`, `lower`, `contains`,
  `startswith`, `endswith`, `round`, `default`, `sum`, `min`, `max`.
- **Card view** — add `view cards` to a `table` query for a **Bases-style grid
  of cards** (title + field rows) instead of a table.

## Canvas

- **Directional links** — edges carry an `arrow` flag (on by default) and honour
  `fromSide`/`toSide` anchors; right-click a connector to **toggle the
  arrowhead** or **reverse its direction**.
- **PDF cards** — show a real page preview with `‹ p.N ›` paging controls (each
  page rendered by `pdftoppm` and cached); double-click opens the PDF at that
  page in the system viewer.
- **Web cards** — fetch the page title/description and open the URL on
  double-click (a web *view* still requires the system browser).

### Not implemented (yet)

The markdown/visualisation gap list is closed: real LaTeX (with numbered
equations and cross-references), native Mermaid for flowcharts, sequence, pie,
state, class, ER, gantt, journey, mindmap and timeline, the extra markdown
syntaxes, table editing, query blocks with functions/flatten/group-by, richer
graph and canvas cards, and the extended search language are all implemented.
**Sync**, **plugin APIs**, and **plain-Markdown export/import** remain out of
scope by request; arbitrary third-party Mermaid diagram types still fall back
to `mmdc`.

## Logo

<p align="center"><img src="assets/logo.svg" width="96" alt="Mahzen logo"></p>

The app logo lives at `assets/logo.svg` — a rounded violet-gradient tile with a
note and a lock. It's shown on the welcome screen and set as the window icon at
startup (via `resvg`).

**Taskbar / system-monitor icon.** A Wayland window has no icon protocol: panels
resolve an app's icon from the `.desktop` file whose name matches the window's
**app id**. On launch the app installs, idempotently, into `~/.local/share`:

- `applications/mahzen.desktop` (with `Icon=mahzen` and
  `StartupWMClass=mahzen`)
- `icons/hicolor/scalable/apps/mahzen.svg`

so panels can show the logo. If your panel was already running, refresh its cache
(e.g. re-login, or run `kbuildsycoca6` on KDE). `packaging/mahzen.desktop`
is provided for a system-wide `pacman` install too.

## Theme

The interface follows Obsidian running the **LYT Mode** theme: deep blue-black
sidebars and ribbon (`#1e1f24`, `#1a1b21`), a lighter blue-grey editor surface
(`#2f303a`), periwinkle-blue accents (`#7c9ce0`), a periwinkle selection
(`#54587d`), muted `#9a9dae` secondary text on `#dcdde3`, and soft `#3a3b47`
borders with small corner radii.

Details borrowed from that theme:

- **Coloured headings** — H1 lavender, H2 periwinkle, H3 steel blue, then sage,
  sand and rose.
- **Yellow text highlight** — `==highlight==` renders as LYT's warm yellow with
  dark text, in both the preview and the editor.
- **Tabs** — a dark strip with the active tab lifted to the editor colour and an
  accent line along its top edge; `Ctrl+click` still targets the second pane.
- **Ribbon** — a narrow, darker icon strip on the far left.
- **Side panels** — uppercase micro-headers, rounded selected rows with an
  accent edge, and tag pills.
- **Calendar card** — month header with `TODAY`, weekday row and an accent-filled
  circle for today plus a dot on days that have a daily note.
- **Breadcrumb + inline title** — vault ▪ folders ▪ note above an oversized
  coloured note title.
- **Vertical tab strips (sliding panes)** — narrow columns down the left edge of
  the editor whose titles are rotated 90° (reading top → bottom, like
  Obsidian's sliding panes), with a gold edge on the active note, a dirty dot,
  and working **preview** and **close** buttons at the bottom (toggle in
  Settings).
- **Selected file preview** — the selected note in the explorer expands into a
  rounded block showing its opening lines.
- **Mini graph in the sidebar** — a read-only force-directed graph docked at the
  bottom of the left panel; click a node to open that note.
- **Tag pane** — a `#` header with a sort-by-count toggle, hierarchical tags
  with chevrons and right-aligned counts.
- **Coloured link states** — resolved wikilinks are periwinkle, unresolved ones
  warm orange; list bullets and numbers are gold; headings carry a fold chevron.
- **Status bar** — a mode chip, the current message and `N words  M characters`.
- **Resize-safe layout** — panes never squeeze the editor to nothing:
  - the vertical tab strip is capped at ~30% of the editor width and shrinks its
    tabs; with too many tabs it falls back to the wrapping horizontal bar,
  - the split preview and the second pane are laid out as **explicit
    rectangles** with a draggable divider (not nested `SidePanel`s), so
    widening one pane can never draw it over the other; the divider keeps a
    minimum width for both sides and its share is remembered in
    `config.json` (`split_frac`),
  - side-panel width limits and the sidebar-graph height range scale with the
    window, and the mini graph is hidden on short windows,
  - the ribbon and right-panel action rows wrap instead of overflowing,
  - leaving fullscreen restores the last windowed size instead of a
    zero/tiny window, and the saved startup window mode is applied once the
    app starts.
- **Reading surface** — wider line height, rounded bordered code blocks, tinted
  callouts with an accent bar, and a quoted block with an accent rule.

## Attachments

Attach **images, voice notes (audio), and videos** to a note from the **Attach:**
bar above the editor (image / mic / video icons). The file is copied into
`<vault>/assets/` and a markdown reference is inserted into the note (recorded
in the note's undo history, so **undo/redo covers attachments** too). In the
preview and live preview the note's attachments are shown at the top, **scroll
with the note**, and **play inline — no external player window opens**:

- **Images** are decoded in-process (PNG / JPEG / GIF / BMP via the `image`
  crate, SVG via `resvg`) — **WEBP is not supported** — rendered inline, and are
  **resizable** (drag the accent handle in the bottom-right corner) and **movable**
  (drag the image itself). The layout is remembered per attachment in
  `media-layout.json`.
- **Audio / voice notes** get an inline player with **play / pause / stop**, a
  **click-to-seek scrubber**, and a **live volume** slider. Audio is decoded to
  raw PCM by `ffmpeg`, volume-scaled in-process, and streamed through PipeWire
  (`pw-cat`) or ALSA (`aplay`).
- **Video** is decoded by `ffmpeg` into RGBA frames rendered as an egui texture,
  with the same **play / pause / stop** controls, **click-to-seek scrubber**, and
  **volume** slider; a video's soundtrack plays through the same audio path as
  its frames render, so you get **picture and sound together** in the note.

**Seeking** — click anywhere on the scrubber (or drag it and release) to jump to
that position; pause remembers the position and resume continues from it.
**Volume** — the slider (0–150%) applies instantly to whatever is playing and is
remembered in `media-layout.json`.

Playback uses the `ffmpeg` / `ffprobe` / `pw-cat` (or `aplay`) programs found
on `PATH`; nothing is linked into the binary, and only one item plays at a time
(playback stops on its own when you navigate away from the note or close the
app).

## Icons & UX

Every control in the app is a hand-drawn **SVG icon** (see `assets/icons/`)
rather than text or emoji: the toolbar (a **vertical strip docked top-left**),
tabs, sidebar tree and dialogs. The icons are monochrome SVGs embedded at
compile time and rasterised by `egui_extras`' SVG loader, then tinted to match
the current state (normal / hovered / selected / disabled). Hover any control to
see a tooltip describing it.

## Supported markdown

Headings, **bold**, *italic*, ~~strikethrough~~, `inline code`, bullet and numbered
lists, task list checkboxes, blockquotes, callouts, fenced code blocks, GitHub
tables, footnotes, horizontal rules, `[text](url)` links, Obsidian-style
`[[wikilinks]]` (with `|alias`, `#heading`, `#^block` anchors) and `![[embeds]]`.

## Install (Arch Linux)

This build is Wayland-only. It links against Wayland client libraries (and no
X11/XCB libraries):

```sh
sudo pacman -S --needed base-devel wayland libxkbcommon mesa
```

Build and install:

```sh
cargo build --release
sudo install -Dm755 target/release/mahzen /usr/local/bin/mahzen
sudo install -Dm644 packaging/mahzen.desktop \
    /usr/local/share/applications/mahzen.desktop
```

The app also installs a per-user desktop entry and icon into
`~/.local/share` on first launch (see [Logo](#logo)).

### Runtime tools (optional)

Some features shell out to programs found on `PATH`; everything else is built in:

| Feature | Needs |
| --- | --- |
| Web-card metadata / PDF page previews | `curl`, `pdftoppm` |
| Audio / video attachments | `ffmpeg`, `ffprobe`, and `pw-cat` (PipeWire) or `aplay` (ALSA) |
| Mermaid diagrams other than flowcharts | `mmdc` (Mermaid CLI) |
| Opening external links | `xdg-open` |

Built-in Markdown, math, and Mermaid **flowcharts** need none of these.

## Run

The app opens as a **normal (windowed) window**. Press **`F11`** to cycle
**fullscreen → maximized → windowed** (or use the *Cycle window mode* command); the
fullscreen mode is true borderless fullscreen (no title bar), which works on KDE/KWin.
It also asks the compositor for keyboard focus on the first frame.

```sh
cargo run --release
```

`WAYLAND_DISPLAY` must be set; otherwise the app prints an error and exits. The
binary is compiled without any X11/XCB backend, so it cannot fall back to X11 or
XWayland.

## Security hardening

The code has been audited and hardened:

- **No shell is ever used** — every external tool (`curl`, `ffmpeg`, `ffprobe`,
  `pw-cat`, `aplay`, `mmdc`, `xdg-open`) is run via `Command` with argument arrays,
  so note/feed content cannot inject shell commands.
- **`curl` is sandboxed** — only `http(s)` URLs, `-q` (ignores `.curlrc`),
  `--proto`/`--proto-redir =http,https` (a malicious feed cannot redirect to
  `file://` to read local files), `--max-redirs 5`, and `--` so the URL can never be
  parsed as a curl option.
- **Path confinement** — media/attachment paths taken from note text are resolved
  with `vault_file()`, which rejects absolute paths and `..`, canonicalises, and
  requires the result to stay inside the vault (defeats symlink traversal).
- **Single-instance lock** lives in `$XDG_RUNTIME_DIR` (or the per-user config dir),
  never a world-writable `/tmp` path.
- **Link handlers are scheme-restricted** (`http`/`https`/`mailto` only); XML is
  parsed with `roxmltree` (no DTD/entity expansion, so no XXE / billion-laughs);
  regexes use the linear-time `regex` crate (no ReDoS).
- **Clipboard fallback** — the vendored `egui-winit` never returns `None` for paste
  when the platform backend errors (KWin's ext-data-control is unsupported by
  smithay), and the built-in `ext-data-control` bridge (`src/system_clipboard.rs`)
  provides cross-application copy/paste on KWin.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl+N` | New note |
| `Ctrl+O` | Open vault |
| `Ctrl+S` | Save current note |
| `Ctrl+Z` | Undo |
| `Ctrl+Shift+Z` / `Ctrl+Y` | Redo |
| `Ctrl+P` | Command palette / quick switcher |
| `Ctrl+F` | Focus search |
| `Ctrl+W` | Close current tab |
| `Ctrl+A` | Select all (editor and text fields) |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | Copy / cut / paste |
| `F11` | Cycle window mode (fullscreen/maximized/windowed) |

## Where things are stored

- Config and workspace state: `$XDG_CONFIG_HOME/mahzen/config.json`
- Media playback volume + image layout: `$XDG_CONFIG_HOME/mahzen/media-layout.json`
- Your notes: wherever your vault folder is, as plain `.md` files
- Attachments: `<vault>/assets/`, referenced from notes
- Single-instance socket: `$XDG_RUNTIME_DIR/mahzen.sock`
- Installed desktop entry + icon: `$XDG_DATA_HOME/applications/mahzen.desktop`
  and `$XDG_DATA_HOME/icons/hicolor/scalable/apps/mahzen.svg`

`$XDG_CONFIG_HOME`, `$XDG_DATA_HOME`, and `$XDG_RUNTIME_DIR` fall back to
`~/.config`, `~/.local/share`, and the temp dir respectively.

## Layout

```
src/
  main.rs       entry point, Wayland-only guard, eframe bootstrap
  app.rs        the egui UI, palette, graph, tags, editor and dialogs
  icons.rs      SVG icon set + icon-button helpers
  theme.rs      the dark colour scheme and spacing
  frontmatter.rs minimal YAML frontmatter + inline tags
  index.rs      title/alias/tag/link indexes and wikilink resolution
  palette.rs    fuzzy matching for the command palette
  graph.rs      deterministic force-directed graph layout
  dates.rs      moment-style date formatting, day/month arithmetic, calendar
  vault.rs      vault scanning, tree building, note CRUD, daily notes
  markdown.rs   markdown renderer (tables, callouts, footnotes, embeds)
  math.rs       LaTeX → Unicode math prettifier
  mermaid.rs    built-in Mermaid flowchart parser, layout, and renderer
  search.rs     note search query language (phrases, tag:, path:, file:, regex)
  canvas.rs     Obsidian-compatible `.canvas` documents (nodes, edges)
  vim.rs        modal Vim editing layer (motions, operators, text objects)
  config.rs     persisted configuration and workspace state
  desktop.rs    installs the desktop entry + icon for panel/taskbar display
  media.rs      in-app audio/video playback and image layout
  system_clipboard.rs  ext-data-control clipboard bridge (KWin cross-app copy/paste)
assets/
  logo.svg      app logo (welcome screen, window icon, installed icon)
  icons/*.svg   the icon set
packaging/
  mahzen.desktop  system-wide desktop entry
vendor/
  egui-winit/   patched egui-winit (clipboard fallback for KWin)
```

## Development

```sh
cargo test      # unit tests: icons, frontmatter, index, palette, graph, math,
                # mermaid, dates, search, canvas, vim, markdown, vault,
                # query, portability, watch, media
cargo clippy    # lints
```

There is one debugging flag: `cargo run -- --clip-selftest` starts a Wayland
clipboard instance, writes a test string, reads it back, and exits — useful for
checking the `ext-data-control` bridge on a compositor.

## License

MIT — see [LICENSE](LICENSE).
