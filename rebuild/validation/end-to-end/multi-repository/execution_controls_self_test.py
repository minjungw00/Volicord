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


READINESS_TIMEOUT_SECONDS = 2
RPC_TIMEOUT_SECONDS = .15
COOPERATIVE_EXIT_GRACE_SECONDS = 1
FORCED_CLEANUP_GRACE_SECONDS = .04


def await_file(path, *, process=None, timeout=READINESS_TIMEOUT_SECONDS):
    deadline = time.monotonic() + timeout
    while not path.exists():
        if process is not None and process.poll() is not None:
            raise AssertionError("synthetic process exited before readiness")
        if time.monotonic() >= deadline:
            raise AssertionError("synthetic process did not become ready")
        time.sleep(.01)


def self_check(h):
    with tempfile.TemporaryDirectory(prefix="volicord-v11-execution-") as temporary:
        root = Path(temporary)
        env = dict(os.environ)
        sequence = 0

        def server(body, *, startup="", readiness_timeout=READINESS_TIMEOUT_SECONDS,
                   cleanup_grace_seconds=COOPERATIVE_EXIT_GRACE_SECONDS):
            nonlocal sequence
            sequence += 1
            binary = root / f"server-{sequence}"
            ready = root / f"server-{sequence}.ready"
            # Signal after interpreter/import/startup work, immediately before the
            # request behavior. Keep readiness off the MCP stdout/stderr streams.
            binary.write_text("#!/usr/bin/env python3\nimport json,os,signal,sys,time\n"
                              "from pathlib import Path\n" + startup
                              + f"Path({str(ready)!r}).touch()\n" + body)
            binary.chmod(0o755)
            # Normal/protocol fixtures need time for cooperative interpreter exit.
            # Readiness-failure and timeout fixtures explicitly opt into fast cleanup.
            host = h.Mcp(binary, env, rpc_timeout_seconds=RPC_TIMEOUT_SECONDS,
                         cleanup_grace_seconds=cleanup_grace_seconds)
            try:
                await_file(ready, process=host.process, timeout=readiness_timeout)
            except BaseException as error:
                evidence = host.close()
                if isinstance(error, AssertionError):
                    raise AssertionError(
                        f"synthetic MCP readiness failed: {error}; "
                        f"exit_code={evidence['exit_code']}; "
                        f"cleanup_complete={evidence['cleanup']['complete']}; "
                        f"stderr={evidence['stderr']!r}") from error
                raise
            return host

        # EOF acknowledges close(), so the injected teardown starts after the
        # parent receives the response and closes stdin, regardless of scheduling.
        # It exceeds forced cleanup's 40 ms but fits a cooperative one-second grace.
        teardown = root / "cooperative-exit.json"
        host = server("request=json.loads(sys.stdin.readline())\n"
                      "print(json.dumps({'jsonrpc':'2.0','id':request['id'],'result':{}}),flush=True)\n"
                      "assert sys.stdin.read() == ''\n"
                      "started=time.monotonic()\ntime.sleep(.2)\n"
                      f"Path({str(teardown)!r}).write_text(json.dumps({{'duration_seconds':time.monotonic()-started}}))\n")
        try:
            assert host.rpc("initialize", {})["result"] == {}
            assert host.process.poll() is None
        finally:
            evidence = host.close()
        assert evidence["exit_code"] == 0, evidence
        assert evidence["termination"] is None and evidence["stop_cause"] is None
        assert evidence["cleanup"]["complete"] and evidence["cleanup"]["signals_sent"] == []
        assert FORCED_CLEANUP_GRACE_SECONDS < .2 < COOPERATIVE_EXIT_GRACE_SECONDS
        assert .2 <= json.loads(teardown.read_text())["duration_seconds"] < COOPERATIVE_EXIT_GRACE_SECONDS
        assert len(evidence["stdout"].splitlines()) == 1 and evidence["stderr"] == ""
        json.dumps(evidence)

        # This delay is the injected regression condition, not a race workaround:
        # readiness takes longer than the unchanged short RPC budget.
        started = time.monotonic()
        host = server("request=json.loads(sys.stdin.readline())\n"
                      "print(json.dumps({'jsonrpc':'2.0','id':request['id'],'result':{}}),flush=True)\n",
                      startup="time.sleep(.3)\n")
        try:
            assert time.monotonic() - started >= .3 > host.rpc_timeout_seconds
            assert host.rpc("initialize", {})["result"] == {}
        finally:
            evidence = host.close()
        assert evidence["exit_code"] == 0 and evidence["cleanup"]["complete"]
        assert evidence["stop_cause"] is None and evidence["termination"] is None
        assert evidence["cleanup"]["signals_sent"] == []

        # Crashed and never-ready fixtures fail at readiness and clean up even
        # though no RPC has been attempted.
        for startup, timeout, diagnostic in [
            ("print('startup-crash',file=sys.stderr,flush=True)\nsys.exit(23)\n",
             2, "exited before readiness"),
            ("signal.pause()\n", .15, "did not become ready"),
        ]:
            active = set(h.Mcp.active)
            started = time.monotonic()
            try:
                server("", startup=startup, readiness_timeout=timeout,
                       cleanup_grace_seconds=FORCED_CLEANUP_GRACE_SECONDS)
            except AssertionError as error:
                assert "synthetic MCP readiness failed" in str(error)
                assert diagnostic in str(error) and "cleanup_complete=True" in str(error)
                if diagnostic == "exited before readiness":
                    assert "exit_code=23" in str(error) and "startup-crash" in str(error)
            else:
                raise AssertionError("unready MCP fixture was accepted")
            assert time.monotonic() - started < timeout + 1
            assert h.Mcp.active == active

        identity_error = "MCP response identity or result/error framing is invalid"
        invalid = [
            ('{"jsonrpc":"2.0","id":999,"result":{}}', identity_error),
            ('{"jsonrpc":"2.0","result":{}}', identity_error),
            ('{"jsonrpc":"2.0","id":true,"result":{}}', identity_error),
            ('{"id":1,"result":{}}', identity_error),
            ('{"jsonrpc":"1.0","id":1,"result":{}}', identity_error),
            ('{"jsonrpc":"2.0","id":1}', identity_error),
            ('{"jsonrpc":"2.0","id":1,"result":{},"error":{"code":-1,"message":"test"}}', identity_error),
            ('{"jsonrpc":"2.0","id":1,"error":{"code":-1,"message":"test"}}',
             "MCP returned an explicit RPC error"),
            ('{"jsonrpc":"2.0","id":1,"error":"bad"}', "MCP RPC error framing is invalid"),
            ('{"jsonrpc":"2.0","id":1,"result":[]}', "MCP result shape is invalid"),
            ('{"jsonrpc":"2.0","id":1,"id":2,"result":{}}', "MCP response is malformed JSON"),
            ('[1,2]', identity_error),
            ('not-json', "MCP response is malformed JSON"),
            ('', "MCP response is malformed JSON"),
        ]
        for response, diagnostic in invalid:
            host = server(f"sys.stdin.readline()\nprint({response!r},flush=True)\n")
            try:
                host.rpc("initialize", {})
            except RuntimeError as error:
                assert str(error) == diagnostic
            else:
                raise AssertionError("invalid MCP response was accepted")
            finally:
                evidence = host.close()
            assert evidence["stop_cause"]["kind"] == "rpc_failure"
            assert evidence["exit_code"] == 0 and evidence["termination"] is None
            assert evidence["cleanup"]["signals_sent"] == []
            assert evidence["cleanup"]["complete"]
            json.dumps(evidence)

        for body, params in [
            ("sys.stdin.readline()\nsignal.pause()\n", {}),
            ("sys.stdin.readline()\nsys.stdout.write('{');sys.stdout.flush()\nsignal.pause()\n", {}),
            ("signal.pause()\n", {"large": "x" * (1024 * 1024)}),
        ]:
            host = server(body, cleanup_grace_seconds=FORCED_CLEANUP_GRACE_SECONDS)
            started = time.monotonic()
            try:
                host.rpc("initialize", params)
            except TimeoutError as error:
                assert str(error) == "MCP RPC deadline expired"
            else:
                raise AssertionError("silent, incomplete or blocked-write MCP did not time out")
            finally:
                evidence = host.close()
            assert host.rpc_timeout_seconds == RPC_TIMEOUT_SECONDS == .15
            assert .15 <= time.monotonic() - started < 2
            assert evidence["stdout"] == ("{" if "sys.stdout.write" in body else "")
            assert evidence["stop_cause"]["kind"] == "timeout"
            assert host.cleanup_grace_seconds == FORCED_CLEANUP_GRACE_SECONDS
            assert evidence["termination"] == {"kind": "signal", "number": 15}
            assert evidence["cleanup"]["signals_sent"] == [15]
            assert evidence["cleanup"]["complete"]

        host = server("for line in sys.stdin:\n request=json.loads(line)\n os.write(2,b'e'*1048576)\n print(json.dumps({'jsonrpc':'2.0','id':request['id'],'result':{'structuredContent':{'ok':True},'isError':False}}),flush=True)\n")
        assert host.tool("fake", {}) == ({"ok": True}, True)
        assert host.tool("fake", {}) == ({"ok": True}, True)
        evidence = host.close()
        assert evidence["stderr"] == "e" * 2097152
        assert evidence["termination"] is None and evidence["stop_cause"] is None
        assert evidence["cleanup"]["signals_sent"] == []
        json.dumps(evidence)
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
                evidence = host.close()
            assert evidence["exit_code"] == 0 and evidence["termination"] is None
            assert evidence["cleanup"]["complete"] and evidence["cleanup"]["signals_sent"] == []
            json.dumps(evidence)

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
                                  timeout=.15, cleanup_grace_seconds=FORCED_CLEANUP_GRACE_SECONDS)
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
                      "[sys.executable,'-B','-c'," + repr(command)
                      + f"],dict(os.environ),cleanup_grace_seconds={FORCED_CLEANUP_GRACE_SECONDS!r})")
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
