#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::os::windows::ffi::OsStrExt;

use windows::{
    core::PCWSTR,
    Win32::{
        System::Registry::{
            RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER,
            HKEY_LOCAL_MACHINE, KEY_READ,
        },
        UI::WindowsAndMessaging::{MessageBoxW, IDOK, MB_ICONWARNING, MB_OKCANCEL},
    },
};

const WEBVIEW2_CLIENT_ID: &str = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
const WEBVIEW2_DOWNLOAD_URL: &str = "https://developer.microsoft.com/microsoft-edge/webview2/";

fn wide(value: &str) -> Vec<u16> {
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(Some(0))
        .collect()
}

fn is_valid_webview_version(version: &str) -> bool {
    let version = version.trim();
    let parts = version.split('.').map(|part| part.parse::<u64>());
    let parts = parts.collect::<Result<Vec<_>, _>>().ok();
    matches!(parts.as_deref(), Some([major, minor, build, patch]) if (*major, *minor, *build, *patch) != (0, 0, 0, 0))
}

fn registry_version(root: HKEY, subkey: &str) -> Option<String> {
    let subkey = wide(subkey);
    let value_name = wide("pv");
    let mut key = HKEY::default();
    if unsafe { RegOpenKeyExW(root, PCWSTR(subkey.as_ptr()), Some(0), KEY_READ, &mut key) }.is_err()
    {
        return None;
    }

    let mut buffer = [0_u16; 128];
    let mut size = (buffer.len() * std::mem::size_of::<u16>()) as u32;
    let result = unsafe {
        RegQueryValueExW(
            key,
            PCWSTR(value_name.as_ptr()),
            None,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    unsafe {
        let _ = RegCloseKey(key);
    };
    if result.is_err() {
        return None;
    }
    let chars = (size as usize / 2).min(buffer.len());
    let version = String::from_utf16_lossy(&buffer[..chars])
        .trim_end_matches('\0')
        .trim()
        .to_owned();
    is_valid_webview_version(&version).then_some(version)
}

fn webview2_installed() -> bool {
    let locations = [
        (
            HKEY_LOCAL_MACHINE,
            format!(r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{WEBVIEW2_CLIENT_ID}"),
        ),
        (
            HKEY_LOCAL_MACHINE,
            format!(r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{WEBVIEW2_CLIENT_ID}"),
        ),
        (
            HKEY_CURRENT_USER,
            format!(r"Software\Microsoft\EdgeUpdate\Clients\{WEBVIEW2_CLIENT_ID}"),
        ),
    ];
    locations
        .iter()
        .any(|(root, key)| registry_version(*root, key).is_some())
}

fn show_webview2_missing_message() {
    let title = wide("FileHide 无法启动");
    let message = wide("检测不到 Microsoft Edge WebView2 Runtime。\n\n点击“确定”打开微软官方下载页面，安装完成后再启动 FileHide。\n\n下载地址：\nhttps://developer.microsoft.com/microsoft-edge/webview2/");
    let result = unsafe {
        MessageBoxW(
            None,
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OKCANCEL | MB_ICONWARNING,
        )
    };
    if result == IDOK {
        let _ = std::process::Command::new("explorer.exe")
            .arg(WEBVIEW2_DOWNLOAD_URL)
            .spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::is_valid_webview_version;

    #[test]
    fn webview_version_must_not_be_empty_or_zero() {
        assert!(is_valid_webview_version("125.0.2535.51"));
        assert!(!is_valid_webview_version(""));
        assert!(!is_valid_webview_version("0.0.0.0"));
        assert!(!is_valid_webview_version("not-a-version"));
    }
}

fn main() {
    if !webview2_installed() {
        show_webview2_missing_message();
        return;
    }
    filehide_lib::run();
}
