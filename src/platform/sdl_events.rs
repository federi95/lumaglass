//! Preserve SDL-webOS extension keys before rust-sdl2 converts its closed
//! Scancode enum. The TV keymap can leave their keycode at SDLK_UNKNOWN (zero).
use sdl2::{EventPump, event::Event, keyboard::Keycode, sys};
use std::mem::MaybeUninit;

pub const WEBOS_EXIT_KEYCODE: i32 = (1 << 30) | 505;
const WEBOS_COLOR_FIRST: u32 = 486;

pub fn shortcut_button(key: Keycode, webos: bool) -> Option<crate::features::shortcuts::Button> {
    use crate::features::shortcuts::Button;
    if webos {
        Button::ALL.into_iter().find(|button| {
            let index = button.index() as i32;
            let value = i32::from(key);
            // Native SDK XKB color symbols and their marker-stripped payloads
            // can arrive as keycodes when the backend supplies no scancode.
            value == ((1 << 30) | (WEBOS_COLOR_FIRST as i32 + index))
                || value == 0x0120_0011 + index
                || value == 0x0020_0011 + index
        })
    } else {
        match key {
            Keycode::F1 => Some(Button::Red),
            Keycode::F2 => Some(Button::Green),
            Keycode::F3 => Some(Button::Yellow),
            Keycode::F4 => Some(Button::Blue),
            _ => None,
        }
    }
}

pub fn decode(mut raw: sys::SDL_Event, webos: bool) -> Event {
    // SAFETY: SDL always initializes the type tag; only keyboard variants have
    // a keysym. Read the scancode as its ABI integer, since native webOS values
    // are not valid discriminants of the host-generated sys::SDL_Scancode enum.
    unsafe {
        if raw.type_ == sys::SDL_EventType::SDL_KEYDOWN as u32
            || raw.type_ == sys::SDL_EventType::SDL_KEYUP as u32
        {
            let scancode = std::ptr::addr_of!(raw.key.keysym.scancode)
                .cast::<u32>()
                .read();
            if webos && scancode == 482 {
                raw.key.keysym.scancode = sys::SDL_Scancode::SDL_SCANCODE_AC_BACK;
                raw.key.keysym.sym = i32::from(Keycode::AcBack);
            } else if webos && scancode == 505 {
                raw.key.keysym.scancode = sys::SDL_Scancode::SDL_SCANCODE_ESCAPE;
                raw.key.keysym.sym = WEBOS_EXIT_KEYCODE;
            } else if webos && (WEBOS_COLOR_FIRST..WEBOS_COLOR_FIRST + 4).contains(&scancode) {
                // The checksum-pinned SDL-webOS header defines colors at 486..489.
                // Preserve them as keycodes before rust-sdl2 sees its closed enum.
                raw.key.keysym.scancode = sys::SDL_Scancode::SDL_SCANCODE_UNKNOWN;
                raw.key.keysym.sym = (1 << 30) | scancode as i32;
            } else if webos && matches!(scancode, 484 | 485) {
                // Cursor show/hide are notifications, not another physical key
                // cancelling an in-progress Back hold.
                raw.key.keysym.scancode = sys::SDL_Scancode::SDL_SCANCODE_UNKNOWN;
                raw.key.keysym.sym = 0;
            } else if !matches!(scancode, 0 | 4..=129 | 133..=164 | 176..=221 | 224..=231 | 257..=290 | 512)
            {
                // Do not probe Scancode::from_i32 here: rust-sdl2 transmutes
                // into the host enum before checking it and can abort on a TV
                // extension. These are SDL2's standard ABI discriminants.
                raw.key.keysym.scancode = sys::SDL_Scancode::SDL_SCANCODE_UNKNOWN;
            }
        }
    }
    Event::from_ll(raw)
}

pub fn poll(_pump: &mut EventPump, webos: bool) -> Option<Event> {
    let mut raw = MaybeUninit::uninit();
    // SAFETY: EventPump keeps SDL events initialized on this SDL thread. SDL
    // writes a complete event only when the call succeeds, just as poll_iter.
    unsafe { (sys::SDL_PollEvent(raw.as_mut_ptr()) == 1).then(|| decode(raw.assume_init(), webos)) }
}

pub fn wait(_pump: &mut EventPump, timeout: u32, webos: bool) -> Option<Event> {
    let mut raw = MaybeUninit::uninit();
    // SAFETY: same ownership/initialization contract as poll; preserve webOS
    // keys on the timed-wait path as well as already-queued events.
    unsafe {
        (sys::SDL_WaitEventTimeout(raw.as_mut_ptr(), timeout.min(i32::MAX as u32) as i32) == 1)
            .then(|| decode(raw.assume_init(), webos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::shortcuts::Button;

    #[test]
    fn native_colors_survive_closed_scancode_conversion_on_down_and_up() {
        for button in Button::ALL {
            for down in [true, false] {
                // Model FFI bytes, never construct an invalid Rust enum value.
                let mut raw: sys::SDL_Event = unsafe { std::mem::zeroed() };
                unsafe {
                    raw.type_ = if down {
                        sys::SDL_EventType::SDL_KEYDOWN as u32
                    } else {
                        sys::SDL_EventType::SDL_KEYUP as u32
                    };
                    std::ptr::addr_of_mut!(raw.key.keysym.scancode)
                        .cast::<u32>()
                        .write(WEBOS_COLOR_FIRST + button.index() as u32);
                }
                let key = match decode(raw, true) {
                    Event::KeyDown { keycode, .. } if down => keycode,
                    Event::KeyUp { keycode, .. } if !down => keycode,
                    other => panic!("Unexpected event: {other:?}"),
                };
                assert_eq!(shortcut_button(key.unwrap(), true), Some(button));
            }
        }
    }

    #[test]
    fn desktop_aliases_and_native_color_symbols_have_separate_scope() {
        for (index, button) in Button::ALL.into_iter().enumerate() {
            let desktop = [Keycode::F1, Keycode::F2, Keycode::F3, Keycode::F4][index];
            assert_eq!(shortcut_button(desktop, false), Some(button));
            assert_eq!(shortcut_button(desktop, true), None);
            for value in [0x0120_0011, 0x0020_0011, (1 << 30) | 486] {
                let native = Keycode::from_i32(value + index as i32).unwrap();
                assert_eq!(shortcut_button(native, true), Some(button));
                assert_eq!(shortcut_button(native, false), None);
            }
        }
        for key in [
            Keycode::Return,
            Keycode::Escape,
            Keycode::AcBack,
            Keycode::Menu,
            Keycode::F5,
            Keycode::from_i32(WEBOS_EXIT_KEYCODE).unwrap(),
        ] {
            assert_eq!(shortcut_button(key, true), None);
            assert_eq!(shortcut_button(key, false), None);
        }
    }
}
