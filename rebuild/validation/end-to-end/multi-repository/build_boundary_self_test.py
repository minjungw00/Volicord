#!/usr/bin/env python3
"""Disposable contamination controls through the maintained gate and installer.

Synthetic support stops before Final; this never runs the real aggregate/providers.
"""
from __future__ import annotations
import json
from pathlib import Path
import shutil
import tempfile

import gate_entrypoint_self_test as entry

ROOT = entry.ROOT


def run_checked(argv, cwd, env):
    result = entry.run(tuple(map(str, argv)), cwd=cwd, env=env)
    assert result.returncode == 0, (result.args, result.returncode, result.stdout, result.stderr)
    return result


def controls(parent):
    candidate, env, final_marker, v11_marker = entry.make_candidate(parent)
    shutil.copy2(ROOT / 'rebuild/install.sh', candidate / 'rebuild/install.sh')
    (parent / 'bin/cargo').unlink()  # These controls use the real installed toolchain.
    workspace = candidate / 'rebuild'
    (workspace / 'Cargo.toml').write_text('[workspace]\nmembers=["crates/control"]\nexclude=["crates/dependency"]\nresolver="2"\n')
    (workspace / 'Cargo.lock').unlink()
    crate = workspace / 'crates/control'
    (crate / 'src').mkdir(parents=True)
    (crate / 'Cargo.toml').write_text('''[package]
name="build-control"
version="0.1.0"
edition="2021"
[dependencies]
control-dependency={path="../dependency"}
[[bin]]
name="volicord"
path="src/main.rs"
[[bin]]
name="volicord-mcp"
path="src/mcp.rs"
[[bin]]
name="volicord-viewer"
path="src/viewer.rs"
''')
    dependency = workspace/'crates/dependency'
    (dependency/'src').mkdir(parents=True)
    (dependency/'Cargo.toml').write_text('[package]\nname="control-dependency"\nversion="0.1.0"\nedition="2021"\n')
    (dependency/'src/lib.rs').write_text('pub fn number() -> u8 { 7 }\n')
    (dependency/'build.rs').write_text('use std::{env, process::Command};\nfn main() -> Result<(), Box<dyn std::error::Error>> {\nlet rustc = env::var("RUSTC")?;\nlet mut command = if let Ok(wrapper) = env::var("RUSTC_WRAPPER") { let mut c = Command::new(wrapper); c.arg(&rustc); c } else { Command::new(&rustc) };\nif !command.arg("--version").status()?.success() { return Err("compiler probe failed".into()); }\nOk(())\n}\n')
    (crate/'tests').mkdir()
    (crate/'tests/identity.rs').write_text('#[test]\nfn integration_target() { assert!(std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).is_absolute()); }\n')
    foreign = parent/'foreign'
    (foreign/'src').mkdir(parents=True)
    (foreign/'Cargo.toml').write_text('[package]\nname="foreign-diagnostic"\nversion="0.1.0"\nedition="2021"\n[workspace]\n')
    (foreign/'src/main.rs').write_text('fn main() { println!("diagnostic control"); }\n')
    source = crate / 'src/main.rs'
    source.write_text('fn main() { println!("normal build control"); }\n#[test]\nfn normal() { assert_eq!(2 + 3, 5); }\n')
    (crate/'src/mcp.rs').write_text(source.read_text())
    (crate/'src/viewer.rs').write_text(source.read_text())
    run_checked(['cargo', 'generate-lockfile', '--manifest-path', workspace/'Cargo.toml'], candidate, env)
    prefix = parent / 'installed'
    support = candidate / 'rebuild/scripts/check-fixture-manifest'
    entry.write_executable(support, '#!/usr/bin/env python3\nimport subprocess, os, json\nfrom pathlib import Path\n'
        + f'subprocess.run([{str(candidate / "rebuild/install.sh")!r}, "--prefix", {str(prefix)!r}, "--runtime-dir", {str(parent/"runtime")!r}], check=True)\n'
        + 'subprocess.run(["cargo", "test", "--manifest-path", "rebuild/Cargo.toml"], check=True)\n'
        + 'subprocess.run(["cargo", "clippy", "--manifest-path", "rebuild/Cargo.toml", "--all-targets", "--", "-D", "warnings"], check=True)\n'
        + f'out = Path({str(parent)!r})\n'
        + 'rows = []\n'
        + f'cases = [("late-flags", ["cargo", "build", "--manifest-path", "rebuild/Cargo.toml"], {{**os.environ, "RUSTFLAGS": "--cfg diagnostic_control"}}), ("foreign-source", ["cargo", "build", "--manifest-path", {str(foreign/"Cargo.toml")!r}], os.environ)]\n'
        + 'poison = Path(os.environ["CARGO_TARGET_DIR"])/"debug/uncompiled-executable"\n'
        + f'poison.write_text("#!/bin/sh\\ntouch {str(parent/"uncompiled-executed")}\\nexit 99\\n")\n'
        + 'poison.chmod(0o755)\n'
        + 'cases.append(("uncompiled-executable", [os.environ["RUSTC_WRAPPER"], "execute", str(poison)], os.environ))\n'
        + 'for label, argv, env in cases:\n'
        + '    result = subprocess.run(argv, env=env, capture_output=True, text=True)\n'
        + '    (out/(label+".stdout.log")).write_text(result.stdout)\n'
        + '    (out/(label+".stderr.log")).write_text(result.stderr)\n'
        + '    rows.append({"control": label, "argv": argv, "exit_code": result.returncode})\n'
        + '    assert result.returncode != 0 and "build boundary refused:" in result.stderr, (label, result.stderr)\n'
        + '(out/"compiler-controls.json").write_text(json.dumps(rows, indent=2)+"\\n")\n')
    run_checked(['git', 'add', '.'], candidate, env)
    run_checked(['git', '-c', 'user.name=Build Control', '-c', 'user.email=build@example.invalid',
                 'commit', '-qm', 'build controls'], candidate, env)
    # Establish misleading artifacts independently before invoking the boundary.
    poison = workspace / 'target/release'
    poison.mkdir(parents=True)
    sentinel = parent / 'poison-executed'
    for name in ('volicord', 'volicord-mcp', 'volicord-viewer'):
        entry.write_executable(poison/name, f'#!/bin/sh\ntouch "{sentinel}"\nexit 99\n')
    old = {p.name: p.read_bytes() for p in poison.iterdir()}
    # This ignored diagnostic target must remain untouched throughout all controls.
    (candidate / '.git/info/exclude').write_text('/rebuild/target/\n/rebuild/.cargo/\n')
    arguments = ('gate', '--external-network', 'available', '--authorize-external-transmission',
                 entry.AUTHORIZATION_ASSERTION, '--authorize-provider-source-transmission',
                 entry.PROVIDER_AUTHORIZATION_ASSERTION, '--provider-model', 'synthetic-model')
    outcomes = []

    def refused(label, changes=None):
        result = entry.invoke(candidate, env | (changes or {}), *arguments)
        assert result.returncode == 2 and 'qualification build refused:' in result.stderr, (label, result.stdout, result.stderr)
        assert not prefix.exists() and not sentinel.exists()
        assert not final_marker.exists() and not v11_marker.exists()
        outcomes.append({'control': label, 'exit_code': result.returncode, 'stderr': result.stderr})

    refused('shared diagnostic target', {'CARGO_TARGET_DIR': str(poison.parent)})
    refused('compiler flags', {'RUSTFLAGS': '--cfg diagnostic_control'})
    refused('encoded flags', {'CARGO_ENCODED_RUSTFLAGS': '--cfg\x1fdiagnostic_control'})
    refused('compiler cache wrapper', {'RUSTC_WRAPPER': '/unavailable/diagnostic-wrapper'})
    refused('workspace wrapper', {'RUSTC_WORKSPACE_WRAPPER': '/unavailable/diagnostic-wrapper'})
    refused('target runner', {'CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER': '/unavailable/diagnostic-runner'})
    refused('new build-dir setting', {'CARGO_BUILD_BUILD_DIR': str(poison.parent)})
    configs = workspace / '.cargo'
    configs.mkdir()
    config = configs/'config.toml'
    config.write_text('[build]\ntarget-dir="../target"\nrustflags=["--cfg", "diagnostic_control"]\n')
    refused('ignored Cargo configuration')
    config.unlink()
    cache_home = parent/'diagnostic-cargo-home'
    cache_home.mkdir()
    (cache_home/'config.toml').write_text('[build]\nrustc-wrapper="/unavailable/diagnostic-wrapper"\n')
    refused('shared Cargo home configuration', {'CARGO_HOME': str(cache_home)})
    original = source.read_text()
    source.write_text(original + '// isolated diagnostic input\n')
    run_checked(['git', 'update-index', '--assume-unchanged', str(source.relative_to(candidate))], candidate, env)
    refused('hidden instrumented source')
    source.write_text(original)
    run_checked(['git', 'update-index', '--no-assume-unchanged', str(source.relative_to(candidate))], candidate, env)
    ignored_source = crate/'src/hidden.rs'
    (candidate/'.git/info/exclude').open('a').write('/rebuild/crates/control/src/hidden.rs\n')
    ignored_source.write_text('diagnostic source\n')
    refused('ignored compiler input')
    ignored_source.unlink()

    # Invoke the actual gate; its ordinary support calls run the actual installer
    # and Cargo test in the gate-owned target. An intentional support exit 37
    # prevents all real aggregate/external stages.
    result = entry.invoke(candidate, env | {'DOGFOOD_ASSERTIONS_FAIL': '1'}, *arguments)
    assert result.returncode == 1, (result.stdout, result.stderr)
    capsule = entry.structured_stdout(result)
    assert capsule['final_aggregate']['status'] == 'not_run'
    path = Path(next(line.removeprefix('evidence capsule: ') for line in result.stderr.splitlines()
                     if line.startswith('evidence capsule: '))).parent
    admission = json.loads((path/'admission.json').read_text())
    fixture = next(c for c in admission['checks'] if c['name'] == 'fixture_manifest_integrity')
    assert fixture['status'] == 'passed', fixture
    context = json.loads((path/'build/context.json').read_text())
    assert context['initial_target_empty'] is True
    records = [json.loads(p.read_text()) for p in (path/'build/receipts').glob('*.json')]
    executions = [r for r in records if r['kind'] == 'execution_started']
    installed = [r for r in records if r['kind'] == 'installed_executables']
    assert executions and installed
    for record in executions:
        assert Path(record['binary']['path']).is_relative_to(path/'build/target')
        assert record['compiler_receipts']
    assert all((prefix/'bin'/name).read_bytes() != content for name, content in old.items())
    assert all((poison/name).read_bytes() == content for name, content in old.items())
    assert not sentinel.exists() and not final_marker.exists() and not v11_marker.exists()
    assert not (parent/'uncompiled-executed').exists()
    # Installer must also honor Cargo's selected artifact outside qualification.
    external_target = parent/'normal-custom-target'
    direct = entry.invoke(candidate, env | {'CARGO_TARGET_DIR': str(external_target)}, 'focused',
                          'custom-target-install', '--', str(candidate/'rebuild/install.sh'),
                          '--prefix', str(parent/'direct-install'), '--runtime-dir', str(parent/'direct-runtime'))
    assert direct.returncode == 0, (direct.stdout, direct.stderr)
    assert (external_target/'release/volicord').read_bytes() == (parent/'direct-install/bin/volicord').read_bytes()
    summary = {'controls': outcomes, 'isolated_gate_exit': result.returncode,
               'intentional_support_exit': 37, 'gate_directory': str(path),
               'custom_target_install_exit': direct.returncode, 'execution_receipts': len(executions),
               'old_target_preserved': True, 'compiler_controls': json.loads((parent/'compiler-controls.json').read_text()),
               'final_and_provider': 'not_run'}
    (parent/'controls.json').write_text(json.dumps(summary, indent=2)+'\n')
    return summary


def main():
    local = ROOT/'rebuild/.local/validation'
    local.mkdir(parents=True, exist_ok=True)
    parent = Path(tempfile.mkdtemp(prefix='build-isolation-controls-', dir=local))
    summary = controls(parent)
    print(json.dumps(summary, indent=2))
    print(f'build isolation controls passed; artifacts: {parent}')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
