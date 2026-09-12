#!/usr/bin/env python3
"""Transport failure and signed-payload regression checks (no remote writes)."""
import importlib.util
import io
import json
import subprocess
import sys
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('updater', Path(__file__).parents[1] / 'updater-release.py')
updater = importlib.util.module_from_spec(spec)
spec.loader.exec_module(updater)


class UpdaterReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.artifact = self.root / 'Tura Notes.deb'
        self.artifact.write_bytes(b'installer bytes')
        config = self.root / 'config.json'
        config.write_text(json.dumps({'plugins': {'updater': {'pubkey': 'test public key'}}}))
        self.config = patch.object(updater, 'CONFIG', config)
        self.config.start()
        self.addCleanup(self.config.stop)
        def sign(path):
            Path(str(path) + '.sig').write_text('test signature')
        with patch.object(updater, 'sign', side_effect=sign):
            updater.prepare(self.artifact, '1.1.0', 'linux-aarch64-deb')

    def test_completed_payload_is_reused_without_signing(self):
        with patch.object(updater, 'sign') as sign:
            updater.prepare(self.artifact, '1.1.0', 'linux-aarch64-deb')
        sign.assert_not_called()

    def test_modified_payload_signature_version_and_key_are_rejected(self):
        with self.assertRaises(ValueError):
            updater.verify(self.artifact, '1.1.1')
        signature = Path(str(self.artifact) + '.sig')
        signature.write_text('different signature')
        with self.assertRaises(ValueError):
            updater.verify(self.artifact, '1.1.0')
        signature.write_text('test signature')
        self.artifact.write_bytes(b'tampered')
        with self.assertRaises(ValueError):
            updater.verify(self.artifact, '1.1.0')
        self.artifact.write_bytes(b'installer bytes')
        updater.CONFIG.write_text(json.dumps({'plugins': {'updater': {'pubkey': 'wrong key'}}}))
        with self.assertRaises(ValueError):
            updater.verify(self.artifact, '1.1.0')

    def test_linux_reuse_checks_signed_sidecars(self):
        cache = Path(__file__).parents[1] / 'linux-build-cache.py'
        Path(str(self.artifact) + '.sha256').write_text(updater.sha(self.artifact) + '  package\n')
        identity = [str(self.root), '1.1.0', 'aarch64-unknown-linux-gnu', 'source-hash', '0']
        subprocess.run([sys.executable, str(cache), 'record', *identity, str(self.artifact)], check=True)
        check = [sys.executable, str(cache), 'check', *identity]
        self.assertEqual(subprocess.run(check, capture_output=True).returncode, 0)
        Path(str(self.artifact) + '.sig').unlink()
        self.assertNotEqual(subprocess.run(check, capture_output=True).returncode, 0)

    def publish(self, checksum=None):
        calls = []
        manifests = []
        def run(args, **kwargs):
            calls.append(args)
            if args[0] == 'scp':
                manifests.append(json.loads(Path(args[2]).read_text()))
        def output(args, **kwargs):
            if 'mktemp' in args[-1]:
                return '/tmp/tura-update.unique\n'
            return (checksum or updater.sha(self.artifact)) + '  payload\n'
        def public(url, **kwargs):
            if url.endswith('.json'):
                return io.BytesIO(json.dumps(manifests[0]).encode())
            return io.BytesIO(self.artifact.read_bytes())
        with patch.object(updater.subprocess, 'run', side_effect=run), \
             patch.object(updater.subprocess, 'check_output', side_effect=output), \
             patch.object(updater, 'urlopen', side_effect=public):
            try:
                updater.publish(self.artifact, '1.1.0', 'b3sys@100.64.100.242', '/srv/app', '/tmp', 'https://example.com')
            finally:
                self.calls = calls
        return calls, manifests

    def test_payload_installed_before_atomic_platform_feed(self):
        calls, manifests = self.publish()
        command = next(c[-1] for c in calls if 'sudo -u www-data install' in c[-1])
        self.assertLess(command.index('Tura Notes.deb'), command.index('linux-aarch64-deb.json'))
        self.assertIn('linux-aarch64-deb.json.tura-update.unique.tmp', command)
        self.assertIn('mv -f', command)
        self.assertIn('linux-aarch64-deb', manifests[0]['url'])
        self.assertEqual(calls[0][0], 'scp')
        self.assertTrue(calls[0][-1].startswith('b3sys@100.64.100.242:'))

    def test_bad_upload_never_changes_feed_and_staging_is_cleaned(self):
        with self.assertRaisesRegex(ValueError, 'checksum mismatch'):
            self.publish('wrong checksum')
        self.assertFalse(any('sudo -u' in c[-1] for c in self.calls))
        self.assertIn('rm -rf -- /tmp/tura-update.unique', self.calls[-1][-1])


if __name__ == '__main__':
    unittest.main()
