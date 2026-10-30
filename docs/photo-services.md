# Photo services

**Settings → Background → My images** includes Nextcloud / WebDAV, Google Drive
and Google Photos alongside [Immich](immich.md). Imported photos use the existing
wallpaper, rotation and local-screensaver cache. A qualified system saver can
snapshot those same files. No service is contacted automatically at startup or by
screensaver presentation.

| Source             | Acquisition                                                    | TV credentials                                            |
| ------------------ | -------------------------------------------------------------- | --------------------------------------------------------- |
| Nextcloud / WebDAV | Explicit depth-one folder listing and image import             | Dedicated username and app password                       |
| Google Drive       | Explicit import from up to three configured folders            | Personal Desktop OAuth client and read-only refresh grant |
| Google Photos      | Explicit Picker export on a computer, followed by local import | Local export-folder reference only                        |

The Google Photos route is a user-requested file export, not a permanent API cache
or a full-library synchronization feature. Google's [best practices](https://developers.google.com/photos/overview/best-practices)
limit temporary performance caches, while the [Ambient API](https://developers.google.com/photos/partner-program/overview)
for ongoing TV/frame slideshows requires partner access. Ambient integration is
not included. Lumaglass does not provide a shared Google OAuth client or claim
Google verification; use your own configured project and account. Real service
access and TV behavior remain unverified by mocked CI.

## Private source files

Place each file beside the active `preferences.json`, with the same basename:

- `preferences.webdav.json`
- `preferences.google-drive.json`
- `preferences.google-photos.json`

A custom `--preferences /path/preview.json` uses `preview.webdav.json`, and so on.
The [Immich storage table](immich.md#connection-file) lists the existing desktop
and TV preference locations. Match the actual path and user reported by the app.
Files must be regular non-symlink files, owned by the app user, at most 4 KiB and
mode **0600**. Their immediate parent must be owned and not writable by others.
These files and Google's `client_secret_*.json` downloads are ignored by Git.
They are independent of exported preferences; service identifiers and connector
text are omitted from diagnostic state dumps.

## Nextcloud / WebDAV

Create a dedicated app password using your server's security settings. Configure
an HTTPS folder URL, including a reverse-proxy subpath if applicable:

```json
{
  "schema": 1,
  "server": "https://cloud.example.net/remote.php/dav/files/USERNAME/Photos/",
  "username": "REPLACE_LOCALLY",
  "app_password": "REPLACE_LOCALLY"
}
```

Choose **Load folders**, select the configured root and/or immediate subfolders,
then **Import selected photos**. At most three folders can be selected. The
selection is session-local; loading the same source preserves available choices.
Only direct children are scanned. Recursive traversal, server writes, public-share
login, cookie login and custom certificate bypasses are not implemented.

The client sends only PROPFIND and GET. An app password can have broader server
permissions than those operations; use a restricted account/share if your server
provides one. Credentials stay on the configured HTTPS origin and all returned
paths must remain inside the chosen folder. Redirects, external XML entities,
credential-bearing URLs and traversal paths are refused. Directory responses are
limited to 512 entries, with eight visible choices per native page.

Protocol reference: [Nextcloud WebDAV operations](https://docs.nextcloud.com/server/latest/developer_manual/client_apis/WebDAV/basic.html).

## Google Drive

Enable the Drive API in your own Google project and create a **Desktop app** OAuth
client. Keep its downloaded JSON private, outside the repository. Configure that
project's consent screen/account access before authorizing it. The requested
`drive.readonly` scope is restricted; deployment/access depends on Google's project,
account and verification rules, not on a successful native build.

Run the local setup helper on your computer, using folder IDs from Drive URLs:

```sh
chmod 600 /path/to/client_secret_download.json
sh tools/dev.sh node tools/photos/google-auth.mjs \
  --provider drive \
  --client /path/to/client_secret_download.json \
  --out .build/photo-auth/preferences.google-drive.json \
  --folder REPLACE_WITH_FOLDER_ID \
  --open
```

Repeat `--folder` up to three times. Without a folder ID, the configured scope is
limited by the client's requests to the My Drive root. Folder IDs do not restrict
the OAuth grant itself: `drive.readonly` permits reading the account's Drive files.
The native client uses only the configured roots and never widens a failed query.

The helper uses the system browser, a loopback callback, a random state and S256
PKCE. It stores only the refresh grant/client configuration in the private file;
the access token is not persisted. Transfer the resulting file beside the TV's
preferences with the required owner/mode. Do not transfer the Desktop client
file separately or put tokens in shell arguments.

Choose **Load folders**, select the desired configured folders, then **Import
selected photos**. The client uses fixed Drive v3 metadata and download endpoints,
reads direct child images in name order and skips trashed, non-downloadable,
unsupported or oversized files. Shared-drive support is bounded by the same
folder checks and the signed-in account's permissions. Shortcuts and recursive
folder import are not followed. Changing the connection requires loading it again.

Primary references: [Desktop OAuth and PKCE](https://developers.google.com/identity/protocols/oauth2/native-app),
[Drive scopes](https://developers.google.com/workspace/drive/api/guides/api-specific-auth)
and [file listing](https://developers.google.com/workspace/drive/api/reference/rest/v3/files/list).

## Google Photos export

Enable the **Photos Picker API** in your own Google project and create a Desktop
OAuth client. This route requests only `photospicker.mediaitems.readonly`, with
online authorization. Tokens stay in memory on the computer and are never saved
in a TV connection file.

```sh
chmod 600 /path/to/client_secret_download.json
sh tools/dev.sh node tools/photos/google-photos-export.mjs \
  --client /path/to/client_secret_download.json \
  --out .build/google-photos-export \
  --open
```

Choose a new output directory for each export; existing directories are preserved.
The helper first opens Google authorization, then a Google Photos selection page.
Choose up to twelve photos and finish selection in that page. The helper observes
Google's polling interval, downloads only selected images, converts supported
responses to metadata-stripped JPEGs up to 1920×1080, and attempts to close only
its own Picker session. It does not delete or edit photos on Google. A failed
export removes its staging directory and preserves earlier exports.

The result is a private directory containing `photo-01.jpg`, further numbered
photos and `google-photos-export.json`. It contains no OAuth token, remote photo
ID, account name or download URL. Copy this user-owned export directory to the TV,
preserving owner and private permissions: directory **0700**, files **0600**.
Configure its absolute TV path in `preferences.google-photos.json`:

```json
{
  "schema": 1,
  "export_folder": "/media/internal/.org.lumaglass.app/google-photos-export"
}
```

Choose **Load exported photos**, then **Import selected photos**. The native
client validates the export manifest and reads only its numbered files, without
contacting Google Photos. It does not discover future additions to the remote
library. Create and copy another explicit export when you want different photos.
Picker sessions and base URLs are temporary; they are not saved as wallpaper URLs.

Primary references: [Picker sessions](https://developers.google.com/photos/picker/guides/sessions),
[selected media and authorized downloads](https://developers.google.com/photos/picker/guides/media-items)
and [Photos API policy](https://developers.google.com/photos/support/api-policy).

## Limits, retention and removal

Imports use at most twelve supported photos, 8 MiB per source and 32 MiB aggregate.
Native requests time out after eight seconds with a sixty-second operation budget
checked between requests and processing. A request/decode already in progress can
finish before the budget is observed. Source and derivative bounds, the 64 MiB
cache quota and current-image retention are shared with local images and Immich.
No recurring download timer, retry loop or extra native networking thread is added.

A failed import preserves the previous catalog. Successful imports replace the
personal-image catalog while retaining its current selected old image. Source
files remain untouched. Temporary unreferenced derivatives can remain after a
failure and count toward the existing cache quota. Remote filenames may appear
as bounded local photo titles for WebDAV/Drive; Photos exports use neutral titles.

**Forget connection** opens a confirmation with Cancel first. Confirmation removes
only that source's local connection file; exported originals and cached images
remain. It does not revoke a Google grant or a WebDAV app password. Revoke those
at the provider separately when needed. User export directories can be removed
independently after import; existing offline derivatives remain usable.

CI uses synthetic HTTP/OAuth responses, temporary files and native desktop
rendering. `--cloud-preview` is desktop-only, requires isolated `--preferences`,
and shows synthetic folder pages with connection/import/removal disabled.
No personal service, account, TV installation or firmware capability is qualified
by these checks.
