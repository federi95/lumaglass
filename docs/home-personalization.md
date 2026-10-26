# Home personalization

Open **Settings → General → Home personalization**. Changes apply immediately
and are saved in local launcher preferences. Existing preference files retain
the original appearance and automatic startup focus.

| Setting                | Behavior                                                               |
| ---------------------- | ---------------------------------------------------------------------- |
| Show greeting          | Show or hide the greeting independently of the date and weather.       |
| Greeting mode          | Use the localized time-of-day greeting or a personal static message.   |
| Edit greeting          | Edit the name in time-of-day mode, or the message in custom-text mode. |
| Greeting size          | Cycle Small, Medium and Large. Long text still scales down to fit.     |
| Show date              | Show or hide the date.                                                 |
| Date format            | Cycle localized extended and compact numeric forms.                    |
| Show weekday           | Include or omit the weekday in either date format.                     |
| Show inputs on Home    | Show or hide the entire Home input section.                            |
| Greeting before date   | Swap the order of these two header elements.                           |
| Inputs above favorites | Swap the input and favorites sections.                                 |
| Initial focus          | Select a visible Home favorite or input, or keep Automatic.            |

Use OK to toggle or cycle a value. Arrow-marked rows open their editor or selector;
Back/Left returns to the parent. The existing General → Greeting entry remains
available as a shortcut to the editor for the selected greeting mode.

The timed greeting's name and static message are stored separately. Changing
mode preserves both. Static messages accept up to 80 Unicode characters with
control characters removed. Empty entry dismissal preserves the saved text;
**Clear greeting** explicitly clears it. An empty static message falls back to
the time-of-day greeting. The TV keyboard starts empty; typing saves replacement
text. The date, greeting and weather share a centered header;
weather follows the visible date/greeting elements. Favorites remain available
and the two-row setting works with either order of the main sections.

The input visibility switch is separate from individual input visibility modes.
Favorite order, input names and per-input modes remain available when sections
are hidden. Automatic focus follows the first visible card in the displayed
section order. Reordered sections keep remote navigation, pointer hit targets and
scrolling consistent with their displayed positions.

Initial focus is applied once when the initial inventory arrives while Home
navigation is still selected. Selecting it does not launch an app. Choices are
the visible favorites and inputs that have Home cards; extra scroll-preview
cards are excluded. Hidden, removed or otherwise unavailable choices fall back
to the automatic Home target. The saved choice is retained for a later launch
when it becomes available again. Inventory refreshes and returning to Home do
not repeatedly force the configured startup focus.

The nested `home` preferences store these controls. Missing fields use defaults;
invalid fields fall back independently. Malformed Home data does not discard
the unrelated favorites, weather or input preferences.
