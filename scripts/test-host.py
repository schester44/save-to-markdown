#!/usr/bin/env python3
"""Black-box host checks; uses a temporary HOME and vault, never your real config."""
import concurrent.futures
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile

binary = Path(sys.argv[1] if len(sys.argv) > 1 else 'native-host/target/debug/save-to-markdown-host').resolve()
def frame(obj):
    data = json.dumps(obj).encode()
    return struct.pack('@I', len(data)) + data

def decode(data):
    replies = []
    while data:
        assert len(data) >= 4
        size = struct.unpack('@I', data[:4])[0]
        assert len(data) >= 4 + size
        replies.append(json.loads(data[4:4 + size]))
        data = data[4 + size:]
    return replies

with tempfile.TemporaryDirectory() as tmp:
    home = Path(tmp)
    vault = home / 'Vault with spaces'
    vault.mkdir()
    index = vault / 'Index.md'
    config = home / 'Library/Application Support/SaveToMarkdown/config.json'
    config.parent.mkdir(parents=True)
    config.write_text(json.dumps({'index_path': str(index)}))
    env = dict(os.environ, HOME=str(home))
    def run(payload):
        return subprocess.run([str(binary)], input=payload, capture_output=True, env=env)
    def save(title='Title', **extra):
        request = dict(action='save', title=title, url='https://example.com')
        request.update(extra)
        p = run(frame(request))
        assert p.returncode == 0, p.stderr
        return decode(p.stdout)[0]
    assert save()['ok']
    assert index.read_text() == '- [Title](https://example.com)\n'
    index.write_text('# Existing index')
    assert save()['ok']
    assert index.read_text() == '# Existing index\n- [Title](https://example.com)\n'
    before = index.read_bytes()
    assert not save(file=str(home / 'unexpected.md'))['ok']
    assert not save(url='javascript:alert(1)')['ok']
    assert index.read_bytes() == before
    assert not (home / 'unexpected.md').exists()
    assert save('A [title]\nB')['ok']
    assert index.read_text().endswith('- [A \\[title\\] B](https://example.com)\n')
    # Bad JSON must get a response without breaking the following valid message.
    bad = struct.pack('@I', 1) + b'{'
    good = frame(dict(action='save', title='After error', url='https://example.com'))
    assert [r['ok'] for r in decode(run(bad + good).stdout)] == [False, True]
    assert run(b'').returncode == 0
    for malformed in [b'\x01', struct.pack('@I', 10) + b'{}', struct.pack('@I', 65537)]:
        assert run(malformed).returncode != 0
    index.write_text('')
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        assert all(r['ok'] for r in pool.map(lambda n: save(f'Link {n}'), range(32)))
    lines = index.read_text().splitlines()
    assert len(lines) == 32 and len(set(lines)) == 32
    config.write_text(json.dumps({'index_path': 'relative/Index.md'}))
    assert not save()['ok']
    config.write_text(json.dumps({'index_path': str(home / 'missing/Index.md')}))
    assert not save()['ok']
print('PASS: framing, append/newline, Markdown escaping, path rejection, validation, concurrency, config errors')
