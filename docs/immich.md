# Immich albums

**Settings → Background → My images → Immich albums** adds an optional native
album selector. The connector imports photos into the existing offline image
cache for wallpapers, rotation and the local saver. A qualified future system-saver
application can snapshot those same imported images. Loading albums or importing
requires an explicit action; startup and screensaver presentation never fetch from
Immich. No web panel, WebView, MQTT bridge or background synchronization is added.

## Connection file

Create a dedicated API key in Immich on a computer or phone. Grant only
`album.read`, `asset.read` and `asset.view`. The connector does not need `all`,
upload, download-original, delete, administrative or key-management permissions.
These permissions can expose more library metadata to the key than the selected
albums; the selection limits Lumaglass’s requests, not the key’s server-side scope.

Store the following JSON in **`preferences.immich.json` beside the active
`preferences.json`**, replacing the server and key locally:

```json
{
  "schema": 1,
  "server": "https://photos.example.net",
  "api_key": "REPLACE_LOCALLY",
  "albums": []
}
```

The file must be a regular non-symlink file, at most 4 KiB, owned by the same user
as the Lumaglass process, with mode **0600**. Its immediate parent must be a real
app-owned directory without group/other write permission. Use a trusted local
editor and set file permissions before loading it; do not put a real key in shell
arguments, a screenshot, chat or a repository. `*.immich.json` is ignored by Git.
A credential file is deliberately not accepted through the preference JSON or a
text box that could be included in rendering diagnostics.

Examples of corresponding locations:

| Active preferences                                    | Private connection                                           |
| ----------------------------------------------------- | ------------------------------------------------------------ |
| `~/.config/lumaglass-native/preferences.json`         | `~/.config/lumaglass-native/preferences.immich.json`         |
| `/media/internal/.org.lumaglass.app/preferences.json` | `/media/internal/.org.lumaglass.app/preferences.immich.json` |
| `/media/developer/org.lumaglass.app/preferences.json` | `/media/developer/org.lumaglass.app/preferences.immich.json` |
| A custom `--preferences /path/preview.json`           | `/path/preview.immich.json`                                  |

The TV chooses among its existing persistent storage candidates; use the actual
path reported at startup and match that app user. This file is not created,
deployed to a TV or populated with personal data by development tooling.

The server must be an **HTTPS origin**, optionally ending in `/api`. A trusted TLS
certificate is required. Plain HTTP, credentials in the URL, query strings,
fragments and reverse-proxy subpaths are refused. The API key is sent only in the
`x-api-key` header to the configured API origin. Redirects and HTTP proxies are
disabled; there is no certificate-verification bypass. Cookie/basic-auth/shared-link
flows and OAuth are not implemented.

## Native selection and import

1. Open Immich albums and choose **Load / refresh albums**. The file is read only
   when a connection operation is explicitly requested. An unreadable file, denied
   key, unsupported API or invalid response is shown as unavailable; no dummy
   albums replace a failed real connection.
2. Select up to **three** albums. Eight entries are shown per page, with Previous
   and Next controls. The draft selection stays in memory until import.
3. Choose **Import selected albums**. This saves the selection in the private
   connection file and imports up to **twelve** eligible photo previews across
   the selected albums, in selection order. It takes the first bounded result page
   from each needed album; this is not a full-library or full-album sync. The file
   is checked against the connection that produced the album list; after changing
   server/key, reload albums first.
4. Return to My images to select the imported photo, or include it in wallpaper
   rotation. The saved current image is retained when the imported catalog changes.
   Manual wallpaper changes and rotation are held during the import to keep that
   selection consistent. Fit inside / Fill and crop uses the existing image setting.

Only online, non-trashed, non-archived images with timeline visibility are eligible.
Video, hidden/locked, unknown or malformed metadata is skipped or refused. Originals
are not downloaded. The connector does not create, edit, delete, share or upload
anything in Immich. The metadata search uses POST as a read operation, always with
an explicit album filter; it never retries without that filter or widens to the
whole library.

Source previews have an 8 MiB limit each and a 32 MiB aggregate download limit.
JSON responses are capped at 2 MiB, album lists at 512 entries, and returned search
pages at twelve items. Each request has an eight-second timeout; the operation
budget is sixty seconds, checked between requests and image processing. A request
or decode already in progress may finish before that budget is observed. Acquiring,
decoding and generating derivatives stays on the image worker. The existing image
pixel/decode limits and 64 MiB cache quota still apply. There are no automatic
retries or periodic network timers.

A rejected or failed import retains the previous catalog and wallpaper. New
unreferenced derivative files can remain after a failure and count toward the
existing cache quota; originals on the server are untouched. Successful imports
replace the visible personal-image catalog, while retaining its selected old image;
local source settings and cached files are not erased. Cached photo titles use
neutral `Immich N` labels, without remote filenames, album IDs or server URLs.

## Remove connection

**Remove connection** selects Cancel first. Confirmation removes only the private
connection file; cached images remain usable offline. It does not revoke the key on
Immich or delete photos there. Revoke that dedicated key separately in Immich if
needed. Connection commands are unavailable during an import. If the server/key
file has changed, reload before removing a previously loaded identity.

No keys, server URL, album IDs or album names are added to exported preferences or
native state dumps. The selector naturally displays album names, so treat manual
screenshots of a real library as personal data. CI uses synthetic names, fake keys,
temporary files and loopback fixtures only.

## API compatibility and verification

The implemented request shapes were checked against the official specifications
for **Immich 2.5.6** and **3.3.5**. Version detection accepts 2.5+ within major 2 and
major 3: older branches use the documented legacy search filters, while 3.2+ uses
structured filters. Unknown majors and pre-2.5 APIs are refused. This range is a
protocol compatibility boundary, not a claim of live testing against every release.
API changes or incompatible reverse-proxy responses can still make a particular
server unavailable. No personal server or TV has been used to validate this connector.

CI covers endpoint filtering, credentials, redirects, metadata exclusion, limits,
offline derivatives and failure retention, plus native settings and album navigation.
An explicit desktop-only `--immich-preview` shows synthetic album pages while
blocking all connection/import/removal commands; TV builds reject the flag. It
can be used for layout review without a server and does not persist selections.

Primary API references: [Immich 3.3.5 specification](https://github.com/immich-app/immich/blob/v3.3.5/open-api/immich-openapi-specs.json),
[Immich 2.5.6 specification](https://github.com/immich-app/immich/blob/v2.5.6/open-api/immich-openapi-specs.json),
and the [official API documentation](https://api.immich.app/). This original client
bundles no Immich server, frontend, generated SDK or assets; see [NOTICE](../NOTICE.md).
Google Photos/Drive and iCloud remain separate future connectors.
