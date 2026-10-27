# Render resolution and recovery

Settings → System → Render resolution shows the requested window size, actual
SDL drawable and SDL display mode separately. LG compositor geometry remains
unverified. A 3840×2160 drawable is not proof of native 4K presentation on the panel.
System → Text rendering retains the independent MSAA/Analytic selection.

The TV default is 1080p. Try experimental 4K restarts Lumaglass with a borderless
3840×2160 window request. The restarted interface opens the resolution page with
Use 1080p / cancel trial selected first. Keep this resolution must be saved within
fifteen seconds of startup. A watchdog in a separate process claims an expired
trial atomically, writes a safe-mode marker and restarts the launcher at 1080p,
including after startup failure or a blocked renderer. Confirmation and expiry
compete for one marker; a late confirmation cannot claim an expired trial.

The confirmed preference is saved by the normal preference worker before the
watchdog is disarmed. Future confirmed 4K launches still have a startup watchdog,
disarmed after a presented frame. This cannot detect every compositor/panel
failure; manual TV verification remains necessary. CLI `--resolution` overrides
are explicit development controls outside this persisted Settings trial.

Display sidecars live beside preferences: `.display-request.json` tracks a
pending trial/startup, and `.display-1080p` forces safe mode. A stale in-flight
marker selects safe mode before SDL initializes. The helper can signal only its
own actual parent process; it accepts no target PID. It does not edit LG drivers,
framebuffer or compositor configuration, mount system files, or invoke a shell.
An unconfirmed normal exit cancels the trial and leaves safe mode for next launch.

For independent recovery, close Lumaglass and create the safe-mode sidecar beside
your chosen preferences file (for `preferences.json`, use `preferences.display-1080p`).
The next launch uses 1080p. Existing Home recovery in tv-integration.md remains
available if the launcher itself cannot be opened. 4K quality, memory use and
frame pacing on QNED756RA/webOS 25 are unverified until hardware checks.
