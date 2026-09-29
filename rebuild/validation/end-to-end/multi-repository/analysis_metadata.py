"""Read the current content-addressed Analysis metadata blob for V11 evidence."""
from __future__ import annotations

import ctypes
import hashlib
import json
from pathlib import Path

RAW_MAGIC = b"VOLICORD-BLOB-RAW1\0"
ZSTD_MAGIC = b"VOLICORD-BLOB-ZSTD1\0"
MAX_METADATA_BYTES = 64 * 1024 * 1024


def read_metadata(manifest_path: Path, digest: str) -> dict:
    if len(digest) != 64 or any(char not in "0123456789abcdef" for char in digest):
        raise ValueError("invalid Analysis metadata blob identity")
    blob = (manifest_path.parent / "blobs" / f"{digest}.metadata").read_bytes()
    if hashlib.sha256(blob).hexdigest() != digest:
        raise ValueError("Analysis metadata blob content identity mismatch")
    if blob.startswith(RAW_MAGIC):
        payload = blob[len(RAW_MAGIC):]
    elif blob.startswith(ZSTD_MAGIC):
        header = len(ZSTD_MAGIC)
        if len(blob) < header + 8:
            raise ValueError("Analysis metadata blob is truncated")
        length = int.from_bytes(blob[header:header + 8], "little")
        if length > MAX_METADATA_BYTES:
            raise ValueError("Analysis metadata blob exceeds the bounded decoder")
        source = blob[header + 8:]
        library = ctypes.CDLL("libzstd.so.1")
        library.ZSTD_decompress.argtypes = [ctypes.c_void_p, ctypes.c_size_t,
                                            ctypes.c_void_p, ctypes.c_size_t]
        library.ZSTD_decompress.restype = ctypes.c_size_t
        output = ctypes.create_string_buffer(length)
        decoded = library.ZSTD_decompress(output, length, source, len(source))
        if decoded != length:
            raise ValueError("Analysis metadata blob cannot be decompressed")
        payload = output.raw
    else:
        raise ValueError("unsupported Analysis metadata blob encoding")
    if len(payload) > MAX_METADATA_BYTES:
        raise ValueError("Analysis metadata blob exceeds the bounded decoder")
    metadata = json.loads(payload)
    if not isinstance(metadata, dict):
        raise ValueError("Analysis metadata is not an object")
    return metadata


def self_check() -> None:
    """Exercise the packed representation used by Production and reject tampering."""
    import tempfile

    payload = json.dumps({"identity": "a" * 64, "capabilities": ["x" * 4096]}).encode()
    library = ctypes.CDLL("libzstd.so.1")
    library.ZSTD_compressBound.argtypes = [ctypes.c_size_t]
    library.ZSTD_compressBound.restype = ctypes.c_size_t
    library.ZSTD_compress.argtypes = [ctypes.c_void_p, ctypes.c_size_t,
                                      ctypes.c_void_p, ctypes.c_size_t, ctypes.c_int]
    library.ZSTD_compress.restype = ctypes.c_size_t
    compressed = ctypes.create_string_buffer(library.ZSTD_compressBound(len(payload)))
    length = library.ZSTD_compress(compressed, len(compressed), payload, len(payload), 1)
    packed = ZSTD_MAGIC + len(payload).to_bytes(8, "little") + compressed.raw[:length]
    with tempfile.TemporaryDirectory() as temporary:
        manifest_path = Path(temporary) / "analysis.json"
        blobs = manifest_path.parent / "blobs"
        blobs.mkdir()
        digest = hashlib.sha256(packed).hexdigest()
        blob = blobs / f"{digest}.metadata"
        blob.write_bytes(packed)
        assert read_metadata(manifest_path, digest) == json.loads(payload)
        blob.write_bytes(packed[:-1])
        try:
            read_metadata(manifest_path, digest)
        except ValueError:
            pass
        else:
            raise AssertionError("corrupt Analysis metadata blob was accepted")
