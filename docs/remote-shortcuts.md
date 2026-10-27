# Remote shortcuts

**Settings → General → Remote shortcuts** assigns navigation actions to the red,
green, yellow and blue buttons. Shortcuts are **off by default**. Enable them
with OK on the first row; OK on a button row cycles its action. Reset restores
the preset below and turns shortcuts off. Preferences survive restarts.

| Button | Preset action | Desktop alias |
| ------ | ------------- | ------------- |
| Red    | Search apps   | F1            |
| Green  | Favorites     | F2            |
| Yellow | Inputs        | F3            |
| Blue   | Recent apps   | F4            |

Each button offers Off, Search apps, Favorites, Inputs, Apps, Settings and Recent
apps. Search opens the Apps keyboard. Favorites focuses the first visible
favorite, or Browse when none remain. Inputs focuses the first visible Home
input; when the section is hidden or no input is visible, the current page stays
open and a notice explains why. Apps opens the app list and Settings opens
Lumaglass General. Recent apps opens the existing switcher and requests its
history. These shortcuts do not launch or close an app or activate an input;
subsequent explicit interactions use the normal launcher controls.

## Scope and received-button status

Shortcuts work only inside Lumaglass while its own window is visible and focused.
Shift, Control, Alt, Command/GUI and Mode combinations are ignored. An open
dialog, keyboard, recent-app overlay, favorite-reordering session or pending app
launch blocks them. The configuration page also blocks shortcut navigation so
buttons can be observed without leaving Settings. Repeats and duplicate downs
are consumed until release; an ignored press is not queued for later. Focus loss
cancels held-button bookkeeping. Arrows, OK, Back, Menu and F5 keep their existing
roles. There is no system-wide key remapper or background input service.

The four rows report whether a corresponding button event has been received while
focused during the current application session, including while shortcuts are off.
This status is transient and resets when Lumaglass restarts; it is not a compatibility promise.
Desktop F1–F4 report preview input and are not recognized as color aliases on TV.
TV reception depends on the remote, firmware and SDL backend. Reception and
behavior on the target TV have not yet been verified.

## Native input mapping

The native decoder normalizes webOS color scancodes 486–489 before rust-sdl2
converts its closed scancode enum. It also accepts the SDK's XKB color symbols
`0x01200011`–`0x01200014` and their marker-stripped keycode payloads
`0x00200011`–`0x00200014`, for backends that supply no color scancode. Back, Exit
and cursor notifications retain their separate handling.

The bundled headers come from the checksum-pinned
[SDL-webOS release](https://github.com/webosbrew/SDL-webOS/releases/tag/release-2.30.12-webos.5)
configured in [webos-runtime.mjs](../tools/webos/webos-runtime.mjs); the native SDK
also defines the XKB symbols. The
[client-webos input notes](https://github.com/punktfunk/client-webos/blob/main/docs/NOTES.md)
document an observed red keycode with a missing scancode. These are protocol
references; no implementation code was imported from that project.

Shortcut bindings are stored in the local `shortcuts` preference block.
Missing blocks keep shortcuts disabled. Invalid enabled values become false,
invalid individual actions become Off, and unrelated valid preferences remain
available. This feature does not change TV settings or Home takeover.
