# Save to Markdown

A small Firefox extension that saves tabs, pages, and links to a Markdown vault.
Right-click → **Save for later**. Each click appends one line to `Index.md`:

```markdown
- [Page title](https://example.com)
```

Includes a bookmark menu icon and a Rust native messaging helper. No server,
telemetry, dates, tags, or accounts. Firefox launches the helper when you save;
it exits when the connection closes.

## Setup (macOS)

Requires desktop Firefox 140+, [Rust/Cargo](https://rustup.rs), and Python 3.
The vault folder must already exist.

```sh
git clone https://github.com/schester44/save-to-markdown.git
cd save-to-markdown
bash scripts/install-macos.sh "/Users/yourname/Documents/My Vault/Index.md"
```

In Firefox, open `about:debugging#/runtime/this-firefox`, click **Load Temporary
Add-on…**, and select `extension/manifest.json`. Right-click a tab, link, or page
and choose **Save for later**. Errors appear as notifications.

Temporary installation lasts until Firefox restarts. Permanent installation in
regular Firefox requires a Mozilla-signed XPI, installed through `about:addons`
→ gear menu → **Install Add-on From File…**. This repository contains unsigned
source for version 1.0.2; the earlier 1.0.0 signing approval does not cover it.
See [Mozilla signing instructions](https://extensionworkshop.com/documentation/publish/signing-and-distribution-overview/).

## Configuration

The installer writes one local config file:
`~/Library/Application Support/SaveToMarkdown/config.json`.

```json
{
  "index_path": "/Users/yourname/Documents/My Vault/Index.md"
}
```

Use an absolute path ending in `Index.md`. Edit this file to change your vault;
no restart is required. The helper creates the file if missing, but not its parent
folder. Browser messages cannot specify paths; unknown fields are rejected.

The executable lives in `~/Library/Application Support/SaveToMarkdown/`.
Firefox's host manifest lives in
`~/Library/Application Support/Mozilla/NativeMessagingHosts/local.save_to_markdown.json`.
The installer fills in the included manifest template; its extension ID must
match the Firefox manifest.

## Behavior

- Supports HTTP and HTTPS URLs. Link saves use link text; page and tab saves use
  the tab title. Missing titles fall back to the URL.
- Preserves existing contents and adds a newline separator when needed. Duplicate
  clicks append duplicate entries.
- Escapes Markdown punctuation in titles and encodes URL destination delimiters.
- Locks across helper instances and syncs before reporting success. External
  editors and sync tools may not honor these advisory locks.
- Uses Firefox's native-endian, four-byte length-prefixed JSON protocol, capped
  at 64 KiB per request. Standard output is reserved for protocol messages.
- The local config and its symlinks are trusted. A disk error can leave a partial
  or already-written entry; inspect the file before retrying.

## Tests

```sh
cargo test --locked --manifest-path native-host/Cargo.toml
cargo build --locked --manifest-path native-host/Cargo.toml
python3 scripts/test-host.py native-host/target/debug/save-to-markdown-host
node scripts/test-extension.cjs
```

The host checks use a temporary vault and config. They cover framing, appending,
Markdown escaping, path rejection, invalid URLs, concurrency, and config errors.
The JavaScript test mocks Firefox to check page/link/tab capture and errors.
Node is only needed for that test. All these checks passed before publication;
Firefox UI testing is separate.

If saving fails, inspect the extension from `about:debugging`. Check the config,
vault permissions, and native manifest path. To uninstall, remove the extension
and the installed helper, config, and host manifest. Your vault is preserved.

## License

[MIT](LICENSE).
