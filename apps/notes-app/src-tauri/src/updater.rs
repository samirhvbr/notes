//! Signed updates are checked automatically; installation is explicitly requested.
use serde::Serialize;

#[derive(Serialize)]
pub struct UpdateStatus {
    supported: bool,
    version: Option<String>,
    notes: Option<String>,
}

#[cfg(desktop)]
mod desktop {
    use super::UpdateStatus;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    };
    use tauri::{AppHandle, Manager};
    use tauri_plugin_updater::{Update, UpdaterExt};

    #[derive(Default)]
    pub struct Pending(pub Mutex<Option<Update>>);
    static BUSY: AtomicBool = AtomicBool::new(false);
    struct Guard;
    impl Guard {
        fn acquire() -> Result<Self, String> {
            BUSY.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .map(|_| Self)
                .map_err(|_| "update_busy".into())
        }
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            BUSY.store(false, Ordering::SeqCst);
        }
    }

    pub fn supported() -> bool {
        if cfg!(debug_assertions) {
            return false;
        }
        #[cfg(target_os = "linux")]
        if std::fs::read_to_string("/usr/share/tura-notes/package-manager")
            .is_ok_and(|s| s.trim() == "pacman")
        {
            return false;
        }
        use tauri::utils::{config::BundleType, platform::bundle_type};
        matches!(
            bundle_type(),
            Some(BundleType::App | BundleType::AppImage | BundleType::Deb | BundleType::Rpm)
        )
    }

    pub async fn check(app: AppHandle) -> Result<UpdateStatus, String> {
        let _guard = Guard::acquire()?;
        if !supported() {
            return Ok(UpdateStatus {
                supported: false,
                version: None,
                notes: None,
            });
        }
        let update = app
            .updater_builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| e.to_string())?
            .check()
            .await
            .map_err(|e| e.to_string())?;
        let result = UpdateStatus {
            supported: true,
            version: update.as_ref().map(|u| u.version.clone()),
            notes: update
                .as_ref()
                .and_then(|u| u.body.as_ref())
                .map(|s| s.chars().take(600).collect()),
        };
        *app.state::<Pending>().0.lock().map_err(|_| "update_lock")? = update;
        Ok(result)
    }

    pub async fn install(app: AppHandle) -> Result<(), String> {
        let _guard = Guard::acquire()?;
        if !supported() {
            return Err("update_unsupported".into());
        }
        // The UI closes the workspace through the normal save/conflict flow first.
        // Check again natively: restart must never bypass that lifecycle.
        if app
            .state::<crate::commands::App>()
            .svc
            .lock()
            .map_err(|_| "workspace_lock")?
            .workspace_id()
            .is_some()
        {
            return Err("update_workspace_open".into());
        }
        if app
            .state::<crate::commands::App>()
            .received
            .lock()
            .map_err(|_| "workspace_lock")?
            .is_some()
        {
            return Err("update_workspace_open".into());
        }
        let update = app
            .state::<Pending>()
            .0
            .lock()
            .map_err(|_| "update_lock")?
            .clone()
            .ok_or("update_unavailable")?;
        update
            .download_and_install(|_, _| {}, || {})
            .await
            .map_err(|e| e.to_string())?;
        app.restart();
    }
}

#[cfg(desktop)]
pub use desktop::Pending;

#[tauri::command]
pub async fn update_check(app: tauri::AppHandle) -> Result<UpdateStatus, String> {
    #[cfg(desktop)]
    {
        desktop::check(app).await
    }
    #[cfg(mobile)]
    {
        let _ = app;
        Ok(UpdateStatus {
            supported: false,
            version: None,
            notes: None,
        })
    }
}

#[tauri::command]
pub async fn update_install(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(desktop)]
    {
        desktop::install(app).await
    }
    #[cfg(mobile)]
    {
        let _ = app;
        Err("update_unsupported".into())
    }
}
