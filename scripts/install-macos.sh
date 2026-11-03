#!/bin/bash
set -euo pipefail
[[ "$(uname -s)" == Darwin ]] || { echo 'This installer requires macOS.' >&2; exit 1; }
[[ $# == 1 ]] || { echo "Usage: $0 /absolute/path/to/vault/Index.md" >&2; exit 1; }
command -v cargo >/dev/null || { echo 'Install Rust (https://rustup.rs) first.' >&2; exit 1; }
command -v python3 >/dev/null || { echo 'Python 3 is required for installation.' >&2; exit 1; }
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# Validate before building or changing installed files.
python3 - "$1" <<'CHECK'
import pathlib, sys
p = pathlib.Path(sys.argv[1])
if not p.is_absolute() or p.name != 'Index.md' or not p.parent.is_dir() or (p.exists() and not p.is_file()):
    sys.exit('Provide an absolute Index.md path inside an existing vault directory.')
CHECK
cargo test --manifest-path "$ROOT/native-host/Cargo.toml"
cargo build --release --manifest-path "$ROOT/native-host/Cargo.toml"
APP="$HOME/Library/Application Support/SaveToMarkdown"
HOSTS="$HOME/Library/Application Support/Mozilla/NativeMessagingHosts"
mkdir -p "$APP" "$HOSTS"
# Rename into place so an existing running host is not overwritten in place.
install -m 755 "$ROOT/native-host/target/release/save-to-markdown-host" "$APP/save-to-markdown-host.new"
mv -f "$APP/save-to-markdown-host.new" "$APP/save-to-markdown-host"
python3 - "$ROOT" "$APP" "$HOSTS" "$1" <<'INSTALL'
import json, os, pathlib, sys
root, app, hosts = map(pathlib.Path, sys.argv[1:4])
def write_json(path, data):
    temp = path.with_suffix('.json.tmp')
    temp.write_text(json.dumps(data, indent=2) + '\n')
    temp.chmod(0o600)
    os.replace(temp, path)
write_json(app / 'config.json', {'index_path': sys.argv[4]})
manifest = json.loads((root / 'native-host/local.save_to_markdown.json.template').read_text())
manifest['path'] = str(app / 'save-to-markdown-host')
write_json(hosts / 'local.save_to_markdown.json', manifest)
INSTALL
printf 'Installed. Destination: %s\nLoad extension/manifest.json in Firefox about:debugging.\n' "$1"
