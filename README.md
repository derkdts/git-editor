# Git Editor

A small desktop file editor for Git repositories built with Tauri. It lets you open or clone a local repository and edit its files, save changes, and create local commits through a simple interface.

> The interface and application messages are currently mostly in Russian. The application performs local operations only and does not send repository or file contents to a server.

## Current features

- Open an existing repository or initialize Git in the selected folder.
- Clone a repository over HTTPS, HTTP, SSH, or a `git@host:path` address into an empty folder.
- Browse repository files in an expandable folder tree, including nested directories, with modified (`M`) and untracked (`U`) status badges.
- Create a file in the repository root, edit its text, and save it to disk.
- Save changes as a local Git commit using the **Save locally** button.
- View up to 20 recent commits and read the selected file version from a commit in a separate read-only panel.
- Display the current branch and save state, and warn before losing unsaved changes.

## Architecture

- **Interface:** `index.html` contains the HTML markup, styles, and the application's main JavaScript code. It builds an expandable file tree from the recursively listed files and uses `window.__TAURI__.core.invoke` for calls to Tauri commands.
- **Type bridge:** `src/tauriBridge.ts` defines the repository state types and wrappers for Tauri command calls.
- **Backend:** `src-tauri/src/lib.rs` registers the commands; `fs_ops.rs` implements file and repository operations; `git_ops.rs` performs Git operations.
- **Git calls:** Commands run in the selected folder through the system `git` executable. Blocking operations are executed in a worker thread to keep the interface responsive.
- **Configuration:** `src-tauri/tauri.conf.json` contains the application settings and `frontendDist: "../dist"`; Rust dependencies are listed in `src-tauri/Cargo.toml`.

### Limitations of the current version

- Hidden files and directories (names beginning with `.`) are omitted from the tree; empty directories are not displayed.
- The **New file** action creates files in the repository root only.
- The editor is a plain text field with no syntax highlighting or built-in line-by-line diff.
- The save button creates a local commit for all repository changes (`git add --all`), not only for the open file.
- There are no push/pull operations or remote repository or branch management.
- For interface builds, the Tauri configuration expects a `dist` directory, but the root `package.json` does not yet contain a build script, and generated output must be arranged manually.

## Running and development

Node.js with npm, Rust with Cargo, Tauri 2 system dependencies for your OS, and Git must be installed. Git operations require Git to be available in `PATH`.

After configuring the frontend location to match `frontendDist` in `src-tauri/tauri.conf.json`:

```sh
npm install
npm run tauri dev
```

The `tauri` script in `package.json` passes arguments to the Tauri CLI. Automatic `beforeDevCommand` and frontend build scripts are not currently configured.

---

Ideas not yet implemented:

1. **Navigation throughout the repository:** Creating files in the selected directory and navigating directly to folders.
2. **Improved editor:** Syntax highlighting, line numbers, find and replace, keyboard shortcuts, and encoding selection.
3. **Real change comparison:** A line-by-line diff of the working file against `HEAD` and the selected commit, with highlighting of added and removed lines.
4. **Commit controls:** Commit message input, file selection for commits, diff preview before confirmation, and commit author settings.
5. **Branch and remote operations:** Creating and switching branches, viewing remotes, fetching, pulling, and pushing with clear status and error messages.
6. **Security and resilience:** Autosave or draft recovery, confirmation before dangerous operations, validation of repository paths, and safe handling of untrusted repositories.
7. **Build and releases:** Configure a unified frontend build pipeline, development and build commands for operating systems, and automatic versioning and packaging.

## Project structure

```text
.
├── index.html
├── package.json
├── src/
│   └── tauriBridge.ts
└── src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json
    └── src/
        ├── fs_ops.rs       # files, repository open/initialize/clone
        ├── git_ops.rs      # Git status, history, and local commits
        ├── lib.rs          # Tauri command registration
        └── models.rs       # data structures
```
