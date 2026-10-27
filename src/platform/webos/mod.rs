//! webOS-only lifecycle and LS2 support; never used by the desktop adapter.
#[cfg(all(feature = "webos", any(all(target_os = "linux", target_arch = "arm"), all(test, unix))))]
pub(crate) mod app_subscription;
#[cfg(all(feature = "webos", any(target_os = "linux", test)))]
pub(crate) mod compat;
#[cfg(all(feature = "webos", target_os = "linux", target_arch = "arm"))]
pub(crate) mod luna;
