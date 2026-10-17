# Vectro desktop

The offline desktop app for macOS, Linux and Windows: a [Tauri 2](https://v2.tauri.app) shell
around the same Svelte editor in `editor/`, with native Open / Save / Export dialogs and signed
updates pulled from GitHub Releases.

```
desktop/
├── package.json            the Tauri CLI, `npm run dev` and `npm run build`
└── src-tauri/
    ├── Cargo.toml          the Rust shell (standalone, not a root workspace member, see below)
    ├── tauri.conf.json     product name, window, bundle targets, updater endpoint and public key
    ├── capabilities/       what the window may ask the shell for (dialogs, the picked files)
    ├── icons/              generated from brand/icon-1024.png with `npm run icons`
    └── src/lib.rs          plugins, the menu, the update check
```

The editor decides at run time whether it is inside the app: `isDesktop()` in
`editor/src/lib/desktop.ts` is true when `window.__TAURI__` exists, and then Open, Save,
Export .riv and Import SVG go through `@tauri-apps/plugin-dialog` and `@tauri-apps/plugin-fs`.
In a browser the same buttons keep the file input and download link. Nothing else in the editor
knows about the shell.

## Run it

Needs Rust stable, Node 22, `wasm-pack`, and the platform's Tauri prerequisites (on macOS the
Xcode command line tools; on Linux `libwebkit2gtk-4.1-dev librsvg2-dev libgtk-3-dev
libayatana-appindicator3-dev patchelf`; on Windows WebView2, which ships with Windows 10 and 11).

```
cd editor && npm install && npm run wasm && cd ..
cd desktop && npm install
npm run dev            # starts the editor's dev server on 127.0.0.1:5180 and opens the window
npm run build          # builds wasm + editor, then the installers under src-tauri/target/release/bundle
```

A local `npm run build` bundles the app and then stops with "A public key has been found, but no
private key" unless `TAURI_SIGNING_PRIVATE_KEY` (and its password) are set in the environment: the
updater artifacts must be signed. The `.app` / installers are already written by then and run fine.

Always run the Tauri commands from `desktop/`: `beforeDevCommand` and `beforeBuildCommand` in
`tauri.conf.json` reach the editor as `../editor`. The dev server deliberately uses port 5180 so
it does not clash with `npm run dev` in `editor/` (5173).

`cargo check` works from `desktop/src-tauri` on its own. The crate is not a member of the root
Cargo workspace on purpose: `cargo test` at the root stays a core-only build and never needs
webkit2gtk or a Tauri compile.

## Updates

The app checks `https://github.com/openvectro/vectro/releases/latest/download/latest.json` once
at start (quietly) and whenever the user picks **Check for Updates…** (in the Vectro menu on
macOS, in Help elsewhere). A newer signed build is offered, downloaded, installed, and the app
restarts on the user's say-so.

Artifacts are signed with a minisign keypair. The public key sits in `tauri.conf.json` under
`plugins.updater.pubkey`; the private key never enters the repository. The app refuses an
update whose signature does not match, so losing the private key means shipping a new key with
one last manually installed release.

Two repository secrets are needed for the release workflow:

| secret | what |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | the contents of the private key file (`~/.tauri/vectro.key` on the machine that generated it) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | its password (`~/.tauri/vectro.key.password` beside it) |

## Cut a release

1. Bump `version` in `desktop/src-tauri/tauri.conf.json` and `desktop/src-tauri/Cargo.toml`
   (keep them equal; the tag below must match, `latest.json` carries this number and installed
   copies compare against it).
2. Commit, then tag and push the tag:
   ```
   git tag v0.1.0
   git push origin v0.1.0
   ```
3. `.github/workflows/desktop.yml` builds on macOS (universal), Ubuntu 22.04 and Windows with
   `tauri-apps/tauri-action`, and creates a **draft** release `Vectro v0.1.0` holding the
   `.dmg` / `.app.tar.gz`, `.deb` / `.rpm` / `.AppImage`, `.exe` / `.msi`, their `.sig`
   files and `latest.json`.
4. Open the draft on GitHub, check the assets, write the notes, and **Publish**. Only a
   published, non-prerelease release is served from `releases/latest`, so installed copies see
   the update at that moment and not before.

`workflow_dispatch` runs the same build by hand for a dry run; without a tag it uploads to a
draft named after the branch ref.

### Signing the binaries themselves

The updater signature is separate from OS code signing. Without an Apple Developer certificate
the macOS build is unsigned and not notarised: the first launch needs right-click → Open (or
System Settings → Privacy & Security → Open Anyway on recent macOS). Windows shows SmartScreen
for an unsigned installer. When certificates exist, add the usual `APPLE_CERTIFICATE`,
`APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`,
`APPLE_TEAM_ID` secrets to the workflow's `env` and tauri-action signs and notarises on its own;
Windows signing goes through `bundle.windows.certificateThumbprint` or a `signCommand`.

## Icons

```
cd desktop && npm run icons    # tauri icon ../brand/icon-1024.png → src-tauri/icons
```

It also writes `android/` and `ios/` sets; delete them, the app is desktop only.
