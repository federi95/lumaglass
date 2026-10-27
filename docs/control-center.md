# Control center and header

Lumaglass has an original native quick-controls panel inspired by the compact
right-hand layout and focus treatment of tvOS Control Center. The panel uses
rounded translucent surfaces, a precomputed soft wallpaper and explicit focus
outlines. It does not perform a full-screen blur or refraction on every frame.

## Opening and closing

Select the avatar in the header. With a remote, press Up from the navigation bar,
then OK. In desktop preview, F6 opens or closes the panel. A color button can also
be assigned to **Control center** in **Settings → General → Remote shortcuts**;
shortcuts remain disabled by default. Back closes the panel and restores the
previous focus. Selecting the header avatar again also closes it.

These entries work while Lumaglass is active. The LG Home key remains under LG's
control, including while another app is in front. The Apple TV remote's hold-TV
behavior is a design reference, not an LG key binding.

## Controls

| Control                               | Behavior                                                                                                                                                                                                                               |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Standby                               | Requires a known active power state and confirmation, with Cancel selected initially. Sends one fixed LG standby request. An accepted request is not proof that the TV entered standby.                                                |
| Network                               | Reads Wi-Fi/Ethernet connection status without storing SSID or IP addresses. Selecting it opens the existing LG settings app.                                                                                                          |
| Audio                                 | Reads volume and mute state. OK toggles mute after rereading the expected state, with one write and readback. It does not switch audio outputs.                                                                                        |
| Screensaver                           | Starts the local screensaver immediately, including when its automatic timeout is Off. The first wake gesture is consumed.                                                                                                             |
| Sleep timer                           | Shows the native LG timer. Select a draft value with the slider, then press OK and confirm. Supported slots are Off, 10, 30, 60, 90 and 120 minutes. Off cancels the LG timer. This timer can turn the TV off after leaving Lumaglass. |
| Search, inputs, recent apps, settings | Opens the corresponding existing Lumaglass view.                                                                                                                                                                                       |
| Wallpaper dimming                     | Adjusts the saved local wallpaper dimming immediately.                                                                                                                                                                                 |

Opening the panel performs a bounded, read-only status refresh on the information
worker. System services remain off the rendering thread. Unavailable or refused
observations are shown explicitly and disable the associated mutation. Changes
are not automatically retried after timeouts. Desktop preview does not issue TV
commands; hardware-dependent fields report that they do not apply.

## Clock and avatar

**Settings → General → Clock** includes **Clock and avatar together**,
**Clock position** (Left/Right), and **Avatar style** (Initials/Symbol).
Existing installations keep the separate clock on the left by default. Moving
the clock also moves the combined capsule when enabled. Without the capsule the
avatar remains a separate header button.

Initials come from the local greeting name, using at most two letters. Empty
names use the generic symbol. These controls do not create accounts or alter
the saved name. Twelve-/24-hour format and optional seconds work in both layouts.

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
