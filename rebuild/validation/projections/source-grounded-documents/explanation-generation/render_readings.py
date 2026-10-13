"""Compare one immutable output's diagnostic and presentation-only readings."""
import argparse
from pathlib import Path

from inputs import binding, encoded
from render_comparison import render


def compare(attempt, output):
    output = Path(output).resolve()
    output.mkdir(mode=0o700, parents=True, exist_ok=False)
    baseline = render([attempt], output / 'diagnostic')
    reader = render([attempt], output / 'presentation', presentation='reader')
    index = output / 'index.html'
    index.write_text('<!doctype html><html lang="en"><meta charset="utf-8">'
        '<meta name="viewport" content="width=device-width, initial-scale=1">'
        '<title>Compare explanation readings</title><h1>Compare explanation readings</h1>'
        '<p>The same original response, with different presentation. No semantic or human approval.</p>'
        '<ol><li><a href="diagnostic/comparison.html">Diagnostic original · current reader baseline</a></li>'
        '<li><a href="presentation/comparison.html">Reader presentation · unchanged wording and paragraph order</a></li></ol></html>')
    (output / 'views.json').write_bytes(encoded({'attempt': binding(attempt),
        'views': [{'kind': 'diagnostic_original', 'display': binding(baseline)},
                  {'kind': 'presentation_only', 'display': binding(reader)}], 'semantic_assessment': 'not_assessed', 'H1': 'pending'}))
    return index


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--attempt', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    print(compare(args.attempt, args.output))
