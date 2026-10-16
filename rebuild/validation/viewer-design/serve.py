#!/usr/bin/env python3
"""Read-only loopback review server. Explicit manifest routes, no filesystem fallback."""
import json
from pathlib import Path
import socket
import socketserver
import sys
import time
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parent
TYPES = {'.html':'text/html', '.js':'text/javascript', '.css':'text/css', '.json':'application/json', '.png':'image/png', '.txt':'text/plain', '.py':'text/plain', '.rs':'text/plain'}


def parse(raw, authority):
    try:
        lines=raw.decode('utf-8').split('\r\n'); method,target,version=lines[0].split(' ')
        if version!='HTTP/1.1' or not target.startswith('/') or target.startswith('//') or any(ord(c)<=32 for c in target):
            return 400,None
        host=[]
        for line in lines[1:]:
            if not line:break
            name,value=line.split(':',1)
            if not name or not all(c.isascii() and (c.isalnum() or c=='-') for c in name) or any(ord(c)<32 or ord(c)==127 for c in value):return 400,None
            if name.lower()=='host':host.append(value.strip())
            if name.lower()=='transfer-encoding' or (name.lower()=='content-length' and value.strip()!='0'):return 400,None
        if len(host)>1:return 400,None
        if method!='GET' or host!=[authority]:return 403,None
        return 200,urlsplit(target).path
    except (ValueError,UnicodeError):return 400,None


class Connection(socketserver.BaseRequestHandler):
    def handle(self):
        status=400; path=None; raw=b''; deadline=time.monotonic()+2
        try:
            while b'\r\n\r\n' not in raw:
                remaining=deadline-time.monotonic()
                if remaining<=0:raise TimeoutError()
                self.request.settimeout(remaining)
                chunk=self.request.recv(1024)
                if not chunk:break
                raw+=chunk
                if len(raw)>16384:status=431;break
            else:
                if raw.endswith(b'\r\n\r\n'):
                    status,path=parse(raw,self.server.authority)
            if status==200:
                name='index.html' if path=='/' else path[1:]
                if name not in self.server.routes:status=404
                else:body=(ROOT/name).read_bytes();kind=TYPES.get(Path(name).suffix,'text/plain')
            if status!=200:body=f'Review request rejected: {status}'.encode();kind='text/plain'
            self.respond(status,kind,body)
        except (TimeoutError,socket.timeout):
            try:self.respond(408,'text/plain',b'Request timeout')
            except OSError:pass
        except OSError as error:
            print(f'connection failed: {error}',file=sys.stderr)

    def respond(self,status,kind,body):
        self.request.settimeout(2)
        head=f"HTTP/1.1 {status} Response\r\nContent-Type: {kind}; charset=utf-8\r\nContent-Length: {len(body)}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'; script-src 'self'; style-src 'unsafe-inline'; img-src 'self'; base-uri 'none'; frame-ancestors 'none'\r\n\r\n"
        self.request.sendall(head.encode()+body)


def main():
    manifest=json.loads((ROOT/'manifest.json').read_text())
    with socketserver.TCPServer(('127.0.0.1',0),Connection) as server:
        server.authority=f'127.0.0.1:{server.server_address[1]}'
        server.routes=set(manifest['files'])|{'manifest.json'}
        print(f'http://{server.authority}/',flush=True)
        try:server.serve_forever(poll_interval=.1)
        except KeyboardInterrupt:pass


if __name__=='__main__':main()
