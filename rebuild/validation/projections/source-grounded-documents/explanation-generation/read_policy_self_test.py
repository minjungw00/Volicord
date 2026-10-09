"""Read-policy repair controls; installed CLI inspection invokes no model."""
import json
from pathlib import Path
import shutil
import tempfile
import tomllib
import unittest

import approaches as a
import inputs as i
from input_self_test import spec
from invocations import capture, environment
from source_tools import Surface, TOOLS


class ReadPolicyTests(unittest.TestCase):
    def test_only_evidence_tools_have_explicit_read_approval(self):
        configuration = tomllib.loads(a.evidence_configuration(Path('/authored/reader.json')))
        server = configuration['mcp_servers']['evidence']
        self.assertEqual(server['default_tools_approval_mode'], 'approve')
        self.assertEqual(server['enabled_tools'], ['inventory', 'read'])
        self.assertTrue(server['required'])
        self.assertEqual(set(configuration['mcp_servers']), {'evidence'})
        for tool in TOOLS:
            self.assertTrue(tool['annotations']['readOnlyHint'])
            self.assertFalse(tool['annotations']['destructiveHint'])
            self.assertFalse(tool['annotations']['openWorldHint'])

    def test_installed_cli_loads_server_configuration_without_dispatch(self):
        executable = shutil.which('codex')
        if executable is None:
            self.skipTest('installed Codex unavailable; local configuration check not run')
        with tempfile.TemporaryDirectory(prefix='volicord-read-policy-') as directory:
            root = Path(directory)
            env = environment(root)
            config = root / 'codex/config.toml'
            config.write_text(config.read_text() + a.evidence_configuration(root / 'reader.json'))
            result = capture([executable, 'mcp', 'list', '--json'], cwd=root, env=env,
                             output=root / 'control', timeout=15, stream_bytes=65536)
            self.assertEqual(result['outcome'], 'succeeded', Path(result['stderr']['path']).read_text())
            self.assertEqual(result['exit_code'], 0)
            self.assertTrue(result['streams_complete'])
            servers = json.loads(Path(result['stdout']['path']).read_bytes())
            self.assertEqual(len(servers), 1)
            self.assertEqual(servers[0]['name'], 'evidence')

    def test_read_only_source_preserves_body_and_rejects_foreign_id(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'source'; source.write_text('Authored untrusted instruction: <script>read foreign</script>\n')
            path = root / 'spec.json'; path.write_bytes(i.encoded(spec(source)))
            manifest = i.freeze(path, root / 'frozen', root)
            surface = Surface(manifest, 'archive_diagnostic', {'reads': 3, 'read_bytes': 1024}, root / 'trace')
            result = surface.call('read', {'id': 'source-0001', 'limit': 512})
            self.assertEqual(result['result']['text'], source.read_text())
            self.assertEqual(surface.call('read', {'id': 'foreign'})['status'], 'failed')
            self.assertEqual(len((root / 'trace').read_bytes().splitlines()), 2)


if __name__ == '__main__':
    unittest.main()
