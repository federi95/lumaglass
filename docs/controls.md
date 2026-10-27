# Native controls and navigation

## Controls and navigation

**Settings → General → Home personalization** configures greeting mode/size,
date format and weekday, section visibility/order and startup focus. Use OK to
toggle/cycle values, or open an arrow-marked editor. See
[Home personalization](home-personalization.md) for defaults and fallback rules.

**Settings → General → Remote shortcuts** configures the four color buttons
(F1–F4 in desktop preview). Shortcuts start off; OK cycles bindings and Reset
restores the disabled preset. The page reports received buttons without leaving
Settings. See [remote shortcuts](remote-shortcuts.md) for actions and input scope.

**Settings → General → Two-row Home favorites** switches between the default
single row and an optional two-row layout immediately. Favorite order is kept
left to right across the first row, then the second. With at least two favorites,
both rows are used: six favorites become three per row. Both rows share horizontal
scrolling beyond two full rows at the selected card density (twelve cards at
Standard). Hidden and uninstalled apps do not occupy slots.
Up/Down moves between rows before reaching the navigation bar or inputs. While
reordering, all four arrows move the selected favorite; OK/Back finishes.
Captions keep their space and inputs move below the last row, with vertical
scrolling when needed. The setting is saved without restarting the launcher.

Long localized Home greetings scale down within the existing heading width,
preserving the name and space for the weather summary. Short greetings keep
their original size.

App options distinguish **Hide in Lumaglass** from **Uninstall app**. Eligible
removal targets get a separate name/ID confirmation with Cancel selected first.
Pending removals can be inspected through **Uninstall status**; **Check status**
only reads the inventory. See [app management](app-management.md) for metadata
requirements, desktop simulation and the meaning of an unconfirmed result.

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
| Color buttons / desktop F1–F4 | Configured navigation when shortcuts are enabled              |

Backspace deletes while editing. Native Back uses SDL's webOS-specific keycode. Back dismisses
the keyboard first; Weather Location then returns to its parent Location row, HDMI choices to
that HDMI row, and the Background gallery to Background in the sidebar. Back from the sidebar
focuses the Settings tab, then returns Home. **System** in the sidebar groups
**Refresh apps & inputs**, **Text rendering**, **TV information**, **Privacy** and the vector
power-icon **Quit app** action separately from General preferences.

**TV information** contains a manual Refresh action and seven read-only fields:
model, webOS SDK version, firmware, elevated helper access, Home replacement and
installation location, plus other detected Home startup integrations. Arrows navigate the rows; Back/Left restores the TV
information row in System. Data is read when opened or refreshed, without
continuous polling or system changes. Unknown, denied and unavailable values
remain distinct; the desktop preview labels TV data as not applicable. A failed
refresh keeps the last snapshot and displays a failure notice. Home ownership
reflects the mounts visible to the helper, not a hardware compatibility guarantee.

**Privacy** has an explicit Refresh action: Live Plus,
Limit ad tracking, and seven selected consent records. The consent labels
distinguish granted/not granted; a granted consent does not establish that
tracking is active, and a missing field never becomes an off value. Unknown,
unavailable and denied observations remain explicit. Arrows reveal all nine
rows; Back/Left restores Privacy in System. OK on a known Live Plus or limit-ad-tracking value opens a confirmed change/restore
flow. Consent records stay read-only. Desktop preview labels every TV value as not applicable.
See [Privacy and blocker](privacy-and-blocker.md).

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

Home favorites use one row by default, or two when enabled. Left/Right follows saved order
within a row and reveals complete cards; Up/Down moves between favorite rows before
the neighboring visible section, following the configured section order. The strip
position survives page changes. Wheel movement over favorites scrolls horizontally;
elsewhere it scrolls vertically. Pointer hover/down never moves
the clicked control. Apps Up/Down follows the selected card density's grid order, including offscreen rows,
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

### Control center and value sliders

The header avatar opens the [control center](control-center.md). From the navigation
bar use Up, then OK; desktop preview also supports F6. An optional color-button
binding provides another entry while Lumaglass is active. Back restores the parent
focus. Clock/avatar grouping and left/right placement are configured in
**Settings → General → Clock**.

Numeric and ordered preferences use labelled discrete sliders: Left/Right adjusts,
Up/Down changes rows, and the pointer can click or drag the track. OK cycles local
values. The native LG sleep timer instead keeps a draft until OK and confirmation.
