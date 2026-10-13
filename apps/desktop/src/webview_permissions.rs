//! The main window grants the microphone to the app's own page, so "Test microphone" never shows
//! WebView2's "wants to use your microphones" prompt, and a Block clicked there cannot break the
//! test for good. Every other permission and every other origin keeps WebView2's default.

/// The origin of the bundled frontend (`useHttpsScheme` is off, the Tauri default).
const APP_ORIGIN: &str = "http://tauri.localhost";

/// The origins that get the microphone: the bundled frontend and, in a dev build only, the dev
/// server.
#[cfg_attr(not(windows), allow(dead_code))]
fn own_origins(dev_url: Option<&tauri::Url>) -> Vec<String> {
    let mut origins = vec![APP_ORIGIN.to_string()];
    if let Some(url) = dev_url.filter(|_| tauri::is_dev()) {
        origins.push(url.origin().ascii_serialization());
    }
    origins
}

/// `uri` (a page or frame URL) belongs to one of `origins`.
#[cfg_attr(not(windows), allow(dead_code))]
fn is_own_origin(uri: &str, origins: &[String]) -> bool {
    tauri::Url::parse(uri)
        .map(|url| url.origin().ascii_serialization())
        .is_ok_and(|origin| origins.contains(&origin))
}

/// Allows the microphone for the app's origins in `webview`. The bundled origin's Allow is saved
/// in the WebView2 profile, which also replaces a Block saved before this existed (WebView2 may
/// stop asking once a state is saved). Each request from an app origin is allowed too, without
/// saving, so the dev server's grant (dev and release share the profile) ends with the session.
#[cfg(windows)]
pub fn grant_own_microphone(
    webview: &tauri::webview::PlatformWebview,
    dev_url: Option<&tauri::Url>,
) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2PermissionRequestedEventArgs, ICoreWebView2PermissionRequestedEventArgs3,
        ICoreWebView2Profile4, ICoreWebView2_13, COREWEBVIEW2_PERMISSION_KIND,
        COREWEBVIEW2_PERMISSION_KIND_MICROPHONE, COREWEBVIEW2_PERMISSION_STATE_ALLOW,
    };
    use webview2_com::{
        take_pwstr, PermissionRequestedEventHandler, SetPermissionStateCompletedHandler,
    };
    use windows::core::{Interface, HSTRING, PWSTR};

    let core = match unsafe { webview.controller().CoreWebView2() } {
        Ok(core) => core,
        Err(error) => {
            log::warn!("Microphone permission: no CoreWebView2: {error}");
            return;
        }
    };

    let saved = unsafe {
        core.cast::<ICoreWebView2_13>()
            .and_then(|core| core.Profile())
    }
    .and_then(|profile| profile.cast::<ICoreWebView2Profile4>())
    .and_then(|profile| {
        let done = SetPermissionStateCompletedHandler::create(Box::new(|result| {
            if let Err(error) = result {
                log::warn!("Microphone permission: saving Allow failed: {error}");
            }
            Ok(())
        }));
        unsafe {
            profile.SetPermissionState(
                COREWEBVIEW2_PERMISSION_KIND_MICROPHONE,
                &HSTRING::from(APP_ORIGIN),
                COREWEBVIEW2_PERMISSION_STATE_ALLOW,
                &done,
            )
        }
    });
    if let Err(error) = saved {
        log::warn!("Microphone permission: saving Allow failed: {error}");
    }

    let origins = own_origins(dev_url);
    let allow =
        move |args: &ICoreWebView2PermissionRequestedEventArgs| -> windows::core::Result<()> {
            let mut kind = COREWEBVIEW2_PERMISSION_KIND::default();
            unsafe { args.PermissionKind(&mut kind)? };
            if kind != COREWEBVIEW2_PERMISSION_KIND_MICROPHONE {
                return Ok(());
            }
            let mut uri = PWSTR::null();
            unsafe { args.Uri(&mut uri)? };
            if !is_own_origin(&take_pwstr(uri), &origins) {
                return Ok(());
            }
            if let Ok(args) = args.cast::<ICoreWebView2PermissionRequestedEventArgs3>() {
                unsafe { args.SetSavesInProfile(false)? };
            }
            unsafe { args.SetState(COREWEBVIEW2_PERMISSION_STATE_ALLOW) }
        };
    let handler = PermissionRequestedEventHandler::create(Box::new(move |_, args| {
        if let Some(args) = args {
            if let Err(error) = allow(&args) {
                log::warn!("Microphone permission: request not answered: {error}");
            }
        }
        Ok(())
    }));
    let mut token = 0;
    if let Err(error) = unsafe { core.add_PermissionRequested(&handler, &mut token) } {
        log::warn!("Microphone permission: no PermissionRequested handler: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_app_origin_gets_the_microphone() {
        let origins = own_origins(None);
        assert_eq!(origins, vec![APP_ORIGIN.to_string()]);
        assert!(is_own_origin("http://tauri.localhost/", &origins));
        assert!(is_own_origin(
            "http://tauri.localhost/onboarding?step=1",
            &origins
        ));
        for other in [
            "https://tauri.localhost/",
            "http://evil.example/",
            "http://tauri.localhost.evil.example/",
            "http://localhost:1420/",
            "http://tauri.localhost:8080/",
            "file:///C:/fala/index.html",
            "",
            "not a url",
        ] {
            assert!(!is_own_origin(other, &origins), "{other}");
        }
    }

    #[test]
    fn the_dev_server_counts_only_in_a_dev_build() {
        let dev = tauri::Url::parse("http://localhost:1420").unwrap();
        let origins = own_origins(Some(&dev));
        assert_eq!(
            is_own_origin("http://localhost:1420/", &origins),
            tauri::is_dev()
        );
        assert!(!is_own_origin("http://localhost:1421/", &origins));
    }
}
