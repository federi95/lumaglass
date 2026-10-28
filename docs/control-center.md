# Control center and header

Lumaglass has an original native quick-controls panel inspired by the compact
right-hand layout and focus treatment of tvOS Control Center. The floating group
has a large power tile, network/audio capsules, screensaver/timer capsules and
four circular shortcuts. There is no large enclosing backplate. The parent page
is attenuated before backdrop capture, with a contrast scrim so its rows/text
cannot compete with quick controls. The controls themselves keep the clear,
lightly tinted glass used by the header. The panel uses the shared [Liquid Glass appearance](liquid-glass.md), the chosen wallpaper and explicit focus outlines. Off selects opaque surfaces; Light uses simpler reflections and Full retains lens refraction and RGB dispersion. Both optical modes use a bounded cached GPU backdrop. Backdrop updates are conditional; there is no per-frame blur engine.

## Opening and closing

Select the dedicated control-center button in the header. With a remote, press Up from the navigation bar,
then OK. In desktop preview, F6 opens or closes the panel. A color button can also
be assigned to **Control center** in **Settings → General → Remote shortcuts**;
shortcuts remain disabled by default. Back closes the panel and restores the
previous focus. Selecting the dedicated button again also closes it. The avatar opens a dedicated local profile; it never opens the control center or changes the current Settings section.

These entries work while Lumaglass is active. The LG Home key remains under LG's
control, including while another app is in front. The Apple TV remote's hold-TV
behavior is a design reference, not an LG key binding.

## Controls

| Control                               | Behavior                                                                                                                                                                                                                                                                                                               |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Standby                               | Requires a known active power state and confirmation, with Cancel selected initially. Sends one fixed LG standby request. An accepted request is not proof that the TV entered standby.                                                                                                                                |
| Network                               | Reads Wi-Fi/Ethernet connection status without storing SSID or IP addresses. Selecting it opens the existing LG settings app.                                                                                                                                                                                          |
| Audio                                 | Reads volume and mute state. OK toggles mute after rereading the expected state, with one write and readback. It does not switch audio outputs.                                                                                                                                                                        |
| Screensaver                           | Starts the local screensaver immediately, including when its automatic timeout is Off. The first wake gesture is consumed.                                                                                                                                                                                             |
| Sleep timer                           | Shows the native LG timer. Select the timer tile to open its detail slider, choose a draft value, then press OK and confirm. Back first returns to the main control center. Supported slots are Off, 10, 30, 60, 90 and 120 minutes. Off cancels the LG timer. This timer can turn the TV off after leaving Lumaglass. |
| Search, inputs, recent apps, settings | Opens the corresponding existing Lumaglass view.                                                                                                                                                                                                                                                                       |
| Wallpaper dimming                     | Adjusts the saved local wallpaper dimming immediately.                                                                                                                                                                                                                                                                 |

Opening the panel performs a bounded, read-only status refresh on the information
worker. System services remain off the rendering thread. Unavailable or refused
observations are shown explicitly and disable the associated mutation. Changes
are not automatically retried after timeouts. Desktop preview does not issue TV
commands; hardware-dependent fields report that they do not apply.

## Clock and avatar

**Settings → General → Clock** includes **Clock and avatar together**,
**Clock position** (Left/Right), and **Avatar style** (Initials/Symbol).
New/default preferences group clock and avatar on the right. Existing saved layout choices remain respected. Moving
the clock also moves the combined capsule when enabled. Without the capsule the avatar remains a separate profile button. The control-center button has its own icon, target and action.

Initials come from the local profile name, using at most two letters. Empty
names use the generic symbol. Twelve-/24-hour format and optional seconds work
in both layouts. The combined capsule and separate avatar button use clear
chrome, like the tab track and active tab pill.

## Local profile

Selecting the avatar opens **Local profile** over the current page. Edit or clear
the name and choose Initials/Symbol. Changes stay on this TV in Lumaglass
preferences; no LG, Apple or online account is created or accessed. The name is
also used by timed Home greetings. Editing it leaves a custom static greeting
unchanged. Finishing text entry returns to the profile; Back restores avatar
focus on the underlying page. Pointer and remote navigation share the controls.
The profile has no enclosing container: avatar, name and independent optical
pill rows float over the subdued parent, using the same clear glass as quick
controls. Typography, row padding, focus and opaque fallback stay shared with the
current interface. App-options menus retain their enclosing optical panel.

## Favorites dock

**Settings → General → Home personalization → Favorites dock** enables a centered,
lower Home tray with padded rounded glass, Off by default. Its width follows the
visible card count and density, up to one full row. Longer favorites lists scroll
inside the tray; favorite ordering, reordering and remote reveal remain available.
The dock always hides captions and uses opaque material if Liquid Glass is Off.

**Home personalization → Favorites** groups **Two-row favorites**, **Favorites
dock** and **Favorite names**. Greeting, date, inputs and startup options follow
in separate groups. General retains a separate **App names** entry for Home and
Apps labels. Favorite names can be changed from either route when the dock is Off.

Enabling two rows turns the dock Off and locks its switch. Turn two rows Off
before enabling the dock; clicking the disabled switch cannot override the choice.
The caption switches are locked while the dock is On. The saved caption preference
is preserved and becomes effective again when the dock is Off. Layout changes
reset horizontal scroll without changing favorite IDs/order. Two rows take
precedence when loading contradictory saved preferences, and turning two rows Off
does not silently restore the dock.

## Xbox Series X input

**Settings → Inputs → [discovered HDMI port] → Use Xbox Series X** assigns the local
name and original console-tower icon to that port. Clear name restores its reported
label. Recognition also works when the TV reports an Xbox name or it is typed as
a custom input name. The app ID, HDMI port and observed connection state remain
unchanged; selecting the preset does not send TV commands or create a virtual port.
Desktop example inventory includes an Xbox Series X alongside the existing inputs;
TV inventory continues to come exclusively from the TV services.

Selected/active chrome, hover/remote focus and presses have distinct tint,
rim and shadow treatments. App artwork lifts on focus inside stable hit areas;
reduced motion disables that lift. These states do not add an idle animation loop.

## Sliders

The Rust UI includes a reusable discrete slider with a labelled value, track,
ticks and focus ring. It is used for screensaver timeout, card density, wallpaper
dimming/blur, greeting size and the LG sleep-timer draft. Automatic screensaver
timeouts are Off, 1, 5, 10, 15, 30, 45 and 60 minutes.

Left/Right changes one step and stops at the endpoints; Up/Down moves to another
control. OK cycles local settings; on the sleep timer it opens confirmation.
Click or drag the track to snap to a step. Releasing a drag does not also trigger
OK or change another setting. Local settings are saved only when their value
changes. Changing a sleep-timer draft never writes to the TV.

## Design and protocol references

- [Apple TV Control Center guide](https://support.apple.com/guide/tv/adjust-general-settings-atvb18c04fc0/tvos): invocation and grouping of power, network, audio and timer controls.
- [Apple slider guidance](https://developer.apple.com/design/human-interface-guidelines/sliders): labelled ranges and clear feedback. Lumaglass implements its own Rust/SDL widget.
- The user-provided tvOS 26 screenshots inform the visual arrangement and combined clock/avatar capsule. No Apple screenshots, icons, fonts or avatars are shipped as new assets.
- [webOSbrew commands](https://www.webosbrew.org/pages/commands-cheatsheet): power-state observation.
- [Glasshouse controls](https://github.com/rorygallagher2024/glasshouse/blob/8e2521644f646ae6e6966f7ee06148a4daea0e41/server/lib/controls.js) and [power adapter](https://github.com/rorygallagher2024/glasshouse/blob/8e2521644f646ae6e6966f7ee06148a4daea0e41/server/lib/power.js): fixed LS2 protocol references. The adapter is original Lumaglass code; the Glasshouse MIT notice is retained in [NOTICE](../NOTICE.md).

Real standby behavior, audio routing and native sleep-timer availability still
need validation on each supported TV/firmware. Automated CI uses mocked services
and desktop rendering; it does not certify webOS 25 hardware behavior.
