# TV installation and Home integration

> These procedures derive from Luma. Lumaglass has a separate app ID and has not yet been verified on TV hardware. Disable any Luma/Glasshouse Home takeover using its own restore command before enabling Lumaglass. Existing preferences and boot hooks are not migrated.

## Read-only TV information

Settings → System → TV information reads model, webOS SDK version, firmware,
elevated helper access, Home mount observations and installation location. It
does not enable takeover or change settings. Refresh is explicit; missing or
denied values remain visible. Use it from the normally installed app to observe
the Homebrew Channel caller path. Root SSH access alone does not establish the
app's permissions. Lumaglass hardware verification is still pending.

## Installation

### Homebrew Channel

The URL below is the fork’s publication target. No Lumaglass release has been published by this work; use a locally built package until one exists.

```text
https://github.com/federi95/lumaglass/releases/latest/download/repo.json
```

### SSH

On your computer, replace `TV_IP`:

```sh
scp -O dist/org.lumaglass.app_2.9.0_arm.ipk root@TV_IP:/tmp/lumaglass.ipk
ssh root@TV_IP
```

In the TV's root shell:

```sh
luna-send -i luna://com.webos.appInstallService/dev/install \
  '{"id":"org.lumaglass.app","ipkUrl":"/tmp/lumaglass.ipk","subscribe":true}'
```

## Replacing LG Home

In the TV's root shell, resolve the installed package directory first. Homebrew
Channel can install under cryptofs; do not assume the developer path:

```sh
APP=
for CANDIDATE in \
  /media/cryptofs/apps/usr/palm/applications/org.lumaglass.app \
  /media/developer/apps/usr/palm/applications/org.lumaglass.app \
  /usr/palm/applications/org.lumaglass.app
do
  if [ -f "$CANDIDATE/appinfo.json" ] && [ -x "$CANDIDATE/lumaglass" ]; then
    APP=$CANDIDATE
    break
  fi
done
[ -n "$APP" ] || { printf '%s\n' 'Lumaglass installation not found.' >&2; exit 1; }
printf '%s\n' "$APP"
```

The commands below use this `APP` in the same shell. If multiple copies are
installed, select the intended directory explicitly before enabling a hook.

### Takeover Method

> [!NOTE]
> Upstream Luma documents takeover on **webOS 5, 6, 25 and 26**; Lumaglass still requires hardware verification.\
> The takeover does not delete LG Home: it remains underneath a temporary mount, which is cleared by
> a reboot. If Lumaglass is uninstalled first, LG Home returns on reboot, but the separate takeover boot
> hook remains and logs an error because it can no longer find Lumaglass. No TV reset is needed. For
> clean removal, disable takeover before uninstalling.

Install Lumaglass and verify that it opens normally. Disable any Home-button mapping in
[LG Input Hook](https://github.com/Simon34545/lginputhook) (remove its Home-key action) and any
other Home replacement, then run in the TV's root SSH shell:

If you enabled Lumaglass's autostart method earlier, disable it first:

```sh
sh "$APP/autostart.sh" disable
```

To enable the takeover run:

```sh
sh "$APP/home.sh" enable
```

To see the status or disable the home override use:

```sh
sh /var/lib/lumaglass-home/home.sh status
sh /var/lib/lumaglass-home/home.sh disable
```

`restore` is an alias for `disable`.

Boot log: `/var/lib/lumaglass-home/boot.log`. See the
[development guide](development.md#optional-native-home-integration) for implementation details.

<details>
<summary>Method for webOS 5 / 6</summary>

On **webOS 5 and 6**, SAM cannot reread a mounted Home manifest, so `home.sh enable` instead
bind-mounts Lumaglass's executable over LG's `/usr/bin/com.webos.app.home`. LG's Home manifest stays
in place and Lumaglass registers with SAM itself, so the Home key brings the running Lumaglass back instead
of restarting it. The commands and the cold-boot hook are the same.

</details>

<details>
<summary>Method for webOS 25 / 26</summary>

Mounts Lumaglass over the system Home app and reapplies the mount at cold boot. The LG splash
shown before Lumaglass's window opens is the wallpaper you selected in **Settings → Background**, staged
from the saved preferences at each boot. Normal app launches also use your selected wallpaper:
Lumaglass updates the launch splash through Homebrew Channel's root service after loading or saving your
settings. Open Lumaglass once after installing or updating so subsequent launches use your saved background.
LG Home's original files remain underneath, using the same approach as
[Glasshouse Home](https://github.com/rorygallagher2024/glasshouse/blob/main/server/lib/home-takeover.js).

</details>

### Autostart Method

Open Lumaglass 25 seconds after a full cold boot. After resolving `APP` above,
run in the TV's root SSH shell:

```sh
sh "$APP/autostart.sh" enable
```

Use this when takeover is disabled. It launches Lumaglass as a regular app and does not remap the Home button.

Disable before enabling takeover or uninstalling Lumaglass:

```sh
sh "$APP/autostart.sh" disable
```

### LG Input Hook Method

You can use [LG Input Hook](https://repo.webosbrew.org/apps/org.webosbrew.inputhook/), configure an Execute action with this one-line command:

```sh
/usr/bin/luna-send -n 1 -w 5000 luna://com.webos.service.applicationmanager/launch '{"id":"org.lumaglass.app"}'
```

Test on a spare button first. Disable the mapping in Input Hook to restore normal Home behavior.

## TV startup diagnostics

SAM passes launch metadata as a JSON object in `argv[1]`. The CLI skips that object while leaving
the original process arguments available to SDL/libhelpers. To expose the actual developer-mode
sandbox's stdout/stderr, close Lumaglass and run in the TV's root SSH shell:

```sh
APP=/media/developer/apps/usr/palm/applications/org.lumaglass.app
XDG_RUNTIME_DIR=/tmp/xdg RUST_BACKTRACE=1 /usr/bin/jailer \
  -t native_devmode -i org.lumaglass.app -p "$APP" \
  "$APP/lumaglass" '{"@system_native_app":true,"nid":"org.lumaglass.app"}'
```

Startup prints accepted launch parameters, SDL driver, actual sample count, cumulative stage
timings, first-frame latency and vector-text cache size. Timings begin inside the executable and
exclude SAM/jailer process-launch overhead. The IPK is unpacked at install, not every launch.
A previous vector-font startup on the development TV presented its first frame in 399 ms;
new rendering paths still need their own hardware comparisons. SDL helper `getAppInfo` or
`getSystemSettings` registration calls can report permission warnings; they have not blocked
the verified interface/inventory runs. Normal launch/relaunch behavior should be checked too.

The native adapter registers LS2 connections from the executable and calls `LSCallOneReply`.
Anonymous names avoid collisions with SDL's registration; the hub still applies the executable's
role. `luna-send-pub -a` app impersonation is rejected on the development webOS 26 TV and is not
used; the only `-a` use is the root system-bus `luna-send` that holds a staged Home's lifecycle
registration (see [Home lifecycle on webOS 10/11](#home-lifecycle-on-webos-1011)). Native requests run on one worker's private GLib context with a 6.5-second budget and 2 MiB
reply limit. Errors include URI and the bus refusal. Fixed inventory commands run via elevated
Homebrew Channel `exec`, parsing complete multi-line `luna-send -n 1` JSON replies. Input-read
failures preserve the last valid inputs. No failed inventory read substitutes demo apps on TV.

### Optional native Home integration

`tv/home.sh` and `tv/home.cjs` are packaged beside `lumaglass`. They are a root-only, explicit integration
for webOS 10/25 and 11/26 (and webOS 5/6, see below). `enable` stages a native manifest with ID `com.webos.app.home` and
symlinks to the installed executable, libraries and assets under `/var/lib/lumaglass-home/app`, then
bind-mounts it over `/usr/palm/applications/com.webos.app.home`. SAM rereads that one app through
`blockSystemApp` followed by `releaseBlockSystemApp`, including `forceBlockSystemApp: false` when
the TV schema requires it. The script verifies the reported manifest before stopping the previous
Home process. It attempts to restore LG Home if applying the mount or refreshing SAM fails.

Before staging, `enable` closes the normal Lumaglass instance with `closeByAppId` and waits up to five
seconds for it to leave the running list. The TV's `close` endpoint identifies a process, so passing
the package ID there can return `no app matched by pid`. If closing reports an error, the script
rechecks the running list and proceeds only if the normal instance has already exited.

Root-shell calls use `luna-send -n 1` on both firmware versions, with bounded stdout/stderr
capture and operation-specific diagnostics for empty or invalid replies. `-t 1` is timing mode:
it logs the payload to stderr and writes timing summaries to stdout, not a JSON response.

The `enable` shell entry point prints the change summary and requires a `y`/`n` confirmation
before creating the state directory, acquiring the lock or invoking the installer. Only `y`/`Y`
proceeds. `n`/`N` or end-of-input cancels, and other input repeats the prompt. The boot hook and
`apply` remain non-interactive, and `disable`/`restore` remain available without confirmation.

The native CLI accepts SAM's Home `nid` and gives SDL the matching Home surface identity. The
executable itself stays at its installed path for LS2 authentication, `$ORIGIN/lib`, asset discovery
and preference selection. Opening Lumaglass's normal tile while the Home mount is active redirects to
the Home instance. The scripts do not create a browser wrapper or change the package's app ID.

`64-lumaglass-home` reapplies the mount from a detached boot hook. The existing TV Home Auto Launch
and Quick Start+ settings determine whether power-on opens Home. Until the hook applies, LG Home
can still appear. This integration does not change those settings or install a remote-key filter.
`disable` (also `restore`) removes the mount, refreshes LG's manifest, stops the native Home
instance and removes the boot hook. The copied restore scripts remain in `/var/lib/lumaglass-home`
even if the installed package is removed. Disable the standalone `65-lumaglass` cold-boot hook before
enabling this integration.

Build and script checks cannot establish native Home compatibility. Manually verify Home, Back
out of the last app, standby wake with the TV's chosen Quick Start+ setting, a full cold boot,
saved preferences and restoring LG Home. The broader webOS 6+ app compatibility statement does
not establish Home-replacement support on older firmware.

### Running in place of LG's Home executable (webOS 5)

On webOS 5 the manifest takeover above does not apply: SAM restarts a v1 native Home on every
Home launch. A root integration can instead bind-mount the installed `lumaglass` executable over LG's
`/usr/bin/com.webos.app.home`. SAM then starts it with LG's own Home manifest and LS2 role, which
is bound to that executable path. Lumaglass detects this (`platform::runs_as_lg_home`) and:

- disables SDL's own app registration, which cannot reach the bus under LG's Home role, and
  registers with SAM itself through `registerApp` (lifecycle v2) on its own LS2 connection.
  `luna::subscribe` waits up to 10 s for SAM's first reply (`registered`); a failed connection,
  call or refused registration is a startup error, so Lumaglass exits and SAM starts Home afresh.
  `relaunch` raises the window and refreshes the inventory, `close` and a later
  `returnValue: false` (registration lost) quit. SAM keeps the process across Home launches and
  standby (`pause`, reason `keepAlive`) instead of restarting it;
- resolves `assets/`, `lib/` (extra RPATH) and the splash directory from the installed package,
  since `current_exe()` names LG's path. Production Homebrew Channel installs under
  `/media/cryptofs`, developer installs under `/media/developer` and system installs under
  `/usr/palm/applications` share the lookup list in `tv/install-directories.txt`. Library search
  retains `$ORIGIN/lib` first and includes all three locations before Rust startup. Native
  discovery validates the package manifest and matches the executable by device/inode, so assets
  and splash updates use the source package even when developer and production copies both exist.
  A missing or unrelated source package is an explicit startup error;
- writes its stderr to `/tmp/lumaglass-home.log`, because SAM discards a system app's output;
- skips the per-icon elevated-service fallback for installed artwork: as root every file is
  readable, so that fallback would only spend seconds per icon to fail the same way.

`tv/home.sh` chooses this method (`"method": "executable"` in `state.json`) on webOS 5/6 when LG
Home's manifest is native, starts `file:///usr/bin/com.webos.app.home` and declares
`nativeLifeCycleInterfaceVersion: 2`. `enable` and the boot hook bind-mount the installed
executable over that path, stop the previous Home process and open Home if it was in front (or
on `enable`). Lumaglass in front within about 15 s is the success condition, with one repeated launch
after 6 s, because SAM can accept the first launch right after the old process stopped without
starting anything; otherwise the mount is removed again. A mount whose source is Lumaglass's executable
counts as its own also after a package update (`//deleted`) and is replaced; any other mount over
LG's executable is refused. `disable` unmounts (lazily while Home runs), stops Lumaglass and reopens LG
Home if Home was in front. While mounted, `/var/run/lumaglass-home` names Lumaglass, so a normal
tile launch redirects to the Home instance: the tile's jail sees `/usr/bin` without the mount but
shares `/var/run`, which is volatile like the mount itself. LG's file stays underneath, and a cold boot
without the hook gives LG's Home back. Verified on an LG 49UN7100
(webOS 5): Home key to Lumaglass in about 0.4 s instead of a restart of about 2 s.

### Home lifecycle on webOS 10/11

With the staged manifest (see [Optional native Home integration](#optional-native-home-integration)), SAM starts Lumaglass's own executable for `com.webos.app.home`, but
never as a registered app: on every Home key it kills Lumaglass and starts it again. Running in place
of LG's executable is not an option here, because LG Home's role
(`/usr/share/luna-service2/roles.d/com.webos.app.home.app.json`) is app-ID based
(`"appId": "com.webos.app.home"`, names `com.webos.app.home-*`), not bound to a path, and Lumaglass's
role only allows `org.lumaglass.app` or an anonymous name.

The staged Home runs as root, and as root the system-bus `luna-send -a` may act for an app ID.
So when `options.webos_home` is set outside LG's executable path and the effective user is root,
Lumaglass calls `luna::subscribe_as_app`, which runs `app_subscription::subscribe` with a child
`luna-send -a com.webos.app.home -i` that holds
`registerNativeApp`. SAM expects lifecycle v1 for this manifest and refuses `registerApp` with
"unmatched method with nativeLifeCycleInterfaceVersion". v1 names its events in `message` rather
than `event`; `platform::Lifecycle::from_reply` reads both and feeds the same `relaunch`/`close`
handling as v2.

- Startup does not wait for SAM. Until it answers, Home is simply unregistered.
- No answer within 10 s, a refused answer or the child ending before an answer kills and reaps
  the child and leaves Home unregistered, as before this integration: SAM restarts it per Home
  key. Once SAM has accepted the registration, losing it (a later `returnValue: false` or the
  child ending) quits Lumaglass, as on the v2 path, so SAM starts Home afresh.
- The child gets `SIGKILL` when the thread that started it (Lumaglass's main thread) ends
  (`PR_SET_PDEATHSIG`, re-checked against the parent PID after arming). A registration left with
  nobody handling its events leaves the Home key dead and a black screen after leaving an app.
- On `relaunch` Lumaglass restores, shows and raises its window (on both lifecycle paths). If a
  relaunch arrives while Lumaglass is not both focused and visible, `platform::ForegroundCheck` waits
  3 s for `FocusGained` or a remote key, which only reach the focused surface. `Shown` and
  `Restored` do not count, because `SDL_ShowWindow` emits `Shown` itself whether or not the raise
  reached the screen. Without confirmation, Lumaglass assumes it cannot bring itself forward on this
  firmware: it creates `/var/lib/lumaglass-home/no-lifecycle` and ends the registration, which quits
  it, so SAM starts Home afresh. While that file exists, Lumaglass does not register and Home behaves
  as before. Delete it to try again.
- A setup error after the child started (e.g. a thread cannot be created) kills and reaps the
  child before returning the error, so no registration is left behind while Lumaglass keeps running.
- stderr goes to `/tmp/lumaglass-home.log` in this mode too. Replies are logged as
  `Lumaglass lifecycle: …`, the helper's end as `Lumaglass app subscription ended (was registered: …)`,
  and window visibility and focus changes as `Lumaglass window: …`.
- `app_subscription` has no LS2 dependency, so its host tests run it against fake `luna-send`
  scripts: replies and noise, refusal, silence, an early exit, a registration lost later, and
  failed thread starts. `ForegroundCheck` has its own tests, including relaunch → `Shown` without
  focus.
- SDL's own app registration (`SDL_WEBOS_REGISTER_APP`) is left as it was for the staged
  manifest. It does not register Home there (SAM restarted Lumaglass per Home key with it enabled),
  and disabling it would change startup on a path without a reason seen on a TV.

Upstream Luma observation, verified on an LG OLED65C36LC (webOS 26, 11.2.0-3701): without registration the Lumaglass PID changes
on every Home key. With it, the PID stays the same across Home presses and Back out of apps, SAM
sends `relaunch` with `launchType: homeKey`, Lumaglass comes to the front, and it registers again after
a cold boot. SDL's own registration logs `Failed to register app: Failed to get
nativeLifeCycleInterfaceVersion` at startup there, without effect. That the compositor sends
`FocusGained` after such a relaunch, which the watchdog relies on, shows in the `Lumaglass window:` log.
