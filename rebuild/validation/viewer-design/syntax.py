"""Build-time lexical coloring; exact Source text remains independently checked."""
from html import escape
from pathlib import PurePosixPath
from pygments.lexers import get_lexer_by_name
from pygments.token import Token


def lines(text, locator):
    language = {'.rs':'rust','.py':'python'}.get(PurePosixPath(locator).suffix)
    if language is None:
        return [escape(line.rstrip('\n')) for line in text.splitlines(keepends=True)]
    lexer = get_lexer_by_name(language, stripnl=False, ensurenl=False)
    tokens = list(lexer.get_tokens_unprocessed(text))
    if ''.join(value for _,_,value in tokens) != text:
        raise ValueError('syntax lexer changed retained Source bytes')
    rendered = ['']
    for _,kind,value in tokens:
        css = None
        for family,name in [(Token.Comment,'comment'),(Token.Keyword,'keyword'),
                            (Token.Literal.String,'string'),(Token.Literal.Number,'number'),
                            (Token.Name.Function,'function'),(Token.Name.Class,'function'),
                            (Token.Operator,'operator')]:
            if kind in family:
                css = 'syntax-' + name; break
        for n,part in enumerate(value.split('\n')):
            if n:
                rendered.append('')
            if part:
                rendered[-1] += f'<span class="{css}">{escape(part)}</span>' if css else escape(part)
    if text.endswith('\n'):
        rendered.pop()
    return rendered
