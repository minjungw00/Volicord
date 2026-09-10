"""Private active-host realization regressions; no naturalistic source data."""
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import os
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import campaign as c
import campaign_self_test as fixtures
import harness as h
import document_realization as r


def plan_for(arguments):
    return {"document_kind": arguments["kind"], "requested_language": arguments["language"],
        "plan_fingerprint": "sha256:" + hashlib.sha256((arguments["kind"] + arguments["project_id"]).encode()).hexdigest(),
        "source_title": "Project handoff", "generator": {"generator": "volicord-codex-host", "agent": "codex", "model": None},
        "sections": [{"key": "current", "source_title": "Current work", "claims": [{"identity": "work-1",
            "source_text": "The source is src/example.py and uses Example.", "protected_terms": ["src/example.py", "Example"],
            "class": "canonical", "source_basis": [], "decision_basis": [], "analysis_basis": [],
            "source_text_omission": None, "omitted_protected_term_count": 0}]}]}


def product_preview(_binary, _runtime, arguments):
    result = {"kind": arguments["kind"], "format": arguments["format"],
        "requested_language": arguments["language"], "canonical_mutation": False}
    if "realization" not in arguments:
        return {**result, "outcome": "realization_required", "plan": plan_for(arguments)}
    return {**result, "outcome": "realized", "generator": arguments["realization"]["generator"],
        "content": "# 작업 인수인계\n확인할 소스: src/example.py, Example\n" if arguments["format"] == "markdown"
            else '<!doctype html><html lang="ko-KR"><body>작업 인수인계: src/example.py, Example</body></html>'}


def complete_synthetic_draft(value):
    """Literal test input standing in for an active host, never a campaign realizer."""
    value = deepcopy(value)
    value["all_generated_prose_realized"] = True
    value["realization"]["title"] = "프로젝트 인수인계"
    value["realization"]["generator"] = {"generator": "synthetic-test-host", "agent": "fixture", "model": "fixture"}
    value["realization"]["sections"][0]["title"] = "현재 작업"
    value["realization"]["sections"][0]["claims"][0]["text"] = "소스 src/example.py에서 Example을 사용합니다."
    return value


def snapshot(root):
    return {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in root.rglob("*") if p.is_file()}


class DocumentRealizationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.parent = Path(self.temporary.name)
        self.binary = self.parent / "candidate/bin/volicord"
        fixtures.write_fake_binary(self.binary)
        self.clean = patch.object(h, "git_clean", return_value=True)
        self.clean.start()
        self.addCleanup(self.clean.stop)

    def test_cross_locale_blocks_before_terminal_publication(self):
        root, captures, bundles = fixtures.prepared_batch(self.parent, "missing-realization", self.binary)
        campaign = c.load_campaign(root)
        campaign["document_language"] = "ko-KR"
        c.save_campaign(root, campaign)
        before = snapshot(root)
        with self.assertRaises(c.CampaignError):
            c.collect_batch(root, captures, exporter=fixtures.batch_exporter(bundles),
                            documenter=fixtures.documenter, snapshotter=fixtures.snapshotter)
        self.assertEqual(snapshot(root), before)

    def prepared(self):
        root, captures, bundles = fixtures.prepared_batch(self.parent, "realizations", self.binary)
        campaign = c.load_campaign(root)
        campaign.update(document_language="ko-KR", document_realization_route=r.route(self.binary))
        c.save_campaign(root, campaign)
        return root, captures, bundles

    def test_private_preparation_fix_and_batch_both_formats(self):
        root, captures, bundles = self.prepared()
        raw_before = {p: p.read_bytes() for p in captures}
        with patch.object(r, "preview", side_effect=product_preview):
            result = r.prepare(root, list(reversed(captures)))
            self.assertEqual(result["qualification_state"], "not_run")
            index = c.read_json(root / result["index"])
            self.assertEqual(len(index["documents"]), 32)
            visible = "\n".join(p.read_text() for p in (root / "realizer").rglob("*.json"))
            for secret in (*c.BEHAVIOR_CLASSES, "expected_qualification", "evaluation_basis", "cycle_key", "raw_inputs"):
                self.assertNotIn(secret, visible)
            before = snapshot(root)
            with self.assertRaises(c.CampaignError):
                c.collect_batch(root, captures)
            self.assertEqual(snapshot(root), before)
            for entry in index["documents"]:
                draft_path = root / entry["draft"]
                c.write_json(draft_path, complete_synthetic_draft(c.read_json(draft_path)))
                before = snapshot(root)
                r.validate(root, entry["realization_id"], draft_path)
                self.assertEqual(snapshot(root), before)
                r.record(root, entry["realization_id"], draft_path)
                recorded = r.artifact(root, "recorded", entry["realization_id"])
                exact = recorded.read_bytes()
                draft_path.write_text("changed after recording")
                self.assertEqual(recorded.read_bytes(), exact)
            before = snapshot(root)
            with self.assertRaises(c.CampaignError):
                r.record(root, entry["realization_id"], draft_path)
            self.assertEqual(snapshot(root), before)
            summary = c.collect_batch(root, captures, exporter=fixtures.batch_exporter(bundles),
                documenter=lambda *args: self.fail("cross-locale documents used the CLI exporter"),
                snapshotter=lambda binary, runtime, project, destination, locale, language:
                    fixtures.snapshotter(binary, runtime, project, destination, locale, "en"))
            self.assertIsNone(c.load_campaign(root)["terminal_outcome"], summary)
            paths = list(root.glob("slots/*/evidence/generated-documents/*"))
            self.assertEqual(len(paths), 64)
            self.assertTrue(all("인수인계" in p.read_text() for p in paths))
            self.assertFalse(c.safe_archive_artifact("realizer/plans/anything.json", include_raw=True))
        self.assertEqual({p: p.read_bytes() for p in captures}, raw_before)

    def test_invalid_drafts_and_immutable_bindings(self):
        root, captures, _ = self.prepared()
        with patch.object(r, "preview", side_effect=product_preview):
            r.prepare(root, captures)
            entry = c.read_json(root / "realizer/index.json")["documents"][0]
            identity, draft_path = entry["realization_id"], root / entry["draft"]
            valid = complete_synthetic_draft(c.read_json(draft_path))
            cases = []
            def case(change):
                value = deepcopy(valid)
                change(value)
                cases.append(value)
            case(lambda v: v.update(preparation_sha256="0" * 64))
            case(lambda v: v.update(requested_language="en"))
            case(lambda v: v.update(all_generated_prose_realized=False))
            case(lambda v: v["realization"].update(plan_fingerprint="0" * 64))
            case(lambda v: v["realization"].update(title="Project handoff"))
            case(lambda v: v["realization"].update(generator={"generator": "fake"}))
            case(lambda v: v["realization"].update(sections=[]))
            case(lambda v: v["realization"]["sections"].append(deepcopy(v["realization"]["sections"][0])))
            case(lambda v: v["realization"]["sections"][0].update(claims=[]))
            case(lambda v: v["realization"]["sections"][0]["claims"].append({"identity": "extra", "text": "추가"}))
            case(lambda v: v["realization"]["sections"][0]["claims"][0].update(identity="wrong"))
            case(lambda v: v["realization"]["sections"][0]["claims"][0].update(text="번역된 경로"))
            for value in cases:
                c.write_json(draft_path, value)
                before = snapshot(root)
                with self.assertRaises(c.CampaignError):
                    r.validate(root, identity, draft_path)
                with self.assertRaises(c.CampaignError):
                    r.record(root, identity, draft_path)
                self.assertEqual(snapshot(root), before)
            c.write_json(draft_path, valid)
            before = snapshot(root)
            with patch.object(r, "consume", side_effect=c.CampaignError("Product rejected realization")):
                with self.assertRaises(c.CampaignError):
                    r.record(root, identity, draft_path)
            self.assertEqual(snapshot(root), before)
            r.record(root, identity, draft_path)
            with self.assertRaises(c.CampaignError):
                r.validate(root, identity, r.artifact(root, "recorded", identity))
            r.artifact(root, "recorded", identity).write_text("tampered")
            before = snapshot(root)
            with self.assertRaises(c.CampaignError):
                c.collect_batch(root, captures)
            self.assertEqual(snapshot(root), before)

    def test_same_locale_fixed_export_needs_no_realizer(self):
        for language, locale in (("en", "en"), ("en-US", "en"), ("ko-KR", "ko")):
            self.assertFalse(r.required(language, locale))
        root, captures, bundles = fixtures.prepared_batch(self.parent, "fixed-locale", self.binary)
        with patch.object(r, "preview", side_effect=AssertionError("same-locale preview should use CLI")):
            c.collect_batch(root, captures, exporter=fixtures.batch_exporter(bundles),
                documenter=fixtures.documenter, snapshotter=fixtures.snapshotter)
        self.assertIsNone(c.load_campaign(root)["terminal_outcome"])

    def test_preflight_is_realizer_only_and_publication_rolls_back(self):
        root, captures, _ = self.prepared()
        with patch.object(r, "preview", side_effect=product_preview):
            r.prepare(root, captures)
            entry = c.read_json(root / "realizer/index.json")["documents"][0]
            draft = root / entry["draft"]
            c.write_json(draft, complete_synthetic_draft(c.read_json(draft)))
            before = snapshot(root)
            with patch.object(c, "load_campaign", side_effect=AssertionError("preflight read private campaign")):
                r.validate(root, entry["realization_id"], draft)
            cli = subprocess.run([str(c.ROOT / "rebuild/scripts/dogfood-campaign"), "validate-document-realization",
                "--campaign-root", str(root), "--realization-id", entry["realization_id"], "--draft", str(draft)],
                text=True, capture_output=True)
            self.assertEqual(cli.returncode, 0, cli.stdout + cli.stderr)
            self.assertEqual(json.loads(cli.stdout)["state"], "valid")
            self.assertEqual(snapshot(root), before)
            original = c.atomic_write_bytes
            failed = False
            def fail_inventory(path, data):
                nonlocal failed
                if path == c.inventory_path(root) and not failed:
                    failed = True
                    raise OSError("synthetic inventory publication failure")
                original(path, data)
            with patch.object(c, "atomic_write_bytes", side_effect=fail_inventory):
                with self.assertRaises(OSError):
                    r.record(root, entry["realization_id"], draft)
            self.assertEqual(snapshot(root), before)
            old_capture = captures[0].read_bytes()
            captures[0].write_bytes(b" " + old_capture)
            mapped = c.map_batch_rollouts(root, captures)
            with self.assertRaises(c.CampaignError):
                r.require_batch_ready(root, c.load_campaign(root), mapped)
            self.assertEqual(snapshot(root), before)

    def test_missing_candidate_route_and_unavailable_plan_are_preparation_states(self):
        root, captures, _ = self.prepared()
        binary = self.binary.with_name("volicord-mcp")
        old = binary.read_bytes()
        binary.write_bytes(b"changed candidate")
        before = snapshot(root)
        with self.assertRaises(c.CampaignError):
            r.prepare(root, captures)
        self.assertEqual(snapshot(root), before)
        binary.write_bytes(old)
        with patch.object(r, "preview", return_value={"outcome": "unavailable", "canonical_mutation": False}):
            with self.assertRaisesRegex(c.CampaignError, "preparation required"):
                r.prepare(root, captures)
        self.assertEqual(snapshot(root), before)

    def test_actual_product_contract_and_canonical_purity(self):
        binary = c.ROOT / "rebuild/target/debug/volicord"
        subprocess.run(["cargo", "build", "--manifest-path", str(c.ROOT / "rebuild/Cargo.toml"),
            "--bin", "volicord", "--bin", "volicord-mcp"], check=True)
        runtime, repository = self.parent / "product-runtime", self.parent / "repository"
        repository.mkdir()
        (repository / "example.py").write_text("def example():\n    return 1\n")
        client = h.load_v11().Mcp(binary.with_name("volicord-mcp"),
            {**os.environ, "VOLICORD_RUNTIME_DIR": str(runtime)})
        try:
            client.initialize()
            project, ok = client.tool("project_initialize", {"display_name": "Example", "repository": str(repository)})
            self.assertTrue(ok)
            project_id = project["project_id"]
            _, ok = client.tool("repository_analyze", {"project_id": project_id})
            self.assertTrue(ok)
        finally:
            self.assertEqual(client.close()["exit_code"], 0)
        before_path = self.parent / "before.bundle.json"
        c.default_export(binary, runtime, repository, before_path)
        before = h.load_canonical_bundle(before_path)
        destination = self.parent / "export/evidence/generated-documents/english.md"
        destination.parent.mkdir(parents=True)
        fixed = c.generate_document(binary, runtime, repository, "handoff-resume", "markdown", destination, "en", "en")
        self.assertEqual(fixed["status"], "passed")
        unavailable = c.generate_document(binary, runtime, repository, "handoff-resume", "markdown",
            destination.with_name("unavailable.md"), "ko-KR", "en")
        self.assertEqual(unavailable["product_result"]["outcome"], "unavailable")
        self.assertEqual(unavailable["_process_result"]["exit_code"], 0)
        protected_tested = False
        for kind in c.DOCUMENT_KINDS:
            preparation = {"project_id": project_id, "document_kind": kind, "language": "ko-KR", "locale": "en"}
            plan = r.current_plan(binary, runtime, preparation, "markdown")
            self.assertEqual(plan, r.current_plan(binary, runtime, preparation, "html"))
            preparation["plan"] = plan
            # Synthetic host input tests topology/grounding transport, not language quality.
            realization = {"plan_fingerprint": plan["plan_fingerprint"], "title": "프로젝트 설명",
                "generator": {"generator": "contract-test-host", "agent": "fixture", "model": "fixture"},
                "sections": [{"key": s["key"], "title": "프로젝트 상태",
                    "claims": [{"identity": claim["identity"], "text": "확인할 프로젝트 근거: " + " ".join(claim["protected_terms"])}
                        for claim in s["claims"]]} for s in plan["sections"]]}
            for format_name, _ in c.DOCUMENT_FORMATS:
                content = r.consume(binary, runtime, preparation, {"realization": realization}, format_name)
                self.assertIn("프로젝트 설명", content)
                if format_name == "html":
                    self.assertIn('lang="ko-kr"', content.lower())
            if kind != "project-architecture-guide":
                continue
            invalid = []
            def case(change):
                value = deepcopy(realization)
                change(value)
                invalid.append(value)
            case(lambda v: v.update(plan_fingerprint="0" * 64))
            case(lambda v: v.update(title=""))
            case(lambda v: v.update(generator={"generator": "fixture", "agent": "fixture", "model": ""}))
            case(lambda v: v["sections"].pop())
            case(lambda v: v["sections"].append(deepcopy(v["sections"][0])))
            section_index = next(i for i, s in enumerate(plan["sections"]) if s["claims"])
            case(lambda v: v["sections"][section_index]["claims"].pop())
            case(lambda v: v["sections"][section_index]["claims"].append({"identity": "extra", "text": "추가"}))
            for i, section in enumerate(plan["sections"]):
                for j, claim in enumerate(section["claims"]):
                    if claim["protected_terms"]:
                        case(lambda v, i=i, j=j: v["sections"][i]["claims"][j].update(text="보호된 경로가 누락됨"))
                        protected_tested = True
                        break
                if protected_tested:
                    break
            for value in invalid:
                with self.assertRaises(c.CampaignError):
                    r.preview(binary, runtime, {**r.request(preparation, "markdown"), "realization": value})
        self.assertTrue(protected_tested)
        after_path = self.parent / "after.bundle.json"
        c.default_export(binary, runtime, repository, after_path)
        self.assertEqual(h.load_canonical_bundle(after_path).tables, before.tables)


def check_document_realization_regressions():
    result = unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(DocumentRealizationTests))
    if not result.wasSuccessful():
        raise AssertionError("document realization regressions failed")


if __name__ == "__main__":
    unittest.main()
