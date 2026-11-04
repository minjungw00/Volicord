#!/usr/bin/env python3
"""Execute an installed consumer with an explicit Linux filesystem allowlist.

This validation helper adds restrictions; it never grants host permissions.
A mandatory denied-file probe establishes that the development tree is absent
from the consumer's view. No Product code or runtime policy depends on this file.
"""
import argparse
import ctypes
import errno
import os
from pathlib import Path
import platform
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--read', action='append', default=[])
    parser.add_argument('--write', action='append', default=[])
    parser.add_argument('--list', action='append', default=[],
                        help='directory descriptors/listing only; no file content')
    parser.add_argument('--deny-probe', action='append', required=True)
    parser.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ['--'] else args.command
    if not command:
        parser.error('a consumer command is required')
    if platform.system() != 'Linux' or platform.machine() != 'x86_64':
        print('filesystem isolation unavailable on this validation host', file=sys.stderr)
        return 77
    libc = ctypes.CDLL(None, use_errno=True)
    libc.syscall.restype = ctypes.c_long
    abi = libc.syscall(444, 0, 0, 1)  # LANDLOCK_CREATE_RULESET_VERSION
    if abi < 1:
        print(f'Landlock unavailable: errno {ctypes.get_errno()}', file=sys.stderr)
        return 77

    class Ruleset(ctypes.Structure):
        _fields_ = [('handled_access_fs', ctypes.c_uint64)]

    class Beneath(ctypes.Structure):
        _pack_ = 1
        _fields_ = [('allowed_access', ctypes.c_uint64), ('parent_fd', ctypes.c_int)]

    # ABI-1 filesystem rights; same-directory atomic publication remains allowed.
    handled = (1 << 13) - 1
    ruleset = Ruleset(handled)
    fd = libc.syscall(444, ctypes.byref(ruleset), ctypes.sizeof(ruleset), 0)
    if fd < 0:
        raise OSError(ctypes.get_errno(), 'create Landlock ruleset')
    try:
        access = ([(p, (1 << 0) | (1 << 2) | (1 << 3)) for p in args.read]
                  + [(p, handled) for p in args.write]
                  + [(p, 1 << 3) for p in args.list])
        for path, rights in access:
            path = Path(path).resolve(strict=True)
            descriptor = os.open(path, os.O_PATH | os.O_CLOEXEC)
            try:
                if not path.is_dir():
                    rights &= (1 << 0) | (1 << 1) | (1 << 2)
                rule = Beneath(rights, descriptor)
                if libc.syscall(445, fd, 1, ctypes.byref(rule), 0) < 0:
                    raise OSError(ctypes.get_errno(), f'add rule: {path}')
            finally:
                os.close(descriptor)
        if libc.prctl(38, 1, 0, 0, 0) != 0:  # PR_SET_NO_NEW_PRIVS
            raise OSError(ctypes.get_errno(), 'no_new_privs')
        if libc.syscall(446, fd, 0) < 0:
            raise OSError(ctypes.get_errno(), 'restrict filesystem')
    finally:
        os.close(fd)
    for probe in args.deny_probe:
        try:
            with open(probe, 'rb') as stream:
                stream.read(1)
        except PermissionError as error:
            if error.errno != errno.EACCES:
                raise
        else:
            raise RuntimeError(f'development-file probe was not denied: {probe}')
    print(f'Landlock ABI {abi}: {len(args.deny_probe)} development-file probes denied', file=sys.stderr)
    os.execvpe(command[0], command, os.environ)


if __name__ == '__main__':
    raise SystemExit(main())
