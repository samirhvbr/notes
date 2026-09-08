//! Startup decisions that must happen before the WebView exists.

/// What we decided about the dmabuf renderer, and why.
#[derive(Debug, PartialEq, Eq)]
pub struct DmabufDecision {
    pub applied: bool,
    pub explanation: String,
}

fn no(why: impl Into<String>) -> DmabufDecision {
    DmabufDecision {
        applied: false,
        explanation: why.into(),
    }
}

pub fn session_kind() -> &'static str {
    if !cfg!(target_os = "linux") {
        return "other";
    }
    match std::env::var("XDG_SESSION_TYPE").as_deref() {
        Ok("wayland") => "wayland",
        Ok("x11") => "x11",
        _ if std::env::var_os("WAYLAND_DISPLAY").is_some() => "wayland",
        Ok(_) => "other",
        Err(_) => "unknown",
    }
}

/// Whether the **proprietary** NVIDIA kernel driver is loaded.
///
/// The distinction from `nouveau` is the whole point of this function, because
/// only the proprietary stack has the GBM failure this workaround exists for —
/// nouveau's GBM works, and turning the DMA-BUF renderer off there would cost
/// compositing performance for nothing.
///
/// They are told apart by what each one creates, not by a name:
///
/// * `/proc/driver/nvidia/version` is created by the proprietary kernel module
///   and by nothing else. It is the proof.
/// * `/sys/module/nvidia/` is the proprietary module's own sysfs directory.
///   Nouveau's module is `nouveau`, and it never creates `nvidia`.
/// * `nvidia-smi` on `PATH` is a weaker hint — it can be installed in a
///   container with no driver loaded — and it is never shipped by nouveau, so it
///   cannot produce a false positive *for this distinction*. It is kept because
///   the asymmetry of the failures makes a false positive cheap: applying the
///   workaround needlessly costs a little compositing performance, and not
///   applying it costs the window.
pub fn nvidia_proprietary() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }
    std::path::Path::new("/proc/driver/nvidia/version").exists()
        || std::path::Path::new("/sys/module/nvidia").exists()
        || which_nvidia_smi()
}

/// True when the open-source `nouveau` driver is loaded. Reported in
/// diagnostics; it deliberately does **not** trigger the workaround.
pub fn nouveau_present() -> bool {
    cfg!(target_os = "linux") && std::path::Path::new("/sys/module/nouveau").exists()
}

fn which_nvidia_smi() -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join("nvidia-smi").exists()))
        .unwrap_or(false)
}

/// The decision, as a pure function of its inputs.
///
/// **The display server is not one of them, and that is the correction.** This
/// function used to require Wayland, on the belief that the DMA-BUF renderer was
/// a Wayland path. WebKitGTK has used it on **X11 too since 2.42**, and the
/// failure is in NVIDIA's GBM rather than in the compositor, so it happens on
/// both. Observed on Debian 13 / X11 / NVIDIA:
///
/// ```text
/// [notes] dmabuf: not needed — session is not wayland
/// src/nv_gbm.c:288: GBM-DRV error (nv_gbm_create_device_native): …failed (ret=-1)
/// KMS: DRM_IOCTL_MODE_CREATE_DUMB failed: Permission denied
/// Failed to create GBM buffer of size 1100x720: Permission denied
/// [notes] window main: close requested
/// [notes] window main: destroyed
/// ```
///
/// That is the window disappearing on Open Folder, and it had nothing to do with
/// the file chooser: the GBM buffer for the new surface cannot be created, the
/// window goes, and Tauri ends its loop with status 0. See
/// `docs/DECISIONS-0.1b.md` D-20 and ADR-033.
///
/// `setting` is `auto` | `off` | `force`. **A missing or unreadable settings
/// file must arrive here as `auto`, never as `off`** (`docs/ARCHITECTURE.md`
/// §12): not applying the workaround costs the window, and applying it
/// needlessly costs slightly slower compositing.
pub fn decide(
    linux: bool,
    setting: &str,
    already_set: Option<&str>,
    nvidia_proprietary: bool,
) -> DmabufDecision {
    if !linux {
        return no("not applicable — linux only");
    }
    if let Some(v) = already_set {
        return no(format!(
            "left alone — already set in the environment to {v:?}"
        ));
    }
    match setting {
        "off" => return no("disabled by settings.linux.webkit_dmabuf_workaround = off"),
        "force" => {
            return DmabufDecision {
                applied: true,
                explanation: "applied — forced by settings".into(),
            }
        }
        _ => {}
    }
    if !nvidia_proprietary {
        return no("not needed — no proprietary nvidia driver");
    }
    DmabufDecision {
        applied: true,
        explanation: "applied — proprietary nvidia driver detected".into(),
    }
}

/// Read the setting and apply. Called before `tauri::Builder`: WebKit reads the
/// variable when it creates a WebView, so calling this later silently does
/// nothing.
pub fn apply(setting: &str) -> DmabufDecision {
    let preset = std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").ok();
    let d = decide(
        cfg!(target_os = "linux"),
        setting,
        preset.as_deref(),
        nvidia_proprietary(),
    );
    if d.applied {
        // SAFETY: single-threaded, before any WebView or other thread exists.
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The case the old rule got wrong, and the one that cost a window.
    #[test]
    fn applies_on_x11_with_nvidia() {
        let d = decide(true, "auto", None, true);
        assert!(d.applied, "{}", d.explanation);
    }

    #[test]
    fn applies_on_wayland_with_nvidia() {
        assert!(decide(true, "auto", None, true).applied);
    }

    /// Without the proprietary driver there is no GBM failure to work around,
    /// and turning the renderer off would cost performance for nothing. This is
    /// also the nouveau case: `nvidia_proprietary()` is false there.
    #[test]
    fn skips_without_the_proprietary_driver() {
        assert!(!decide(true, "auto", None, false).applied);
    }

    #[test]
    fn never_touches_non_linux() {
        assert!(!decide(false, "auto", None, true).applied);
    }

    #[test]
    fn off_wins_over_detection() {
        assert!(!decide(true, "off", None, true).applied);
    }

    #[test]
    fn force_applies_without_any_nvidia() {
        assert!(decide(true, "force", None, false).applied);
    }

    #[test]
    fn never_overrides_a_value_the_user_set() {
        let d = decide(true, "force", Some("0"), true);
        assert!(!d.applied);
        assert!(d.explanation.contains("already set"));
    }

    /// The degrade rule: an unreadable settings file must reach `decide` as
    /// `auto`. Anything unrecognised is treated as `auto` for the same reason —
    /// the failure of not applying it is a window that does not render.
    #[test]
    fn an_unknown_setting_degrades_to_auto_not_off() {
        assert!(decide(true, "", None, true).applied);
        assert!(decide(true, "garbage", None, true).applied);
    }

    /// `session_kind` still answers, because the diagnostics panel reports it —
    /// it just no longer decides anything.
    #[test]
    fn the_session_is_still_reported_even_though_it_no_longer_decides() {
        assert!(matches!(
            session_kind(),
            "wayland" | "x11" | "other" | "unknown"
        ));
    }
}
