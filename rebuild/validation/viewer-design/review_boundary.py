"""Closed, bounded review delivery I/O. Private evidence is never rewritten."""
import hashlib
import html
from html.parser import HTMLParser
import io
import json
import math
import os
from pathlib import Path, PurePosixPath
import re
import stat
import struct
import zipfile
import zlib

# Same per-HTML ceiling as the maintained document publication format. The
# package ceiling is a delivery guard, not a Product runtime/performance SLA.
MEMBER_BYTES = 8 * 1024 * 1024
PACKAGE_BYTES = 128 * 1024 * 1024
MAX_MEMBERS = 4096
CREDENTIAL = re.compile(r'-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----|\bAKIA[A-Z0-9]{16}\b|\bsk-(?:proj-)?[A-Za-z0-9_-]{20,}|\b(?:AWS_SECRET_ACCESS_KEY|api[_-]?key|access[_-]?token|password)\s*[=:]\s*[\"\']?[^\s\"\'<>]{4,}|Authorization\s*:\s*Bearer\s+\S+', re.I)
PRIVATE_PATH = re.compile(r'/(?:home|Users|root|tmp|private|mnt|media)(?:/|$)|/var/(?:folders|tmp|run)(?:/|$)|/run/user(?:/|$)|(?<![A-Za-z0-9])[A-Za-z]:[\\/]', re.I)
RAW_KEYS = {'argv', 'runtime', 'body', 'content', 'record_json', 'stdout', 'stderr', 'user_turn', 'source_bundle', 'error'}


def relative(name):
    if not isinstance(name, str) or not name or '\\' in name or '%' in name or '?' in name or len(name.encode('utf-8')) > 1024 or any(ord(c) < 32 for c in name):
        raise ValueError('unsafe review member path')
    p = PurePosixPath(name)
    if p.is_absolute() or '..' in p.parts or str(p) != name or ':' in name:
        raise ValueError('unsafe review member path')
    return name


def directory(path):
    """Open every component without following symlinks, including root ancestors."""
    path = Path(path).absolute()
    if '..' in path.parts: raise ValueError('review root must not traverse parents')
    fd = os.open('/', os.O_RDONLY | os.O_DIRECTORY)
    try:
        for part in path.parts[1:]:
            nxt = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
            os.close(fd); fd = nxt
        return fd
    except OSError as error:
        os.close(fd)
        raise ValueError('review directory must not traverse a symlink or non-directory') from error


def new_directory(path):
    path = Path(path).absolute()
    parent = directory(path.parent)
    try: os.mkdir(path.name, 0o700, dir_fd=parent)
    finally: os.close(parent)
    return path


def write_new(root, name, data):
    validate(name, data)
    relative(name)
    if len(PurePosixPath(name).parts) != 1: raise ValueError('select a direct output member')
    parent = directory(root)
    try:
        fd = os.open(name, os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, 0o600, dir_fd=parent)
        with os.fdopen(fd, 'wb') as stream: stream.write(data)
    finally: os.close(parent)


def hash_file(path):
    path = Path(path).absolute(); parent = directory(path.parent)
    try:
        fd = os.open(path.name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=parent)
        with os.fdopen(fd, 'rb') as stream:
            before = os.fstat(stream.fileno())
            if not stat.S_ISREG(before.st_mode): raise ValueError('hash input must be regular')
            digest = hashlib.sha256()
            for chunk in iter(lambda: stream.read(1024 * 1024), b''): digest.update(chunk)
            after = os.fstat(stream.fileno())
            if (before.st_size, before.st_mtime_ns) != (after.st_size, after.st_mtime_ns): raise ValueError('hash input changed')
            return digest.hexdigest()
    except OSError as error: raise ValueError('hash input unavailable or is a symlink') from error
    finally: os.close(parent)


def read_member(root, name, limit=MEMBER_BYTES):
    relative(name)
    root_fd = directory(root)
    try:
        parts = PurePosixPath(name).parts
        for part in parts[:-1]:
            nxt = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=root_fd)
            os.close(root_fd); root_fd = nxt
        fd = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=root_fd)
        with os.fdopen(fd, 'rb') as stream:
            before = os.fstat(stream.fileno())
            if not stat.S_ISREG(before.st_mode) or before.st_size > limit:
                raise ValueError('review member must be a bounded regular file')
            data = stream.read(limit + 1)
            after = os.fstat(stream.fileno())
            if len(data) > limit or (before.st_ino, before.st_size, before.st_mtime_ns) != (after.st_ino, after.st_size, after.st_mtime_ns):
                raise ValueError('review member changed or exceeded its bound')
            return data
    except OSError as error:
        raise ValueError('review member unavailable or traverses a symlink') from error
    finally:
        os.close(root_fd)


def safe_text(text):
    if PRIVATE_PATH.search(text):
        raise ValueError('private host path in review delivery')
    if CREDENTIAL.search(text) or 'request_authenticity' in text:
        raise ValueError('credential-looking content or live authenticity in review delivery')


def safe_json(data):
    value = json.loads(data)
    def walk(item):
        if isinstance(item, dict):
            if RAW_KEYS.intersection(item):
                raise ValueError('raw private fields in review metadata')
            for key, value in item.items():
                safe_text(key)
                walk(value)
        elif isinstance(item, list):
            for value in item: walk(value)
        elif isinstance(item, str): safe_text(item)
    walk(value)
    return value


def static_css(value):
    # Inspect executable CSS contexts, not escaped code displayed by the Reader.
    # Escapes and alternate string-based image loaders cannot bypass URL checks.
    value = html.unescape(value)
    if '\\' in value or re.search(r'@\s*import\b|(?:-webkit-)?image-set\s*\(|\bimage\s*\(', value, re.I):
        raise ValueError('unapproved review CSS resource syntax')
    for _, resource in re.findall(r"url\s*\(\s*(['\"]?)(.*?)\1\s*\)", value, re.I | re.S):
        resource = resource.strip()
        if resource.startswith(('#', 'data:image/png;base64,')): continue
        relative(resource)
        if not resource.endswith('.png'): raise ValueError('unapproved CSS resource')


class StaticHtml(HTMLParser):
    def __init__(self):
        super().__init__()
        self.style = None
        self.text = []

    def handle_starttag(self, tag, attrs):
        if tag in {'script', 'form', 'iframe', 'object', 'embed', 'base'}:
            raise ValueError('review HTML must be script-free and without live authority')
        for key, value in attrs:
            value = value or ''
            safe_text(value)
            if key == 'style': static_css(value)
            if key.startswith('on') or key in {'srcdoc', 'srcset', 'poster', 'background', 'formaction'} or (tag == 'meta' and key == 'http-equiv' and value.lower() == 'refresh'):
                raise ValueError('active review HTML')
            if key in {'src', 'action'} and not value.startswith('data:image/png;base64,'):
                relative(value)
                if key == 'action' or not value.endswith('.png'):
                    raise ValueError('review HTML must not acquire external content')
            if (key == 'href' or key.endswith(':href')) and value and not value.startswith('#'):
                relative(value.split('#', 1)[0])
        if tag == 'style': self.style = []

    def handle_data(self, data):
        if self.style is None: self.text.append(data)
        if self.style is not None: self.style.append(data)

    def handle_endtag(self, tag):
        if tag == 'style' and self.style is not None:
            static_css(''.join(self.style))
            self.style = None

    def close(self):
        super().close()
        if self.style is not None: raise ValueError('unclosed review style element')
        # Syntax spans must not split a private path past the delivery check.
        safe_text(''.join(self.text))

    handle_startendtag = handle_starttag


def validate(name, data):
    relative(name)
    if len(data) > MEMBER_BYTES or data.startswith(b'SQLite format 3\x00'):
        raise ValueError('raw database or oversized review member')
    suffix = PurePosixPath(name).suffix
    if suffix == '.png':
        if not data.startswith(b'\x89PNG\r\n\x1a\n'):
            raise ValueError('invalid review PNG')
        offset = 8; first = True; ended = False; expected_pixels = None; inflated = 0
        decoder = zlib.decompressobj()
        while offset + 12 <= len(data):
            size = struct.unpack('>I', data[offset:offset+4])[0]
            kind = data[offset+4:offset+8]; body = data[offset+8:offset+8+size]
            end = offset + size + 12
            if end > len(data) or kind not in {b'IHDR', b'IDAT', b'IEND', b'sRGB', b'gAMA', b'cHRM', b'pHYs'}:
                raise ValueError('PNG contains unreviewed metadata or invalid chunks')
            if zlib.crc32(kind + body) & 0xffffffff != struct.unpack('>I', data[end-4:end])[0]:
                raise ValueError('invalid PNG integrity')
            if first:
                if kind != b'IHDR' or len(body) != 13: raise ValueError('invalid PNG header')
                width, height = struct.unpack('>II', body[:8])
                if not width or not height or width * height > 40_000_000: raise ValueError('oversized PNG dimensions')
                depth, color, compression, filtering, interlace = body[8:]
                if depth != 8 or color not in {2, 6} or compression or filtering or interlace:
                    raise ValueError('review PNG must be normalized RGB/RGBA')
                expected_pixels = height * (1 + width * (3 if color == 2 else 4))
            elif kind == b'IHDR': raise ValueError('duplicate PNG header')
            ancillary_lengths = {b'sRGB': 1, b'gAMA': 4, b'cHRM': 32, b'pHYs': 9}
            if kind in ancillary_lengths and size != ancillary_lengths[kind]:
                raise ValueError('PNG ancillary payload is not a normalized image field')
            if kind == b'IDAT':
                pending = body; block = b''
                try:
                    while pending or len(block) == 65536:
                        block = decoder.decompress(pending, 65536); pending = decoder.unconsumed_tail
                        inflated += len(block)
                        if inflated > expected_pixels or decoder.unused_data: raise ValueError('PNG pixel stream exceeds its declared image')
                except zlib.error as error: raise ValueError('invalid PNG pixel stream') from error
            first = False; offset = end
            if kind == b'IEND':
                if size: raise ValueError('PNG ending contains a payload')
                ended = True; break
        if not ended or offset != len(data) or not decoder.eof or inflated != expected_pixels:
            raise ValueError('invalid PNG ending or pixel stream')
        return
    if suffix not in {'.html', '.json'}:
        raise ValueError('raw/runtime/log/source files are excluded from review delivery')
    text = data.decode('utf-8')
    decoded = html.unescape(text)
    safe_text(decoded)
    if suffix == '.html':
        if not text.lstrip().lower().startswith('<!doctype html'):
            raise ValueError('review HTML requires its static export envelope')
        parser = StaticHtml()
        parser.feed(text)
        parser.close()
    else:
        safe_json(data)


def sanitized_result(result):
    """Keep numeric checks/identities; raw commands, paths, failures and streams stay private."""
    scalar = {'candidate', 'outcome', 'status', 'state', 'screen_reader', 'native_zoom', 'independent_review', 'identity', 'evidence_links', 'exit_code', 'key', 'project', 'work', 'decision', 'evidence_class', 'origin', 'journey', 'view', 'width', 'attempt', 'matches', 'native_anchors', 'bytes', 'exit', 'code', 'signal', 'label', 'duration_ms', 'stdout_bytes', 'stderr_bytes', 'started_at', 'ended_at', 'browser_version', 'read_state_unchanged', 'synthetic_controls', 'human_acceptance', 'repository_state', 'work_count', 'navigation_ms', 'browser_task_ms', 'dom_elements', 'code_elements', 'token_spans', 'code_utf8_bytes', 'visible_text_utf8_bytes'}
    hashes = {'executable_sha256', 'viewer_sha256', 'cli_sha256', 'driver_sha256', 'config_sha256', 'controls_sha256', 'diff_sha256', 'worktree_diff_sha256'}
    containers = {'cases', 'widths', 'source', 'work_source', 'http_reads', 'commands', 'listeners', 'termination', 'process_termination', 'repository_cost', 'source_reads', 'publication'}
    def clean(value):
        if isinstance(value, dict):
            return {key: clean(item) for key, item in value.items() if key in scalar or key in containers or key in hashes}
        if isinstance(value, list): return [clean(item) for item in value[:MAX_MEMBERS]]
        if isinstance(value, str):
            if len(value) > 512: raise ValueError('oversized public review metadata')
            return value
        if value is None or isinstance(value, (bool, int)): return value
        if isinstance(value, float) and math.isfinite(value): return value
        raise ValueError('invalid public review metadata')
    public = clean(result)
    validate('result.json', json.dumps(public).encode())
    return public


def copy_bound(root, destination, names, bindings):
    """Copy bytes sealed by the reviewing driver, with a closed output list."""
    members = []
    for name in sorted(names):
        source = Path(root) / relative(name)
        if not source.exists() and not source.is_symlink(): continue
        data = read_member(root, name)
        expected = bindings.get(name)
        if not isinstance(expected, dict) or expected.get('sha256') != hashlib.sha256(data).hexdigest() or expected.get('bytes') != len(data):
            raise ValueError('output changed since browser review')
        validate(name, data); members.append((name, data))
    if len(members) > MAX_MEMBERS or sum(len(data) for _, data in members) > PACKAGE_BYTES:
        raise ValueError('oversized review delivery')
    new_directory(destination)
    for name, data in members: write_new(destination, name, data)


def inspect_members(members, expected_candidate=None):
    if len(members) > MAX_MEMBERS or sum(len(b) for b in members.values()) > PACKAGE_BYTES:
        raise ValueError('oversized review package')
    for name, data in members.items():
        parts = PurePosixPath(relative(name)).parts
        allowed = len(parts) == 1 and name in {'index.html', 'technical.html', 'manifest.json', 'selected-reference-manifest.json'}
        if len(parts) == 2:
            folder, leaf = parts
            allowed = (folder == 'product' and (leaf == 'result.json' or leaf.endswith('-offline.html') or leaf.endswith('.png'))
                or folder in {'inspection', 'citation'} and (leaf in {'result.json', 'review.html', 'product-snapshot.html'} or leaf.endswith('.png'))
                or folder == 'comparison' and leaf.endswith('.png'))
        if not allowed: raise ValueError('unapproved review member role')
        validate(name, data)
        if name.endswith('/result.json') and json.loads(data) != sanitized_result(json.loads(data)):
            raise ValueError('unsanitized review result')
    manifest = json.loads(members.get('manifest.json', b'{}'))
    candidate = manifest.get('candidate', '')
    if not re.fullmatch(r'[a-f0-9]{40}', candidate) or expected_candidate is not None and candidate != expected_candidate:
        raise ValueError('review candidate identity changed')
    expected = manifest.get('files', {})
    if set(members) - {'manifest.json'} != set(expected): raise ValueError('review package membership changed')
    for name, sha in expected.items():
        relative(name)
        if hashlib.sha256(members[name]).hexdigest() != sha: raise ValueError('review package bytes/path changed')
    return manifest


def verify_package(root, expected_candidate=None):
    root = Path(root).absolute(); fd = directory(root); os.close(fd)
    members = {}
    for path in root.rglob('*'):
        if path.is_symlink(): raise ValueError('review package contains a symlink')
        if path.is_file(): members[path.relative_to(root).as_posix()] = read_member(root, path.relative_to(root).as_posix())
        elif not path.is_dir(): raise ValueError('non-regular review member')
    return inspect_members(members, expected_candidate)


def write_archive(root, archive, expected_candidate):
    verify_package(root, expected_candidate)
    archive = Path(archive).absolute()
    if archive.is_relative_to(Path(root).absolute()): raise ValueError('archive must be outside package membership')
    parent = directory(archive.parent)
    try:
        fd = os.open(archive.name, os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, 0o600, dir_fd=parent)
        with os.fdopen(fd, 'wb') as stream, zipfile.ZipFile(stream, 'w', zipfile.ZIP_DEFLATED) as output:
            for path in sorted(Path(root).rglob('*')):
                if path.is_file():
                    name = path.relative_to(root).as_posix()
                    data = read_member(root, name); validate(name, data)
                    info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0)); info.external_attr = (stat.S_IFREG | 0o600) << 16
                    output.writestr(info, data, compress_type=zipfile.ZIP_DEFLATED)
    finally: os.close(parent)
    verify_archive(archive, expected_candidate)


def archive_directory(data):
    # Reject impossible counts before ZipFile allocates central-directory objects.
    end = data.rfind(b'PK\x05\x06', max(0, len(data) - 65557))
    if end < 0 or end + 22 != len(data): raise ValueError('archive comments/trailers are excluded')
    disk, central_disk, count_disk, count, size, offset, comment = struct.unpack('<4H2IH', data[end+4:end+22])
    if disk or central_disk or count_disk != count or count > MAX_MEMBERS or comment or offset + size != end:
        raise ValueError('invalid or oversized archive directory')
    cursor = offset; observed = 0
    while cursor < end:
        if observed >= MAX_MEMBERS or cursor + 46 > end or data[cursor:cursor+4] != b'PK\x01\x02':
            raise ValueError('invalid archive entry count/header')
        name, extra, note = struct.unpack('<3H', data[cursor+28:cursor+34])
        if name > 1024 or extra or note: raise ValueError('unbound archive metadata')
        cursor += 46 + name; observed += 1
    if cursor != end or observed != count: raise ValueError('archive directory count changed')
    return offset


def verify_archive(path, expected_candidate=None):
    data = read_member(Path(path).absolute().parent, Path(path).name, PACKAGE_BYTES)
    central_offset = archive_directory(data)
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        entries = archive.infolist()
        if len(entries) > MAX_MEMBERS or sum(i.file_size for i in entries) > PACKAGE_BYTES: raise ValueError('oversized review archive')
        names = [relative(i.filename) for i in entries]
        if len(names) != len(set(names)): raise ValueError('duplicate archive member')
        for info in entries:
            if info.is_dir() or stat.S_IFMT(info.external_attr >> 16) not in {0, stat.S_IFREG}: raise ValueError('archive contains non-regular member')
            if info.file_size > MEMBER_BYTES: raise ValueError('oversized archive member')
        cursor = 0
        for info in sorted(entries, key=lambda i: i.header_offset):
            start = info.header_offset
            if start != cursor or start + 30 > central_offset or data[start:start+4] != b'PK\x03\x04':
                raise ValueError('unbound archive bytes or local header')
            flags = struct.unpack('<H', data[start+6:start+8])[0]
            length, extra = struct.unpack('<2H', data[start+26:start+30])
            if flags & 9 or extra: raise ValueError('archive descriptors/encryption/extra payload excluded')
            cursor = start + 30 + length + info.compress_size
        if cursor != central_offset: raise ValueError('unbound archive padding')
        return inspect_members({name: archive.read(name) for name in names}, expected_candidate)
