#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Guard the non-publishing Scorecard workflow contract without dependencies."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
IMAGE = ('docker://ghcr.io/ossf/scorecard-action@sha256:'
         '1e1481c65fc2407d3fd6ab18a23c741eb4d1c0d9359ebafc4b3ab13aa61d8627')


class ScorecardPolicy(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        raw = (ROOT / '.github/workflows/scorecard.yml').read_text()
        cls.workflow = '\n'.join(line.split(' #', 1)[0] for line in raw.splitlines()
                                 if not line.lstrip().startswith('#'))

    def step(self, name):
        match = re.search(r'^      - name: ' + re.escape(name) +
                          r'\n(.*?)(?=^      - |\Z)', self.workflow, re.M | re.S)
        self.assertIsNotNone(match, name)
        return match.group(1)

    def test_immutable_execution(self):
        step = self.step('Run analysis')
        self.assertIn('        uses: ' + IMAGE, step)
        self.assertEqual(self.workflow.count('docker://'), 1)
        self.assertNotIn('uses: ossf/scorecard-action@', self.workflow)

    def test_explicit_upstream_inputs(self):
        step = self.step('Run analysis')
        for value in ('INPUT_RESULTS_FILE: results.sarif',
                      'INPUT_RESULTS_FORMAT: sarif', 'INPUT_FILE_MODE: archive',
                      'INPUT_PUBLISH_RESULTS: "false"',
                      'INPUT_REPO_TOKEN: ${{ github.token }}'):
            self.assertIn('          ' + value + '\n', step + '\n')
        self.assertNotIn('secrets.', step)

    def test_no_public_publisher_credentials(self):
        self.assertNotIn('id-token:', self.workflow)
        self.assertNotIn('read-all', self.workflow)
        self.assertNotIn('write-all', self.workflow)
        self.assertRegex(self.workflow, r'(?m)^permissions:\n  contents: read$')
        self.assertRegex(self.workflow, r'(?m)^      security-events: write$')

    def test_safe_validation_triggers(self):
        for event in ('pull_request', 'push', 'schedule', 'branch_protection_rule'):
            self.assertRegex(self.workflow, r'(?m)^  ' + event + ':')
        self.assertNotIn('pull_request_target:', self.workflow)
        self.assertIn('          persist-credentials: false', self.step('Checkout'))
        self.assertIn('python3 scripts/test-scorecard-policy.py', self.workflow)

    def test_artifact_is_required(self):
        step = self.step('Upload the results as an artifact')
        self.assertRegex(step, r'uses: actions/upload-artifact@[0-9a-f]{40}')
        for value in ('path: results.sarif', 'retention-days: 5',
                      'if-no-files-found: error'):
            self.assertIn(value, step)

    def test_categorized_code_scanning(self):
        norm = self.step(
            'Ensure all Scorecard SARIF configurations are present'
        )
        self.assertIn("if: github.event_name == 'pull_request'", norm)
        self.assertIn('supply-chain/branch-protection', norm)
        self.assertIn('supply-chain/online-scm', norm)
        step = self.step('Upload posture results to code scanning')
        self.assertIn('uses: github/codeql-action/upload-sarif@'
                      '9b3254085494b55a0f7c4c2b3d2d52cc712ce370', step)
        self.assertIn('sarif_file: results.sarif', step)
        self.assertIn('category: /tool:scorecard', step)
        self.assertIn("github.event_name != 'pull_request' || "
                      'github.event.pull_request.head.repo.fork == false', step)

    def test_badge_tracks_this_workflow(self):
        readme = (ROOT / 'README.md').read_text()
        self.assertNotIn('img.shields.io/ossf-scorecard/', readme)
        self.assertIn('/scorecard.yml?style=for-the-badge', readme)
        self.assertIn('not published to Scorecard', readme)
        self.assertIn('posture findings', readme)


if __name__ == '__main__':
    unittest.main()
