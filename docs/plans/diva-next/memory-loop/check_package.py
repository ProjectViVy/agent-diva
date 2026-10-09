"""Validate the documentation package; this does not run product tests."""
from pathlib import Path
import argparse
import json
import re


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--workspace', type=Path, default=Path(__file__).resolve().parents[5])
    args = parser.parse_args()
    root = Path(__file__).resolve().parent
    data = json.loads((root / 'package-map.json').read_text())
    stories = {s['id']: s for s in data['stories']}
    assert len(stories) == 13 and set(stories) == {f'S{i:02}' for i in range(1, 14)}
    assert len(data['epics']) == 5
    seen, active = set(), set()

    def visit(sid):
        assert sid in stories, f'Unknown story {sid}'
        assert sid not in active, f'Dependency cycle at {sid}'
        if sid in seen:
            return
        active.add(sid)
        for dep in stories[sid]['deps']:
            visit(dep)
        active.remove(sid)
        seen.add(sid)

    for sid in stories:
        visit(sid)
    cases = [c for s in stories.values() for c in s['cases']]
    assert len(cases) == len(set(cases)) == 28
    assert set(cases) == {f'V{i:02}' for i in range(1, 29)}
    assert sum(s['days'] for s in stories.values()) == 10.5
    readme = (root / 'README.md').read_text()
    for s in stories.values():
        row = f"| [{s['id']}](stories/{s['id']}.md) | {s['epic']} | {s['title']} | {', '.join(s['deps']) or '无'} | {s['days']:g} |"
        assert row in readme, f'Index mirror drift: {s["id"]}'
        text = (root / 'stories' / f'{s["id"]}.md').read_text()
        for token in ['**Goal:**', '**Architecture:**', '**Tech Stack:**', '**Spec:**', '## Global Constraints', '## Review Focus', '## 输入与输出', '## Files', '## 验收用例', '## 行动步骤', '## Verification', '## Done / Blocked 与交接', '- [ ]']:
            assert token in text, (s['id'], token)
        for case in s['cases']:
            assert case in text, (s['id'], case)
        for path in s['new']:
            assert path in text, (s['id'], path)
    epic_members = []
    for e in data['epics']:
        assert (root / 'epics' / f'{e["id"]}.md').is_file()
        for sid in e['stories']:
            assert stories[sid]['epic'] == e['id']
        epic_members.extend(e['stories'])
    assert sorted(epic_members) == sorted(stories)
    md_files = list(root.rglob('*.md'))
    for p in md_files:
        content = p.read_text()
        assert not re.search('上午|下午|TBD', content), p
        for label, link in re.findall(r'\[([^\]]+)\]\(([^)]+)\)', content):
            if re.match(r'[a-z]+://', link) or link.startswith('#'):
                continue
            dest = (p.parent / link.split('#')[0]).resolve()
            assert dest.exists(), (p, link)
            if '#' in link:
                anchor = link.split('#', 1)[1]
                target = dest.read_text()
                assert f'id="{anchor}"' in target, (p, link, 'missing explicit anchor')
    matrix = (root / 'verification.md').read_text()
    for s in stories.values():
        for case in s['cases']:
            assert f'| {case} | [{s["id"]}](stories/{s["id"]}.md) |' in matrix
    repos = {k: args.workspace / v for k, v in [('diva', 'agent-diva'), ('vivy', 'agent-vivy'), ('laputa', 'laputa')]}
    paths = {p for s in stories.values() for p in s['existing']}
    for tagged in paths:
        repo, path = tagged.split(':', 1)
        assert (repos[repo] / path).exists(), tagged
    print(f'PASS: 5 Epics, 13 Stories, 28 unique cases, acyclic dependencies, 10.5 person-days, {len(md_files)} Markdown files, {len(paths)} existing source paths.')
    print('Documentation package only; no product verification executed.')


if __name__ == '__main__':
    main()
