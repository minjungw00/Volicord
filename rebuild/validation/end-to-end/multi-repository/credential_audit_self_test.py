"""Synthetic credential retention controls; diagnostic evidence never includes values."""
import json
import os
from pathlib import Path
import tempfile


def self_check(h):
    with tempfile.TemporaryDirectory(prefix="volicord-v11-credential-") as directory:
        root = Path(directory)
        values = {
            "OPENAI_API_KEY": "synthetic-api-value-only",
            "tokens": {"access_token": "synthetic-access-value-only",
                       "refresh_token": "synthetic-refresh-value-only",
                       "id_token": "synthetic-id-value-only"},
            "provider": {"client_secret": "synthetic-quote-\"-newline-\n-unicode-한글"},
        }
        auth = root / "source-auth.json"
        auth.write_text(json.dumps(values, indent=2))
        retained = root / "retained"
        retained.mkdir()
        output = retained / "output.log"
        secrets = [values["OPENAI_API_KEY"], *values["tokens"].values(), values["provider"]["client_secret"]]

        def check(expected, content):
            output.write_bytes(content)
            result = h.credential_retention_audit(retained, auth)
            assert result["status"] == expected
            diagnostic = json.dumps(result)
            assert len(diagnostic) < 512
            assert all(secret not in diagnostic for secret in secrets)
            assert not any("hash" in key or "path" in key for key in result)
            return result

        check("passed", b'{"status":"passed","account_id":"public-account"}\n')
        assert check("failed", auth.read_bytes())["whole_auth_file_match_count"] == 1
        for secret in secrets:
            assert check("failed", secret.encode())["credential_value_match_count"] == 1
            for ascii_only in (True, False):
                assert check("failed", json.dumps({"other_field": secret}, ensure_ascii=ascii_only).encode())["credential_value_match_count"] == 1
        check("failed", json.dumps(values, separators=(",", ":"), sort_keys=True).encode())
        check("failed", json.dumps(values, indent=4, ensure_ascii=False).encode())
        # A known value split across stream chunks must still match.
        check("failed", b"x" * (65536 - 9) + secrets[1].encode() + b"y" * 65536)
        check("passed", b"clean")
        (retained / "auth.json").write_text("{}")
        assert h.credential_retention_audit(retained, auth)["auth_named_file_count"] == 1
        (retained / "auth.json").unlink()
        # Links are errors; the audit must not read outside its artifact boundary.
        (retained / "external-link").symlink_to(auth)
        linked = h.credential_retention_audit(retained, auth)
        assert linked["status"] == "failed" and linked["credential_content_match_count"] == 0
        (retained / "external-link").unlink()
        for bad in ("not-json", "[]"):
            auth.write_text(bad)
            result = h.credential_retention_audit(retained, auth)
            assert result["status"] == "failed" and result["scan_error_count"] == 1
        auth.unlink()
        assert h.credential_retention_audit(retained, auth)["scan_error_count"] == 1

        # Audit the actual staged material, including a token refreshed by Codex,
        # even when the original auth file changes or disappears afterwards.
        auth.write_text(json.dumps(values))
        registered = root / "registered"
        registered.mkdir()
        (registered / "config.toml").write_text("# synthetic config\n")
        material = {"needles": set(), "errors": 0}
        staging = root / "staging"
        refreshed = "synthetic-refreshed-secret-only"
        with h.staged_codex_authentication(auth, registered, retained,
                                           staging_parent=staging, credential_material=material) as home:
            (home / "auth.json").write_text(json.dumps({"tokens": {"refresh_token": refreshed}}))
        assert not any(staging.iterdir())
        auth.unlink()
        for secret in (secrets[1], refreshed):
            output.write_text(secret)
            result = h.credential_retention_audit(retained, known_material=material)
            assert result["status"] == "failed" and result["credential_value_match_count"] == 1
            assert secret not in json.dumps(result)
        output.write_text("clean")
        assert h.credential_retention_audit(retained, known_material=material)["status"] == "passed"

        # The authenticated probe consumes this audit rather than accepting a
        # successful child exit that leaked its refreshed authentication value.
        auth.write_text(json.dumps(values))
        repository = root / "repository"
        repository.mkdir()
        fake = root / "fake-codex"
        fake.write_text("#!/usr/bin/env python3\nimport json,os; from pathlib import Path\nauth=Path(os.environ['CODEX_HOME'])/'auth.json'\nauth.write_text(json.dumps({'tokens':{'refresh_token':" + repr(refreshed) + "}}))\nprint(" + repr(refreshed) + ")\n")
        fake.chmod(0o700)
        result = h.authenticated_codex(h.Recorder(retained), str(fake),
            dict(os.environ) | {"CODEX_HOME": str(registered)}, repository,
            "synthetic-project", retained, model="synthetic-model",
            staging_parent=staging, authentication_source=auth)
        assert result["status"] == "failed"
        assert result["evidence"]["credential_audit"]["credential_value_match_count"] == 1
        assert refreshed not in json.dumps(result)
        assert not any(staging.iterdir())
