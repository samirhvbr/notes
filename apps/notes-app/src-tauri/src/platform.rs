//! Startup environment decisions that must happen before the webview exists.

/// What we decided about the dmabuf renderer, kept so the diagnostics panel can
/// show it. Milestone 0.0 acceptance criterion 1 is that this is applied
/// automatically, so "applied" has to be observable — and testable — rather
/// than assumed.
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

/// Linux display server, as far as the environment admits.
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

/// Whether an NVIDIA kernel module is loaded. Read-only probe of two paths that
/// exist only when the proprietary driver is in use.
pub fn nvidia_present() -> bool {
    cfg!(target_os = "linux")
        && (std::path::Path::new("/sys/module/nvidia").exists()
            || std::path::Path::new("/proc/driver/nvidia/version").exists())
}

/// The decision, as a pure function of its inputs.
///
/// Split out from the environment so it can be tested for every combination
/// this machine cannot reproduce — a Wayland session with an NVIDIA module is
/// exactly the case that must work and cannot be checked on the machine that
/// wrote it.
pub fn decide_dmabuf(
    linux: bool,
    opt_out: bool,
    already_set: Option<&str>,
    session: &str,
    nvidia: bool,
) -> DmabufDecision {
    if !linux {
        return no("not applicable — linux only");
    }
    if opt_out {
        return no("disabled by NOTES_NO_DMABUF_WORKAROUND");
    }
    if let Some(v) = already_set {
        return no(format!("left alone — already set in the environment to {v:?}"));
    }
    if session != "wayland" {
        return no("not needed — session is not wayland");
    }
    if !nvidia {
        return no("not needed — wayland without an nvidia module");
    }
    DmabufDecision {
        applied: true,
        explanation: "applied — wayland + nvidia detected".to_string(),
    }
}

/// WebKitGTK on Wayland with the NVIDIA driver has a long history of a black or
/// flickering window, and the known mitigation is to turn off the dmabuf
/// renderer. WebKit reads this variable when the webview is created, so this
/// must run before `tauri::Builder` — calling it later silently does nothing.
///
/// Three things it deliberately does not do: it does not touch the variable on
/// any platform but Linux; it does not override a value the user already set;
/// and it can be switched off with `NOTES_NO_DMABUF_WORKAROUND=1`, because a
/// workaround with no escape hatch becomes a bug of its own the day the driver
/// is fixed.
pub fn apply_dmabuf_workaround() -> DmabufDecision {
    let preset = std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").ok();
    let decision = decide_dmabuf(
        cfg!(target_os = "linux"),
        std::env::var_os("NOTES_NO_DMABUF_WORKAROUND").is_some(),
        preset.as_deref(),
        session_kind(),
        nvidia_present(),
    );
    if decision.applied {
        // SAFETY: single-threaded, before any webview or other thread exists.
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }
    decision
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_on_wayland_with_nvidia() {
        assert!(decide_dmabuf(true, false, None, "wayland", true).applied);
    }

    #[test]
    fn skips_wayland_without_nvidia() {
        assert!(!decide_dmabuf(true, false, None, "wayland", false).applied);
    }

    #[test]
    fn skips_x11_even_with_nvidia() {
        assert!(!decide_dmabuf(true, false, None, "x11", true).applied);
    }

    #[test]
    fn never_touches_non_linux() {
        assert!(!decide_dmabuf(false, false, None, "wayland", true).applied);
    }

    #[test]
    fn opt_out_wins_over_detection() {
        let d = decide_dmabuf(true, true, None, "wayland", true);
        assert!(!d.applied);
        assert!(d.explanation.contains("NOTES_NO_DMABUF_WORKAROUND"));
    }

    #[test]
    fn never_overrides_a_value_the_user_set() {
        let d = decide_dmabuf(true, false, Some("0"), "wayland", true);
        assert!(!d.applied);
        assert!(d.explanation.contains("already set"));
    }
}
