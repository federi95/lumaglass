//! Lumaglass native executable. Original launcher: Luma (MIT); see LICENSE and NOTICE.md.
mod app;
mod features;
mod platform;
mod render;
mod storage;
mod ui;

// Internal module aliases keep the mechanical move separate from behavior changes.
use app::{model, state};
use features::{
    apps::{artwork as app_artwork, branding, recents, scroll_preview},
    weather::{self, service as weather_service},
};
use platform::sdl_events;
#[cfg(all(feature = "webos", target_os = "linux", target_arch = "arm"))]
use platform::webos::app_subscription;
#[cfg(all(feature = "webos", target_os = "linux", target_arch = "arm"))]
use platform::webos::luna;
use render::{damage, diagnostics, egl_damage, font, raster, renderer, text_rendering, vector};
use ui::{i18n, view};

fn main() {
    if features::display::helper() {
        return;
    }
    // The clock worker starts this fresh process solely to resolve local time.
    // Handle it before SDL, Luna, storage, redirects or any worker is initialized.
    if std::env::args().len() == 2
        && std::env::args().nth(1).as_deref() == Some(platform::clock::HELPER)
    {
        match time::UtcOffset::current_local_offset() {
            Ok(offset) => println!("{}", offset.whole_seconds()),
            Err(_) => std::process::exit(1),
        }
        return;
    }
    app::runtime::main();
}
