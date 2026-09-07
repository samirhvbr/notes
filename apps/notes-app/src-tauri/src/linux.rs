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

pub fn nvidia_present() -> bool {
    cfg!(target_os = "linux")
        && (std::path::Path::new("/sys/module/nvidia").exists()
            || std::path::Path::new("/proc/driver/nvidia/version").exists()
            || which_nvidia_smi())
}

fn which_nvidia_smi() -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join("nvidia-smi").exists()))
        .unwrap_or(false)
}

/// The decision, as a pure function of its inputs.
///
/// Split out so the Wayland + NVIDIA case — which the machine this was written
/// on cannot produce — is covered by a test rather than by hope.
///
/// `setting` is `auto` | `off` | `force`. **A missing or unreadable settings
/// file must arrive here as `auto`, never as `off`** (`docs/ARCHITECTURE.md`
/// §12): not applying the workaround yields a black window, applying it
/// needlessly yields slightly slower compositing.
pub fn decide(
    linux: bool,
    setting: &str,
    already_set: Option<&str>,
    session: &str,
    nvidia: bool,
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
    if session != "wayland" {
        return no("not needed — session is not wayland");
    }
    if !nvidia {
        return no("not needed — wayland without an nvidia driver");
    }
    DmabufDecision {
        applied: true,
        explanation: "applied — wayland + nvidia detected".into(),
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
        session_kind(),
        nvidia_present(),
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

    #[test]
    fn applies_on_wayland_with_nvidia() {
        assert!(decide(true, "auto", None, "wayland", true).applied);
    }
    #[test]
    fn skips_wayland_without_nvidia() {
        assert!(!decide(true, "auto", None, "wayland", false).applied);
    }
    #[test]
    fn skips_x11_even_with_nvidia() {
        assert!(!decide(true, "auto", None, "x11", true).applied);
    }
    #[test]
    fn never_touches_non_linux() {
        assert!(!decide(false, "auto", None, "wayland", true).applied);
    }
    #[test]
    fn off_wins_over_detection() {
        assert!(!decide(true, "off", None, "wayland", true).applied);
    }
    #[test]
    fn force_applies_even_without_nvidia() {
        assert!(decide(true, "force", None, "x11", false).applied);
    }
    #[test]
    fn never_overrides_a_value_the_user_set() {
        let d = decide(true, "force", Some("0"), "wayland", true);
        assert!(!d.applied);
        assert!(d.explanation.contains("already set"));
    }
    /// The degrade rule: an unreadable settings file must reach `decide` as
    /// `auto`. Anything unrecognised is treated as `auto` for the same reason.
    #[test]
    fn an_unknown_setting_degrades_to_auto_not_off() {
        assert!(decide(true, "", None, "wayland", true).applied);
        assert!(decide(true, "garbage", None, "wayland", true).applied);
    }
}
