#!/usr/bin/env python3
"""Disjoint serialized HTML accounting; no semantic or qualification verdict.

Every original UTF-8 byte belongs to exactly one region. Nested code/style/SVG
bytes are removed from containing evidence/metadata regions, never added twice.
Inputs are read-only. Browser work and actual transport require separate probes.
"""
from collections import Counter
from html.parser import HTMLParser
import argparse
import gzip
import hashlib
import json
import re
from pathlib import Path


class Audit(HTMLParser):
    VOID = set('area base br col embed hr img input link meta param source track wbr'.split())

    def __init__(self, source):
        super().__init__(convert_charrefs=False)
        self.source = source
        self.lines = [0]
        for index, character in enumerate(source):
            if character == '\n':
                self.lines.append(index + 1)
        self.stack = []
        self.segments = []
        self.counts = Counter()
        self.last = 0
        self.feed(source)
        self.close()
        self.capture(len(source))

    def position(self):
        line, column = self.getpos()
        return self.lines[line - 1] + column

    def region(self):
        regions = [region for _, region in self.stack]
        for region in ('css', 'script', 'svg', 'code', 'metadata', 'origin_context', 'origin_alias', 'claim_context'):
            if region in regions:
                return region
        return 'other_html'

    def capture(self, end):
        if end > self.last:
            self.segments.append((self.region(), len(self.source[self.last:end].encode())))
            self.last = end

    def handle_starttag(self, tag, attributes):
        self.capture(self.position())
        self.counts['dom_elements'] += 1
        classes = dict(attributes).get('class', '').split()
        region = {'style': 'css', 'script': 'script', 'svg': 'svg', 'code': 'code'}.get(tag)
        for name, candidate in [('source-metadata', 'metadata'), ('source-origin-context', 'origin_context'),
                                ('source-origin-target', 'origin_alias'), ('source-point', 'claim_context')]:
            if name in classes:
                self.counts[name] += 1
                region = candidate
        if tag == 'style':
            self.counts['style_elements'] += 1
        self.stack.append((tag, region))
        raw_tag = self.get_starttag_text()
        attributes = re.finditer(r'''\s+([^\s=/>]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+)))?''', raw_tag)
        for attribute in attributes:
            if attribute.group(1).lower() != 'style':
                continue
            value = next((n for n in (2, 3, 4) if attribute.group(n) is not None), None)
            if value is None:
                continue
            self.capture(self.position() + attribute.start(value))
            self.stack.append(('attribute', 'css'))
            self.capture(self.position() + attribute.end(value))
            self.stack.pop()
        self.capture(self.position() + len(raw_tag))
        if tag in self.VOID:
            self.stack.pop()

    def handle_startendtag(self, tag, attributes):
        self.handle_starttag(tag, attributes)
        if tag not in self.VOID:
            self.stack.pop()

    def handle_endtag(self, tag):
        start = self.position()
        self.capture(start)
        self.capture(self.source.index('>', start) + 1)
        for index in range(len(self.stack) - 1, -1, -1):
            if self.stack[index][0] == tag:
                del self.stack[index:]
                break

    def handle_data(self, value):
        self.capture(self.position() + len(value))

    def handle_entityref(self, name):
        self.capture(self.position() + len(name) + 2)

    def handle_charref(self, name):
        self.capture(self.position() + len(name) + 3)


def measure(path):
    raw = Path(path).read_bytes()
    audit = Audit(raw.decode())
    regions = Counter()
    for name, size in audit.segments:
        regions[name] += size
    assert sum(regions.values()) == len(raw), 'disjoint regions must partition original bytes'
    return dict(path=str(Path(path).resolve()), sha256=hashlib.sha256(raw).hexdigest(),
                raw_bytes=len(raw), gzip_bytes_compressibility_only=len(gzip.compress(raw, mtime=0)),
                disjoint_utf8_bytes=dict(regions), counts=dict(audit.counts))


def self_test():
    code = '<code><span>full &amp; 한글</span></code>'
    svg = '<svg><text>diagram</text></svg>'
    css = '<style>p{color:blue}</style>'
    metadata = '<details class="source-metadata">' + code + '</details>'
    raw = css + '''<aside title="style='inert'" data-style="inert" style="color:blue">''' + svg + metadata + '</aside>'
    audit = Audit(raw)
    counts = Counter()
    for name, size in audit.segments:
        counts[name] += size
    assert sum(counts.values()) == len(raw.encode())
    assert counts['code'] == len(code.encode())
    assert counts['svg'] == len(svg.encode())
    assert counts['metadata'] == len(metadata.encode()) - len(code.encode())
    assert counts['css'] == len(css.encode()) + len('color:blue')
    print('disjoint UTF-8/inline-style/nested-code accounting passed')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('html', nargs='*')
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        self_test()
    else:
        parser.error('provide HTML paths or --self-test') if not args.html else None
        print(json.dumps([measure(path) for path in args.html], indent=2))
