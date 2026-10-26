# Appearance

Open **Settings → General → Appearance**. Press OK on a row to cycle its value;
changes apply immediately and are saved locally without restarting the launcher.

## Card density

| Density            | Cards across the Apps grid and fully visible Home strip |
| ------------------ | ------------------------------------------------------- |
| Compact            | 8                                                       |
| Standard (default) | 6                                                       |
| Comfortable        | 5                                                       |

Card proportions and caption text size are preserved. Captions reserve their
measured height. Drawing, hit targets, wheel scrolling and remote navigation use
the same geometry; Apps Up/Down follows the selected column count. Home's optional
two-row layout keeps its row-major favorite order and shares horizontal scrolling.
Changing density resets Home's horizontal scroll to the start. Input cards and
the background gallery keep their own layouts.

Existing preference files use Standard density. Unknown values fall back to
Standard without discarding unrelated preferences.

## Accent color

Cycle Neutral, Blue, Teal, Violet, Amber and Rose. Neutral is the original default.
The palette uses light selection surfaces with dark labels. Accent colors apply
to navigation and settings selections, focus outlines and selected indicators;
the app artwork keeps its original colors. This controls color only: outline
thickness, animation speed and other focus styling are not additional settings.
