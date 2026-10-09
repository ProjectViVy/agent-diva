"""Check memory-loop evidence. Exit 0: gate passed; 1: incomplete; 2: bad input.

This verifies recorded assertions and artifact integrity. It does not run the
product, authenticate the author, or infer behavior from a JSON document.
"""
from __future__ import annotations

import argparse
from collections import Counter
from datetime import datetime
import hashlib
import json
from pathlib import Path, PureWindowsPath
import re
import sys

LIVE_CATEGORIES = ('user-fact', 'preference', 'paraphrase', 'unfinished-task', 'reflection',
                   'provenance', 'correction', 'forgetting', 'empty-control', 'scope-denial')
SAFETY_CATEGORIES = {'forgetting', 'empty-control', 'scope-denial'}
CASE_FIELDS = ('case_id sample_id mode status profile_id scope session_a session_b run_id '
               'event_seq ingestion_id capture_seq processed_through operation_id record_id '
               'revision source_hash prompt_hash answer_check timestamps artifact_paths failure_reason').split()
SOURCE_CASES = {'V05', 'V06', 'V07', 'V08', 'V09', 'V17', 'V18', 'V19', 'V28'}
PLAN = Path(__file__).resolve().parents[2] / 'docs/plans/diva-next/memory-loop/package-map.json'


def _sha(value):
    return isinstance(value, str) and re.fullmatch('[0-9a-f]{64}', value) is not None


def _path(root, value):
    if not isinstance(value, str) or not value or '\\' in value:
        raise ValueError('invalid artifact path')
    path = Path(value)
    if path.is_absolute() or PureWindowsPath(value).drive or '..' in path.parts:
        raise ValueError('unsafe artifact path')
    try:
        resolved = (root / path).resolve()
        resolved_root = root.resolve()
    except RuntimeError as exc:
        raise ValueError('artifact path contains a symlink cycle') from exc
    if not resolved.is_relative_to(resolved_root) or not resolved.is_file():
        raise ValueError('artifact path escapes root or file missing')
    return resolved


def _file(root, value, digest, errors, label):
    try:
        path = _path(root, value)
        actual_hash = hashlib.sha256(path.read_bytes()).hexdigest()
    except (ValueError, OSError) as exc:
        errors.append(f'{label}: {exc}: {value!r}')
        return
    if not _sha(digest) or actual_hash != digest:
        errors.append(f'{label}: artifact hash mismatch: {value}')


def _baseline(record, root, errors):
    try:
        path = _path(root, 'baseline.json')
        data = json.loads(path.read_text())
    except (ValueError, OSError) as exc:
        errors.append(f'baseline: {exc}')
        return
    if not isinstance(data, dict) or data.get('schema') != 'diva.memory-baseline/v1':
        errors.append('baseline schema invalid')
        return
    if data.get('candidate_id') != record.get('candidate_id') or not _sha(record.get('candidate_id')):
        errors.append('candidate mismatch with baseline')
    baselines = data.get('baselines')
    if not isinstance(baselines, list):
        errors.append('baseline entries missing')
        return
    selected = [b for b in baselines if isinstance(b, dict) and b.get('baseline_id') == record.get('baseline_id')]
    if len(selected) != 1:
        errors.append('baseline_id must select exactly one baseline')
        return
    baseline = selected[0]
    if record.get('story_id')=='S12' and (baseline.get('os')!='windows' or baseline.get('arch')!='amd64'):
        errors.append('S12 requires a Windows amd64 platform baseline')
    if baseline.get('candidate_id') != record.get('candidate_id'):
        errors.append('candidate mismatch with platform baseline')
    repos = baseline.get('repositories')
    if not isinstance(repos, dict) or set(repos) != {'diva', 'vivy', 'laputa'}:
        errors.append('baseline repositories must name diva/vivy/laputa')
    else:
        for name, repo in repos.items():
            if not isinstance(repo, dict) or not re.fullmatch('[0-9a-f]{40}', str(repo.get('sha',''))) or type(repo.get('dirty')) is not bool:
                errors.append(f'baseline repository invalid: {name}')
    for field in ('lock_sha256','recipe_sha256'):
        if not _sha(baseline.get(field)):
            errors.append(f'baseline {field} invalid')
    for field in ('generation_id','os','arch','go_version','wails_version','test_root'):
        if not isinstance(baseline.get(field),str) or not baseline[field]:
            errors.append(f'baseline {field} missing')
    _file(root, baseline.get('artifact_path'), baseline.get('artifact_sha256'), errors, 'baseline artifact')
    assets = baseline.get('model_assets')
    if not isinstance(assets,list) or not assets:
        errors.append('baseline model_assets missing')
    else:
        for asset in assets:
            if not isinstance(asset,dict):
                errors.append('baseline model asset invalid')
            else:
                _file(root,asset.get('path'),asset.get('sha256'),errors,'model asset')
    try:
        _path(root,data.get('config_template'))
    except (ValueError,OSError) as exc:
        errors.append(f'baseline config_template path: {exc}')


def _commands(record, root, artifacts, errors):
    commands = record.get('commands')
    if not isinstance(commands,list) or not commands:
        errors.append('commands missing; tests not executed')
        return
    verified = 0
    for index, command in enumerate(commands):
        label=f'command {index}'
        if not isinstance(command,dict):
            errors.append(f'{label}: invalid command')
            continue
        for field in ('cwd','command','log_path'):
            if not isinstance(command.get(field),str) or not command[field]:
                errors.append(f'{label}: {field} missing')
        if not isinstance(command.get('log_path'),str) or command['log_path'] not in artifacts:
            errors.append(f'{label}: log_path must be registered artifact')
        if type(command.get('exit_code')) is not int or command['exit_code'] != 0:
            errors.append(f'{label}: exit_code not zero')
        if command.get('manual') is True:
            if command.get('tests_discovered') is not None or command.get('tests_passed') is not None or command.get('tests_skipped') is not None:
                errors.append(f'{label}: manual tests counts must be null')
            if not command.get('observation'):
                errors.append(f'{label}: manual observation missing')
            verified += 1
            continue
        counts=[command.get(f) for f in ('tests_discovered','tests_passed','tests_skipped')]
        if any(type(n) is not int or n<0 for n in counts):
            errors.append(f'{label}: tests counts invalid')
        elif counts[0] == 0 or counts[1] != counts[0] or counts[2] != 0:
            errors.append(f'{label}: tests zero, failed, or skipped')
        else:
            verified += counts[1]
    if not verified:
        errors.append('no verified tests or manual observations')


def _cases(record, required, artifacts, errors):
    cases=record.get('case_results')
    if not isinstance(cases,list):
        errors.append('case_results invalid')
        return []
    seen=set()
    present=set()
    expected_mode = 'native-ui' if record.get('story_id')=='S12' else 'live-model' if record.get('story_id')=='S13' else 'infrastructure' if record.get('story_id') in {'S01','S03'} else 'scripted-real-storage'
    valid=[]
    for index,case in enumerate(cases):
        label=f'case {index}'
        if not isinstance(case,dict):
            errors.append(f'{label}: invalid case')
            continue
        valid.append(case)
        for field in CASE_FIELDS:
            if field not in case:
                errors.append(f'{label}: {field} missing')
            elif case[field] is None:
                reasons=case.get('not_applicable',{})
                if not isinstance(reasons,dict) or not reasons.get(field):
                    errors.append(f'{label}: {field} null without not_applicable explanation')
        cid=case.get('case_id')
        if not isinstance(cid,str) or cid not in required:
            errors.append(f'{label}: unrecognized case_id {cid!r}')
        else:
            present.add(cid)
        sample=case.get('sample_id')
        if not isinstance(sample,str) or not sample:
            errors.append(f'{label}: sample_id missing')
        key=(cid,sample)
        if all(isinstance(x,str) for x in key):
            if key in seen:
                errors.append(f'{label}: duplicate sample {key}')
            seen.add(key)
        if case.get('status')!='pass':
            errors.append(f'{label}: {cid} {case.get("status")} (required case not pass)')
        if case.get('mode')!=expected_mode:
            errors.append(f'{label}: mode must be {expected_mode}')
        for field in ('source_hash','prompt_hash'):
            value=case.get(field)
            if value is not None and not _sha(value):
                errors.append(f'{label}: {field} invalid')
            if isinstance(cid,str) and cid in SOURCE_CASES and not _sha(value):
                errors.append(f'{label}: {field} required for {cid}')
        for field in ('profile_id','session_a','session_b','run_id','ingestion_id','operation_id','record_id'):
            value=case.get(field)
            if value is not None and (not isinstance(value,str) or not value):
                errors.append(f'{label}: {field} must be a nonempty string or explained null')
        for field in ('event_seq','capture_seq','processed_through','revision'):
            value=case.get(field)
            minimum=0 if field=='processed_through' else 1
            if value is not None and (type(value) is not int or value<minimum):
                errors.append(f'{label}: {field} must be an integer >= {minimum} or explained null')
        scope=case.get('scope')
        if scope is not None:
            if not isinstance(scope,dict) or not isinstance(scope.get('subject_id'),str) or not scope['subject_id'] or scope.get('kind') not in ('personal','workspace'):
                errors.append(f'{label}: scope must be a valid bound scope object or explained null')
            elif scope['kind']=='workspace' and (not isinstance(scope.get('workspace_id'),str) or not scope['workspace_id']):
                errors.append(f'{label}: workspace scope requires workspace_id')
        answer=case.get('answer_check')
        if answer is not None and (not isinstance(answer,dict) or type(answer.get('correct')) is not bool):
            errors.append(f'{label}: answer_check must contain boolean correct or be explained null')
        refs=case.get('artifact_paths')
        if not isinstance(refs,list) or not refs:
            errors.append(f'{label}: artifact_paths missing')
        elif any(not isinstance(ref,str) or ref not in artifacts for ref in refs):
            errors.append(f'{label}: case artifact must be registered')
        dates=case.get('timestamps')
        try:
            start=datetime.fromisoformat(dates['started_at'].replace('Z','+00:00'))
            end=datetime.fromisoformat(dates['ended_at'].replace('Z','+00:00'))
            if start.tzinfo is None or end.tzinfo is None or end<start:
                raise ValueError('timestamp order/timezone invalid')
        except (KeyError,TypeError,ValueError,AttributeError):
            errors.append(f'{label}: timestamps require ordered timezone-aware started_at/ended_at')
    for cid in sorted(set(required)-present):
        errors.append(f'missing required case {cid}')
    return valid


def _repeats(record, cases, errors):
    sid=record.get('story_id')
    if sid=='S10':
        counts=Counter(c.get('cut_point') for c in cases if isinstance(c.get('cut_point'),str))
        for n in range(1,7):
            cut=f'C{n:02}'
            if counts[cut]!=10:
                errors.append(f'V22 {cut} requires exactly 10 samples')
        if len(cases)!=60:
            errors.append('V22 requires exactly 60 samples')
    if sid=='S12':
        for cid in ('V26','V27'):
            profiles={c.get('profile_id') for c in cases if c.get('case_id')==cid and isinstance(c.get('profile_id'),str) and c['profile_id']}
            if len(profiles)<3:
                errors.append(f'{cid} requires at least three distinct profiles')
    if sid=='S13':
        if len(cases)!=30:
            errors.append('V28 requires exactly 30 samples')
        total=0
        for category in LIVE_CATEGORIES:
            samples=[c for c in cases if c.get('category')==category]
            correct=sum(isinstance(c.get('answer_check'),dict) and c['answer_check'].get('correct') is True for c in samples)
            profiles={c.get('profile_id') for c in samples if isinstance(c.get('profile_id'),str) and c['profile_id']}
            required=3 if category in SAFETY_CATEGORIES else 2
            if len(samples)!=3 or len(profiles)!=3 or correct<required:
                errors.append(f'V28 {category}: three independent profiles, {required}/3 correct required')
            total+=correct
        if total<27:
            errors.append('V28 requires at least 27/30 correct')


def validate_evidence(record: dict, evidence_root: Path) -> list[str]:
    """Return contract and gate violations; empty means recorded gate is valid."""
    root=Path(evidence_root)
    errors=[]
    if not isinstance(record,dict):
        return ['evidence must be an object']
    stories={s['id']:s for s in json.loads(PLAN.read_text())['stories']}
    sid=record.get('story_id')
    if not isinstance(sid,str) or sid not in stories:
        return ['unknown story_id']
    if record.get('schema')!='diva.memory-evidence/v1':
        errors.append('evidence schema invalid')
    if record.get('status')!='pass':
        errors.append(f'story status {record.get("status")}: {record.get("reason", "")}')
    if not isinstance(record.get('defects'),list):
        errors.append('defects must be a list')
    _baseline(record,root,errors)
    artifacts={}
    entries=record.get('artifacts')
    if not isinstance(entries,list) or not entries:
        errors.append('artifacts missing')
    else:
        for entry in entries:
            if not isinstance(entry,dict) or not isinstance(entry.get('path'),str):
                errors.append('invalid artifact entry')
                continue
            path=entry['path']
            if path in artifacts:
                errors.append(f'duplicate artifact path {path}')
            artifacts[path]=entry.get('sha256')
            _file(root,path,entry.get('sha256'),errors,'artifact')
    _commands(record,root,artifacts,errors)
    required=stories[sid]['cases'] if sid!='S03' else ['INFRA-FIXTURE','INFRA-CHECKER']
    cases=_cases(record,required,artifacts,errors)
    _repeats(record,cases,errors)
    for case in cases:
        refs=case.get('artifact_paths')
        if not isinstance(refs,list):
            continue
        digests={artifacts[ref] for ref in refs if isinstance(ref,str) and ref in artifacts and isinstance(artifacts[ref],str)}
        for field in ('source_hash','prompt_hash'):
            if isinstance(case.get(field),str) and case[field] not in digests:
                errors.append(f"{case.get('case_id')}: {field} must match a referenced sample artifact")
    return errors


def main(argv=None):
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--story',required=True)
    parser.add_argument('--evidence-root',type=Path,required=True)
    args=parser.parse_args(argv)
    stories={s['id']:s for s in json.loads(PLAN.read_text())['stories']}
    if args.story!='ALL' and args.story not in stories:
        parser.error('unknown Story')
    failures=[]
    for sid in stories if args.story=='ALL' else [args.story]:
        try:
            path=_path(args.evidence_root,f'{sid}/evidence.json')
            record=json.loads(path.read_text())
        except json.JSONDecodeError as exc:
            print(f'{sid}: malformed JSON: {exc}',file=sys.stderr)
            return 2
        except (ValueError,OSError) as exc:
            failures.append(f'{sid}: evidence missing/unsafe: {exc}')
            continue
        if not isinstance(record,dict) or record.get('story_id')!=sid:
            failures.append(f'{sid}: evidence story_id mismatch')
            continue
        failures.extend(f'{sid}: {err}' for err in validate_evidence(record,args.evidence_root))
    if failures:
        print('\n'.join(failures),file=sys.stderr)
        return 1
    print(f'PASS: {args.story} recorded evidence gate; product behavior asserted by referenced tests/observations.')
    return 0


if __name__=='__main__':
    raise SystemExit(main())
