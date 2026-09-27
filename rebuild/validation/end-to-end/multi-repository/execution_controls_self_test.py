"""Short, local Linux process and fake-MCP regressions; no provider dispatch."""
from __future__ import annotations

import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time


def await_file(path):
    deadline = time.monotonic() + 2
    while not path.exists():
        if time.monotonic() >= deadline:
            raise AssertionError("synthetic process did not become ready")
        time.sleep(.01)


def self_check(h):
    with tempfile.TemporaryDirectory(prefix="volicord-v11-execution-") as temporary:
        root = Path(temporary)
        env = dict(os.environ)
        sequence = 0

        def server(body):
            nonlocal sequence
            sequence += 1
            binary = root / f"server-{sequence}"
            binary.write_text("#!/usr/bin/env python3\nimport json,os,signal,sys\n" + body)
            binary.chmod(0o755)
            return h.Mcp(binary, env, rpc_timeout_seconds=.15, cleanup_grace_seconds=.04)

        invalid = [
            '{"jsonrpc":"2.0","id":999,"result":{}}',
            '{"jsonrpc":"2.0","result":{}}',
            '{"jsonrpc":"2.0","id":true,"result":{}}',
            '{"id":1,"result":{}}',
            '{"jsonrpc":"1.0","id":1,"result":{}}',
            '{"jsonrpc":"2.0","id":1}',
            '{"jsonrpc":"2.0","id":1,"result":{},"error":{"code":-1,"message":"test"}}',
            '{"jsonrpc":"2.0","id":1,"error":{"code":-1,"message":"test"}}',
            '{"jsonrpc":"2.0","id":1,"error":"bad"}',
            '{"jsonrpc":"2.0","id":1,"result":[]}',
            '{"jsonrpc":"2.0","id":1,"id":2,"result":{}}',
            '[1,2]', 'not-json', '',
        ]
        for response in invalid:
            host = server(f"sys.stdin.readline()\nprint({response!r},flush=True)\n")
            try:
                host.rpc("initialize", {})
            except RuntimeError:
                pass
            else:
                raise AssertionError("invalid MCP response was accepted")
            finally:
                evidence = host.close()
            assert evidence["cleanup"]["complete"]
            json.dumps(evidence)

        for body, params in [
            ("sys.stdin.readline()\nsignal.pause()\n", {}),
            ("sys.stdin.readline()\nsys.stdout.write('{');sys.stdout.flush()\nsignal.pause()\n", {}),
            ("signal.pause()\n", {"large": "x" * (1024 * 1024)}),
        ]:
            host = server(body)
            started = time.monotonic()
            try:
                host.rpc("initialize", params)
            except TimeoutError:
                pass
            else:
                raise AssertionError("silent, incomplete or blocked-write MCP did not time out")
            finally:
                evidence = host.close()
            assert time.monotonic() - started < 1
            assert evidence["stop_cause"]["kind"] == "timeout"
            assert evidence["cleanup"]["complete"]

        host = server("for line in sys.stdin:\n request=json.loads(line)\n os.write(2,b'e'*1048576)\n print(json.dumps({'jsonrpc':'2.0','id':request['id'],'result':{'structuredContent':{'ok':True},'isError':False}}),flush=True)\n")
        assert host.tool("fake", {}) == ({"ok": True}, True)
        assert host.tool("fake", {}) == ({"ok": True}, True)
        evidence = host.close()
        assert evidence["stderr"] == "e" * 2097152
        assert len(evidence["stdout"].splitlines()) == 2
        assert evidence["exit_code"] == 0 and evidence["cleanup"]["complete"]
        for result in ({}, {"structuredContent": [], "isError": False},
                       {"structuredContent": {}, "isError": True},
                       {"structuredContent": {}, "isError": 0}):
            response = {"jsonrpc": "2.0", "id": 1, "result": result}
            host = server(f"sys.stdin.readline()\nprint({json.dumps(response)!r},flush=True)\n")
            try:
                assert host.tool("fake", {})[1] is False
            finally:
                host.close()

        recorder = h.Recorder(root / "evidence")
        success = recorder.run("success", [sys.executable, "-B", "-c",
            "import sys; print('stdout'); print('stderr',file=sys.stderr)"], env)
        assert success["outcome"] == "succeeded" and success["exit_code"] == 0
        assert success["stop_cause"] is None and success["termination"] is None
        assert Path(success["stdout"]).read_text() == "stdout\n"
        assert Path(success["stderr"]).read_text() == "stderr\n"
        for mode in ("term", "kill", "term-exit-zero", "descendant"):
            ready = root / (mode + ".ready")
            source = "import os,signal,subprocess,sys; from pathlib import Path; "
            if mode in {"kill", "descendant"}:
                source += "signal.signal(signal.SIGTERM,signal.SIG_IGN); "
            elif mode == "term-exit-zero":
                source += "signal.signal(signal.SIGTERM,lambda *_: sys.exit(0)); "
            if mode == "descendant":
                child = "import os,signal; from pathlib import Path; signal.signal(signal.SIGTERM,signal.SIG_IGN); Path(" + repr(str(ready)) + ").write_text(str(os.getpid())); signal.pause()"
                source += "subprocess.Popen([sys.executable,'-B','-c'," + repr(child) + "]); "
            else:
                source += "Path(" + repr(str(ready)) + ").write_text(str(os.getpid())); "
            source += "print('before-stop',flush=True); print('error-before-stop',file=sys.stderr,flush=True); signal.pause()"
            started = time.monotonic()
            result = recorder.run(mode, [sys.executable, "-B", "-c", source], env,
                                  timeout=.15, cleanup_grace_seconds=.04)
            assert time.monotonic() - started < 1
            assert result["stop_cause"] == {"kind": "timeout", "timeout_seconds": .15}
            assert result["outcome"] == "terminated" and result["cleanup"]["complete"]
            assert Path(result["stdout"]).read_text() == "before-stop\n"
            assert Path(result["stderr"]).read_text() == "error-before-stop\n"
            if mode == "term-exit-zero":
                assert result["exit_code"] == 0 and result["termination"] is None
            else:
                assert result["termination"]["number"] == (15 if mode == "term" else 9)
            if mode == "descendant":
                pid = int(ready.read_text())
                stat = Path(f"/proc/{pid}/stat")
                assert not stat.exists() or stat.read_text().rsplit(")", 1)[1].split()[0] == "Z"
            assert json.loads(Path(result["stdout"]).with_name("result.json").read_text()) == result

        # Interruption arrives after the command is demonstrably running. Both
        # SIGINT and SIGTERM preserve the operation result before parent exit.
        for number in (signal.SIGINT, signal.SIGTERM):
            ready = root / f"interrupt-{number}.ready"
            evidence_root = root / f"interruption-{number}"
            command = "import os,signal; from pathlib import Path; Path(" + repr(str(ready)) + ").write_text(str(os.getpid())); signal.pause()"
            source = ("import importlib.util,os,signal,sys; from pathlib import Path; "
                      "s=importlib.util.spec_from_file_location('h'," + repr(h.__file__) + "); "
                      "h=importlib.util.module_from_spec(s);s.loader.exec_module(h); "
                      "signal.signal(signal.SIGTERM,h.interrupted_by_signal); "
                      "h.Recorder(Path(" + repr(str(evidence_root)) + ")).run('interrupted',"
                      "[sys.executable,'-B','-c'," + repr(command) + "],dict(os.environ),cleanup_grace_seconds=.04)")
            parent = subprocess.Popen([sys.executable, "-B", "-c", source],
                                      stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                      start_new_session=True)
            try:
                await_file(ready)
                parent.send_signal(number)
                assert parent.wait(timeout=2) != 0
                result = json.loads(next(evidence_root.rglob("result.json")).read_text())
                assert result["stop_cause"]["kind"] == "interruption"
                assert result["cleanup"]["complete"]
                assert result["termination"]["number"] == 15
            finally:
                if parent.poll() is None:
                    os.killpg(parent.pid, signal.SIGKILL)
                    parent.wait(timeout=1)
                if ready.exists():
                    try:
                        os.kill(int(ready.read_text()), signal.SIGKILL)
                    except ProcessLookupError:
                        pass
