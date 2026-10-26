# Native controls and navigation

## Controls and navigation

### Documentation screenshots

`pnpm screenshots` generates `media/home.png`, `media/apps.png`, `media/settings.png` and
`media/inputs.png`, linked by the README. It builds the desktop executable without the TV feature,
uses isolated temporary preferences, sample inventory and a fixed date/time, and captures actual
1920×1080 OpenGL framebuffers. Weather is unconfigured, so no real weather APIs are contacted.
It does not execute the test suites. A desktop display is required; on headless Linux use
`xvfb-run -a pnpm screenshots`. These images document the native design, not TV hardware behavior.

### Input

| Input                         | Action                                                        |
| ----------------------------- | ------------------------------------------------------------- |
| Arrows                        | Move focus, revealing remote-focused content                  |
| OK / Enter                    | Activate on release                                           |
| Hold OK for 600 ms            | Reorder Home favorite or open app options                     |
| OK or Back in move mode       | Finish reordering without launching                           |
| Escape / Back / Backspace     | Parent navigation, overlay dismissal or Recent apps from Home |
| Hold Back / Escape for 600 ms | Open Recent apps while Lumaglass is focused                   |
| Menu / M / right-click        | App options                                                   |
| Pointer / wheel               | Hover, click, hold and scrolling                              |
| F5                            | Refresh inventory                                             |

Backspace deletes while editing. Native Back uses SDL's webOS-specific keycode. Back dismisses
the keyboard first; Weather Location then returns to its parent Location row, HDMI choices to
that HDMI row, and the Background gallery to Background in the sidebar. Back from the sidebar
focuses the Settings tab, then returns Home. **System** in the sidebar groups
**Refresh apps & inputs**, **Text rendering**, **TV information** and the vector
power-icon **Quit app** action separately from General preferences.

**TV information** contains a manual Refresh action and seven read-only fields:
model, webOS SDK version, firmware, elevated helper access, Home replacement and
installation location, plus other detected Home startup integrations. Arrows navigate the rows; Back/Left restores the TV
information row in System. Data is read when opened or refreshed, without
continuous polling or system changes. Unknown, denied and unavailable values
remain distinct; the desktop preview labels TV data as not applicable. A failed
refresh keeps the last snapshot and displays a failure notice. Home ownership
reflects the mounts visible to the helper, not a hardware compatibility guarantee.

The **Clock** subpage contains **Clock format** (12-hour or 24-hour) and **Time format** (`HH:MM`
or `HH:MM:SS`, defaulting to `HH:MM`). Choosing a format returns to its row in Clock, and Back
from Clock restores its General row. With seconds shown, the header clock redraws once per second
instead of once per minute. Existing saved hour and seconds preferences are retained.
General has an **App names** subpage with independent **Show on Home** and **Show on Apps** toggles.
Home captions default to off and Apps captions default to on, stored independently in `show_home_names`
and `show_app_names`. The legacy `show_names` preference does not override these defaults. Hidden
captions do not reserve layout space.
The **Greeting** subpage contains **Name** and **Clear name**. Selecting Name opens the text keyboard
(the system keyboard on the TV) and the name is appended to the Home greeting ("Good evening, Ana.").
The keyboard starts empty; closing it without a name preserves the saved value. Clear name removes
it. Preferences are saved when the keyboard closes, including automatic system-keyboard dismissal.
Back from either subpage restores its General row. Input name removal also uses **Clear name**.
Choice checkmarks identify saved values, while the bright row fill identifies only the current focus.

Home favorites form one horizontal strip, never a second row. Left/Right follows saved order,
including reorder mode, and reveals complete cards. Up returns to Home navigation; Down enters
the nearest input in the first row. The strip position survives page changes. Wheel movement over
favorites scrolls horizontally; elsewhere it scrolls vertically. Pointer hover/down never moves
the clicked control. Apps Up/Down follows the six-column grid order, including offscreen rows,
and only the first row's Up route reaches the category toolbar. Scrolling Apps content fades into
the existing wallpaper at the header/footer, with one alpha mask for surfaces, shadows, rings,
vector icons/logos and text. The wallpaper/header are unmasked; no blur texture is needed.

TV editing uses the system keyboard through SDL-webOS. Desktop editing uses the custom panel.
System text events update app/location queries; Done/Back stops editing and restores focus.

### Recent apps

Press Back on Home to open Recent apps, or hold Back (Escape on desktop) for 600 ms from any page.
Short Back navigates on release. Back in an overlay, the system keyboard or
move mode keeps its immediate dismissal behavior; Backspace keeps its editing/navigation role.
Losing window focus cancels an unfinished hold. This is an app-local shortcut while Lumaglass is focused.
SDL-webOS Back (scancode 482) and long-Back Exit (505) are normalized before rust-sdl2 decodes the
event, since the native keymap can leave their keycodes unset and the Rust scancode enum does not
include them. Both Back and Exit access policies are claimed before video initialization. The
native Exit signal opens the switcher without waiting for another timer; repeat/dismissal events
are consumed until release, so the same press cannot dismiss an overlay and then reopen it from Home.

The switcher uses an overlapping landscape-card stack on a dimmed wallpaper, a short, reversible
crossfade to and from the actual parent interface, and a non-bouncy, interruptible depth transition.
The front card is largest; neighboring cards recede in scale and brightness. Cards are painted and
hit-tested back-to-front, so only exposed portions of rear cards can be selected with the pointer.
Reopening during dismissal reverses from the current opacity and keeps the visible cards.
Dismissal restores parent input immediately while the outgoing cards finish fading. The transition
does not move card hit targets; pointer interruption pauses only carousel movement, not the fade.
Reduced motion makes the crossfade, navigation and close transitions immediate.
Pointer hover/down preserves the current card positions and interrupts remote carousel movement.
The backdrop uses 86% opacity so more of the wallpaper shows through. There are no app captions, action buttons,
empty-state subtitle, heading, separate dismiss button, card count, footer instructions or Home
header shortcut. Up immediately lifts the focused card off the top of the screen; after a successful
close, the remaining cards slide into the gap and the next card comes forward. A failed close
returns the card to the stack. Exiting cards have no hit target, and closing the final card finishes
its exit before showing the empty state. Held-Up repeats are ignored to prevent
unintentionally closing more apps as the next card becomes focused. Back dismisses the switcher; Quit app remains an
explicit Settings → System action.

| Input                 | Action                                      |
| --------------------- | ------------------------------------------- |
| Left / Right or wheel | Browse recent cards                         |
| OK on a card          | Launch the app                              |
| Up from a card        | Close that card immediately                 |
| Pointer               | Select a card                               |
| Back                  | Return to the exact previous page and focus |

The platform worker uses the stock-Home operations captured on webOS 26:

| Operation | Luna endpoint                                 | Payload                                                                                            |
| --------- | --------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| History   | `com.webos.service.recent/listRecents`        | `{}`                                                                                               |
| Close one | `com.webos.service.recent/removeRecent`       | `{"appId":"APP_ID"}`                                                                               |
| Launch    | `com.webos.service.applicationmanager/launch` | `{"id":"APP_ID","params":{},"reason":"recent","checkUpdateOnLaunch":true,"autoInstallation":true}` |

Service-name capitalization is significant. These fixed calls go through the native process's
authenticated Homebrew Channel `exec` connection, using bounded root `luna-send` commands and no
application impersonation. Mutations are sent once, checked for `returnValue: true`, then history
is refreshed. They are not automatically retried after an ambiguous timeout. One request is active
per overlay; generation IDs discard late replies after dismissal or reopening. Read/action errors
stay in the overlay with a read-only Retry action and diagnostic details in stderr.

History order is preserved, including input entries, with Lumaglass/Home identities and duplicates
excluded. The native integration still needs manual verification in both normal-tile and
Home-takeover modes, including app/input launching, closing individual apps, short/long Back,
focus restoration and service-denial handling.

Cards share the Home/Apps face renderer: the same exact-ID curated vectors, surfaces, relative logo
proportions, installed icons and generic fallbacks. Recent splash/snapshot artwork is not loaded.
Installed icons keep the existing artwork worker and bounded caches. No TV images or recent history
are bundled or saved into preferences. System and OTNCABI installation roots are supported by the
bounded reader.

Desktop preview uses in-memory sample history and simulated mutations, with no TV calls:

```sh
pnpm dev --script recents,await-recents
```

`hold-back` is also a scripted opening action. Desktop smoke scenarios render normal, close-one,
empty, launch, pointer and Home Back flows; unit tests cover hold timing, cancelled
presses, stale responses, failures, navigation, reduced motion, reversible crossfades and consistent card artwork.
