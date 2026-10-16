#!/usr/bin/env python3
"""Exercise the actual disposable listener with independent neutral build inputs."""
import argparse
import json
from pathlib import Path
import select
import shutil
import socket
import subprocess
import tempfile
import time

HERE = Path(__file__).resolve().parent


def exercise(binary):
    process = subprocess.Popen([str(binary)], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    checks = []
    try:
        assert select.select([process.stdout], [], [], 10)[0], 'startup deadline'
        url = process.stdout.readline().decode().strip()
        authority = url.removeprefix('http://').rstrip('/')
        address = ('127.0.0.1', int(authority.split(':')[1]))

        def connect():
            return socket.create_connection(address, timeout=5)

        def request(raw, expected):
            with connect() as client:
                client.sendall(raw)
                response = b''
                try:
                    while chunk := client.recv(65536):
                        response += chunk
                except ConnectionResetError:
                    pass
            assert response.startswith(f'HTTP/1.1 {expected} '.encode()), response[:100]

        def recover(label):
            assert process.poll() is None, f'{label}: listener exited {process.returncode}'
            request(f'GET / HTTP/1.1\r\nHost: {authority}\r\n\r\n'.encode(), 200)
            checks.append(label)

        recover('initial GET')
        with connect():
            time.sleep(2.4)
        recover('silent connection beyond read timeout')
        with connect() as client:
            client.sendall(b'GET / HTTP/1.1\r\nHo')
            time.sleep(2.4)
        recover('partial request timeout')
        with connect():
            pass
        recover('disconnect')
        with connect() as client:
            for chunk in [b'GET / HTTP/1.1\r\n', f'hOsT: {authority}\r\n'.encode(), b'\r\n']:
                client.sendall(chunk)
                time.sleep(.05)
            assert client.recv(65536).startswith(b'HTTP/1.1 200 ')
        recover('fragmented valid request')
        # Dribbling bytes cannot restart the two-second total parsing deadline.
        with connect() as client:
            for _ in range(6):
                try:
                    client.sendall(b'G')
                except (BrokenPipeError, ConnectionResetError):
                    break
                time.sleep(.5)
        recover('slow dribble deadline')
        for raw, code in [
            (b'garbage\r\n\r\n', 400),
            (b'GET / HTTP/1.1\r\nHost: evil.invalid\r\n\r\n', 403),
            (f'GET / HTTP/1.1\r\nHost: {authority}\r\nHost: {authority}\r\n\r\n'.encode(), 400),
            (f'POST / HTTP/1.1\r\nHost: {authority}\r\n\r\n'.encode(), 403),
            (f'GET / HTTP/1.1\r\nHost: {authority}\r\nTransfer-Encoding: chunked\r\n\r\n'.encode(), 400),
            (b'GET / HTTP/1.1\r\nX: ' + b'a' * 17000, 431),
        ]:
            request(raw, code)
            recover(f'rejection {code}: {raw[:24]!r}')
        return dict(status='passed', checks=checks, pid=process.pid, alive_after_controls=process.poll() is None)
    finally:
        if process.poll() is None:
            process.terminate()
        stdout, stderr = process.communicate(timeout=5)
        print(json.dumps(dict(listener_exit=process.returncode, termination='explicit SIGTERM cleanup' if process.returncode == -15 else 'natural exit', stdout=stdout.decode(), stderr=stderr.decode())))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path)
    args = parser.parse_args()
    if args.binary:
        print(json.dumps(exercise(args.binary.resolve()), indent=2))
        return
    with tempfile.TemporaryDirectory(prefix='viewer-listener-') as tmp:
        root = Path(tmp)
        shutil.copyfile(HERE / 'reader.rs', root / 'reader.rs')
        (root / 'data.rs').write_text('const WORKS: &[Work] = &[];')
        for name in ['reader.css', 'app.js', 'data.json']:
            (root / name).write_text('')
        subprocess.run(['rustc', '--edition=2021', str(root / 'reader.rs'), '-o', str(root / 'reader')], check=True)
        print(json.dumps(exercise(root / 'reader'), indent=2))


if __name__ == '__main__':
    main()
