//! The desktop shell around the Vectro editor: a window on `editor/dist`, native file dialogs
//! (through the dialog and fs plugins, called from `editor/src/lib/desktop.ts`), and signed
//! updates from GitHub Releases.

use tauri::menu::{Menu, MenuItem, MenuItemKind, PredefinedMenuItem};
use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

const CHECK_FOR_UPDATES: &str = "check-for-updates";

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .menu(build_menu)
        .on_menu_event(|app, event| {
            if event.id() == CHECK_FOR_UPDATES {
                let app = app.clone();
                tauri::async_runtime::spawn(async move { check_for_updates(app, true).await });
            }
        })
        .setup(|app| {
            // A quiet check at start: it only speaks up when there is something to install.
            let app = app.handle().clone();
            tauri::async_runtime::spawn(async move { check_for_updates(app, false).await });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Vectro");
}

/// The platform's standard menu with one addition: "Check for Updates…", in the app menu on
/// macOS and in Help elsewhere. The editor's own shortcuts (⌘Z, ⌘C, ⌘A …) keep working because
/// the webview handles a key it has claimed before the menu sees it.
fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::default(app)?;
    let check = MenuItem::with_id(app, CHECK_FOR_UPDATES, "Check for Updates…", true, None::<&str>)?;
    let items = menu.items()?;
    let target = if cfg!(target_os = "macos") {
        items.first()
    } else {
        items.iter().find(|item| matches!(item, MenuItemKind::Submenu(s) if s.text().ok().as_deref() == Some("Help")))
    };
    if let Some(MenuItemKind::Submenu(submenu)) = target {
        if cfg!(target_os = "macos") {
            // After "About Vectro".
            submenu.insert(&PredefinedMenuItem::separator(app)?, 1)?;
            submenu.insert(&check, 2)?;
        } else {
            submenu.append(&check)?;
        }
    }
    Ok(menu)
}

/// Asks GitHub Releases for a newer signed build. `announce` is true for the menu entry, so the
/// user always hears back; the start-up check stays silent unless an update is waiting.
async fn check_for_updates<R: Runtime>(app: AppHandle<R>, announce: bool) {
    let say = |text: String, kind: MessageDialogKind| {
        app.dialog().message(text).title("Vectro").kind(kind).blocking_show();
    };
    let ask = |text: String, yes: &str| -> bool {
        app.dialog()
            .message(text)
            .title("Vectro")
            .kind(MessageDialogKind::Info)
            .buttons(MessageDialogButtons::OkCancelCustom(yes.to_string(), "Later".to_string()))
            .blocking_show()
    };

    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(e) => {
            if announce {
                say(format!("Could not check for updates: {e}"), MessageDialogKind::Error);
            }
            return;
        }
    };

    match updater.check().await {
        Ok(Some(update)) => {
            let wanted = ask(
                format!(
                    "Vectro {} is available; this is {}.\nDownload and install it now?",
                    update.version, update.current_version
                ),
                "Install",
            );
            if !wanted {
                return;
            }
            match update.download_and_install(|_, _| {}, || {}).await {
                Ok(()) => {
                    if ask("The update is installed. Restart Vectro now?".to_string(), "Restart") {
                        app.restart();
                    }
                }
                Err(e) => say(format!("The update could not be installed: {e}"), MessageDialogKind::Error),
            }
        }
        Ok(None) => {
            if announce {
                say(
                    format!("You are on the latest version, Vectro {}.", app.package_info().version),
                    MessageDialogKind::Info,
                );
            }
        }
        Err(e) => {
            if announce {
                say(format!("Could not check for updates: {e}"), MessageDialogKind::Error);
            }
        }
    }
}
