"""Explicit reading languages and closed static display bindings, not prose grading."""
import hashlib
from html.parser import HTMLParser


def languages(primary, selected, defaults):
    value = selected if selected is not None else list(dict.fromkeys([primary, *defaults]))
    if (not isinstance(value, list) or not 1 <= len(value) <= 16
        or any(not isinstance(s, str) or not s.strip() or s != s.strip()
               or '\0' in s or len(s.encode()) > 128 for s in value)
        or len(set(value)) != len(value)):
        raise ValueError('reader_languages must select 1..16 unique bounded language strings')
    return value


def basename(language):
    # Language text is never interpreted as a filesystem path or a locale.
    return 'viewer-language-' + hashlib.sha256(language.encode()).hexdigest() + '.html'


class Display(HTMLParser):
    def __init__(self):
        super().__init__()
        self.bodies = []
        self.publications = set()
        self.scripts = 0
        self.locales = []

    def handle_starttag(self, tag, attributes):
        attrs = dict(attributes)
        if tag == 'body':
            self.bodies.append(attrs)
        if tag == 'html':
            self.locales.append(attrs.get('lang'))
        if tag == 'script':
            self.scripts += 1
        if value := attrs.get('data-reader-publication'):
            self.publications.add(value)


def inspect(data, project, language, locale):
    parsed = Display()
    parsed.feed(data.decode('utf-8', errors='strict'))
    if (len(parsed.bodies) != 1 or parsed.scripts or parsed.locales != [locale]
        or parsed.bodies[0].get('data-viewer-mode') != 'snapshot'
        or parsed.bodies[0].get('data-project-id') != project
        or parsed.bodies[0].get('data-requested-language') != language):
        raise ValueError('reader snapshot Project/language/mode binding mismatch or active script')
    # This does not assert that a publication exists, that every subject is
    # current, or that its wording is in the requested language.
    return {'requested_language': language, 'locale': locale,
            'publication_ids': sorted(parsed.publications),
            'semantic_quality_verified': False}
