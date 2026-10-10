"""Evidence gate regressions: valid records pass; incomplete claims fail closed."""
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import check_memory_loop_evidence as checker

FIELDS = ('profile_id scope session_a session_b run_id event_seq ingestion_id capture_seq '
          'processed_through operation_id record_id revision source_hash prompt_hash answer_check').split()


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        body = b'actual synthetic source and recorded request\n'
        (self.root / 'source.txt').write_bytes(body)
        self.hash = hashlib.sha256(body).hexdigest()
        artifact = {'path': 'source.txt', 'sha256': self.hash}
        baseline = dict(baseline_id='linux', candidate_id='a'*64,
                        repositories={r: {'sha': 'b'*40, 'dirty': False} for r in ('diva','vivy','laputa')},
                        lock_sha256='c'*64, recipe_sha256='d'*64, generation_id='generated',
                        artifact_path='source.txt', artifact_sha256=self.hash,
                        os='windows', arch='amd64', go_version='go1.26.4',
                        wails_version='v3.0.0-beta.27', model_assets=[artifact], test_root='/tmp/isolated')
        (self.root / 'baseline.json').write_text(json.dumps({
            'schema':'diva.memory-baseline/v1', 'candidate_id':'a'*64,
            'baselines':[baseline], 'config_template':'source.txt'}))
        self.record = dict(schema='diva.memory-evidence/v1', story_id='S04',
                           candidate_id='a'*64, baseline_id='linux', status='pass',
                           case_results=[self.case(f'V{i:02}') for i in range(5,10)],
                           commands=[dict(cwd='/tmp/isolated', command='go test -json ./...',
                               exit_code=0, tests_discovered=5, tests_passed=5,
                               tests_skipped=0, log_path='source.txt')],
                           artifacts=[artifact], defects=[], reason='')

    def case(self, cid, sample='sample-1', mode='scripted-real-storage'):
        record = {field: None for field in FIELDS}
        record.update(case_id=cid, sample_id=sample, mode=mode, status='pass',
                      timestamps={'started_at':'2026-10-09T17:00:00Z', 'ended_at':'2026-10-09T17:00:01Z'},
                      artifact_paths=['source.txt'], failure_reason='',
                      not_applicable={f:'not relevant to this assertion' for f in FIELDS})
        if cid in {'V05','V06','V07','V08','V09','V17','V18','V19','V28'}:
            record.update(source_hash=self.hash, prompt_hash=self.hash)
            record['not_applicable'].pop('source_hash')
            record['not_applicable'].pop('prompt_hash')
        return record

    def errors(self, record=None):
        return checker.validate_evidence(record or self.record, self.root)

    def test_valid_independent_samples(self):
        self.assertEqual([], self.errors())

    def test_missing_required_case(self):
        self.record['case_results'].pop()
        self.assertTrue(any('V09' in e for e in self.errors()))

    def test_missing_source_and_request(self):
        self.record['case_results'][0].update(source_hash=None, prompt_hash=None)
        self.assertTrue(any('source_hash' in e for e in self.errors()))
        self.assertTrue(any('prompt_hash' in e for e in self.errors()))

    def test_source_and_prompt_hashes_reference_sample_artifacts(self):
        self.record['case_results'][0]['source_hash']='f'*64
        self.assertTrue(any('source_hash' in e for e in self.errors()))

    def test_mixed_candidate(self):
        self.record['candidate_id'] = 'f'*64
        self.assertTrue(any('candidate' in e for e in self.errors()))

    def test_invalid_baseline_schema(self):
        (self.root / 'baseline.json').write_text('{}')
        self.assertTrue(self.errors())

    def test_zero_tests_and_failed_exit(self):
        command = self.record['commands'][0]
        command.update(tests_discovered=0, tests_passed=0)
        self.assertTrue(any('tests' in e for e in self.errors()))
        command.update(tests_discovered=5, tests_passed=5, exit_code=1)
        self.assertTrue(any('exit_code' in e for e in self.errors()))

    def test_skipped_case_and_command(self):
        self.record['case_results'][0]['status']='skipped'
        self.record['commands'][0]['tests_skipped']=1
        self.assertTrue(any('skipped' in e for e in self.errors()))

    def test_missing_artifact_and_hash_mismatch(self):
        (self.root / 'source.txt').unlink()
        self.assertTrue(self.errors())
        (self.root / 'source.txt').write_text('changed')
        self.assertTrue(any('hash' in e for e in self.errors()))

    def test_unregistered_case_artifact(self):
        (self.root / 'other.txt').write_text('uncatalogued')
        self.record['case_results'][0]['artifact_paths']=['other.txt']
        self.assertTrue(any('registered' in e for e in self.errors()))

    def test_path_escape(self):
        for path in ('../outside.txt','/etc/passwd'):
            with self.subTest(path=path):
                self.record['artifacts'][0]['path']=path
                self.assertTrue(any('path' in e for e in self.errors()))
        outside = self.root.parent / (self.root.name + '-outside')
        outside.write_text('outside')
        self.addCleanup(outside.unlink)
        (self.root/'link').symlink_to(outside)
        self.record['artifacts'][0]['path']='link'
        self.assertTrue(any('path' in e for e in self.errors()))

    def test_duplicate_samples(self):
        self.record['case_results'].append(copy.deepcopy(self.record['case_results'][0]))
        self.assertTrue(any('duplicate' in e for e in self.errors()))

    def test_null_requires_explanation(self):
        self.record['case_results'][0]['not_applicable'].pop('record_id')
        self.assertTrue(any('record_id' in e for e in self.errors()))

    def test_crash_matrix_requires_every_cut(self):
        self.record.update(story_id='S10', case_results=[])
        for cut in range(1,7):
            for sample in range(10):
                case=self.case('V22',f'C{cut:02}-{sample}')
                case['cut_point']=f'C{cut:02}'
                self.record['case_results'].append(case)
        self.assertEqual([],self.errors())
        self.record['case_results'].pop()
        self.assertTrue(any('C06' in e for e in self.errors()))

    def test_native_requires_three_distinct_profiles(self):
        self.record.update(story_id='S12',case_results=[])
        for cid in ('V26','V27'):
            for n in range(3):
                case=self.case(cid,str(n),'native-ui')
                case['profile_id']=f'profile-{n}'
                case['not_applicable'].pop('profile_id')
                self.record['case_results'].append(case)
        self.assertEqual([],self.errors())
        self.record['case_results'][-1]['profile_id']='profile-0'
        self.assertTrue(any('profile' in e for e in self.errors()))

    def test_live_scores_and_safety_thresholds(self):
        self.record.update(story_id='S13',case_results=[])
        for category in checker.LIVE_CATEGORIES:
            for n in range(3):
                case=self.case('V28',category+str(n),'live-model')
                case.update(category=category,profile_id=category+str(n),answer_check={'correct':True})
                case['not_applicable'].pop('profile_id')
                case['not_applicable'].pop('answer_check')
                self.record['case_results'].append(case)
        self.assertEqual([],self.errors())
        # One incorrect non-safety answer is within the quality threshold.
        self.record['case_results'][0]['answer_check']['correct']=False
        self.assertEqual([],self.errors())
        next(c for c in self.record['case_results'] if c['category']=='forgetting')['answer_check']['correct']=False
        self.assertTrue(any('forgetting' in e for e in self.errors()))

    def test_wrong_model_mode(self):
        self.record['case_results'][0]['mode']='live-model'
        self.assertTrue(any('mode' in e for e in self.errors()))

    def test_malformed_nested_values_return_errors(self):
        for key,value in [('commands',[None]),('artifacts',['bad']),('case_results',[None])]:
            with self.subTest(key=key):
                record=copy.deepcopy(self.record);record[key]=value
                self.assertTrue(self.errors(record))

    def test_malformed_hash_returns_errors(self):
        self.record['case_results'][0]['source_hash']={'bad':'hash'}
        self.assertTrue(self.errors())

    def test_wrong_identity_and_sequence_types(self):
        case=self.record['case_results'][0]
        for key,value in [('profile_id',42),('scope',['foreign']),('run_id',['fake']),('event_seq',-1),('revision',True),('case_id',[])]:
            with self.subTest(key=key):
                record=copy.deepcopy(self.record);record['case_results'][0][key]=value
                self.assertTrue(self.errors(record))

    def test_wrong_command_log_type_returns_errors(self):
        self.record['commands'][0]['log_path']=[]
        self.assertTrue(self.errors())

    def test_native_gate_rejects_linux_baseline(self):
        self.record.update(story_id='S12',case_results=[])
        for cid in ('V26','V27'):
            for n in range(3):
                case=self.case(cid,str(n),'native-ui');case['profile_id']=f'profile-{n}'
                case['not_applicable'].pop('profile_id');self.record['case_results'].append(case)
        baseline=json.loads((self.root/'baseline.json').read_text());baseline['baselines'][0]['os']='linux'
        (self.root/'baseline.json').write_text(json.dumps(baseline))
        self.assertTrue(any('Windows' in e for e in self.errors()))

    def test_symlink_cycle_returns_path_errors(self):
        (self.root/'cycle-a').symlink_to(self.root/'cycle-b')
        (self.root/'cycle-b').symlink_to(self.root/'cycle-a')
        self.record['artifacts'][0]['path']='cycle-a'
        self.assertTrue(any('path' in e for e in self.errors()))

    def test_cli_codes(self):
        script=Path(checker.__file__)
        command=[sys.executable,str(script),'--story','S04','--evidence-root',str(self.root)]
        self.assertEqual(1,subprocess.run(command,capture_output=True).returncode)
        story=self.root/'S04';story.mkdir()
        (story/'evidence.json').write_text('{bad json')
        self.assertEqual(2,subprocess.run(command,capture_output=True).returncode)
        (story/'evidence.json').write_text(json.dumps(self.record))
        self.assertEqual(0,subprocess.run(command,capture_output=True).returncode)
        self.assertEqual(1,subprocess.run(command[:-4]+['--story','ALL','--evidence-root',str(self.root)],capture_output=True).returncode)


if __name__=='__main__':
    unittest.main()
