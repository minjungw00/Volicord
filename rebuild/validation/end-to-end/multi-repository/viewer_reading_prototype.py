#!/usr/bin/env python3
"""Recreate a disposable navigation sketch from maintained synthetic inputs."""

from html import escape
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[4]
FIXTURE = Path(__file__).parent / "fixtures/viewer-reading/scenario.json"


def main() -> None:
    scenario = json.loads(FIXTURE.read_text(encoding="utf-8"))
    parts = ["<!doctype html><html lang='en'><meta charset='utf-8'>",
             "<title>Work reading prototype</title><h1>Overview</h1>",
             "<p>Disposable prototype; synthetic inputs, no canonical facts.</p>",
             "<p>Project Purpose: " + escape(scenario["purpose_present"]) + "</p><nav>"]
    for work in scenario["works"]:
        parts.append(f'<a href="#work-{work["key"]}">{escape(work["title"])} ({work["key"]})</a> ')
    parts.append("</nav>")
    for work in scenario["works"]:
        key = work["key"]
        parts.append(f'<section id="work-{key}"><h2>{escape(work["title"])}</h2>')
        parts.append('<p>Prototype annotation, not canonical explanation: ' +
                     escape(scenario["prototype_annotation"]["purpose"]) + "</p>")
        if not work["checkpoints"]:
            parts.append("<p>Goal only; result and next step unavailable. Code gap: no seeds.</p>")
        for checkpoint in work["checkpoints"]:
            evidence = f'evidence-{key}-{checkpoint["key"]}'
            parts.append("<p>Work: " + checkpoint["work"] + "; verification: " +
                         checkpoint["verification"] + "; user review: " + checkpoint["review"] +
                         "; acceptance: " + checkpoint["acceptance"] + "</p>")
            parts.append("<p>Next: " + escape(checkpoint["next_step"]) +
                         f' <a href="#{evidence}">Exact evidence</a></p>')
            parts.append(f'<details id="{evidence}"><summary>Original Checkpoint ({checkpoint["key"]})</summary><pre>' +
                         escape(json.dumps(checkpoint, ensure_ascii=False, indent=2)) + "</pre></details>")
        parts.append(f'<p><a href="#decisions-{key}">Decisions</a> <a href="#code-{key}">Code Understanding</a></p>')
        parts.append(f'<h3 id="decisions-{key}">Decisions</h3>')
        for decision in scenario["decisions"]:
            if decision["scope"] == key or (key == "older" and decision["scope"] in ("ProjectWide", "Unresolved")):
                parts.append("<details><summary>" + escape(decision["key"]) + " / " +
                             escape(decision["scope"]) + "</summary><pre>" +
                             escape(json.dumps(decision, ensure_ascii=False, indent=2)) + "</pre></details>")
        parts.append(f'<h3 id="code-{key}">Code Understanding</h3><p>' +
                     escape(", ".join(work["paths"]) or "Code unavailable: no seeds") + "</p></section>")
    parts.append("<details><summary>Tools (separate from reading)</summary><p>Future production surface.</p></details></html>")
    output = ROOT / "rebuild/.local/viewer-reading/prototype.html"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text("\n".join(parts), encoding="utf-8")
    page = output.read_text(encoding="utf-8")
    for work in scenario["works"]:
        assert f'id="work-{work["key"]}"' in page
        assert f'id="code-{work["key"]}"' in page
    assert "Prototype annotation, not canonical explanation" in page
    assert "<script" not in page and "<form" not in page
    print(f"prototype recreated and inspected: {output}")


if __name__ == "__main__":
    main()
