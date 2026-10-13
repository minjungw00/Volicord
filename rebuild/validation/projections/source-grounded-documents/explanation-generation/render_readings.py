"""Compare one immutable output's diagnostic and presentation-only readings."""
import argparse
from pathlib import Path

from inputs import binding, encoded
from render_comparison import render


def compare(attempt, output, *, editorial_order=None, editor=None, reason=None):
    output = Path(output).resolve()
    output.mkdir(mode=0o700, parents=True, exist_ok=False)
    baseline = render([attempt], output / 'diagnostic')
    reader = render([attempt], output / 'presentation', presentation='reader')
    views = [{'kind': 'diagnostic_original', 'display': binding(baseline)},
             {'kind': 'presentation_only', 'display': binding(reader)}]
    editorial_link = ''
    if editorial_order is not None:
        from editorial_reading import create
        receipt = create(attempt, editorial_order, editor, reader, reason, output / 'editorial-reading')
        edited = render([attempt], output / 'editorial', presentation='reader', editorial=receipt)
        views.append({'kind': 'manual_editorial', 'display': binding(edited), 'transformation': binding(receipt)})
        editorial_link = '<li><a href="editorial/comparison.html">Manual editorial reading · original paragraphs reordered</a></li>'
    index = output / 'index.html'
    index.write_text('<!doctype html><html lang="en"><meta charset="utf-8">'
        '<meta name="viewport" content="width=device-width, initial-scale=1">'
        '<title>Compare explanation readings</title><h1>Compare explanation readings</h1>'
        '<p>The same original response, with different presentation. No semantic or human approval.</p>'
        '<ol><li><a href="diagnostic/comparison.html">Diagnostic original · current reader baseline</a></li>'
        '<li><a href="presentation/comparison.html">Reader presentation · unchanged wording and paragraph order</a></li>'
        + editorial_link + '</ol></html>')
    (output / 'views.json').write_bytes(encoded({'attempt': binding(attempt),
        'views': views, 'semantic_assessment': 'not_assessed', 'H1': 'pending'}))
    return index


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--attempt', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--editorial-order', help='Manually selected zero-based paragraph order; complete permutation')
    parser.add_argument('--editor', help='Self-reported manual editor identity')
    parser.add_argument('--reason', help='Observed narrative need remaining in presentation-only view')
    args = parser.parse_args()
    print(compare(args.attempt, args.output,
                  editorial_order=[int(n) for n in args.editorial_order.split(',')] if args.editorial_order else None,
                  editor=args.editor, reason=args.reason))
