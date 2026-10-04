#!/usr/bin/env python3
"""Validate docs/plans/diva-next/fixtures/wails-candidate-acceptance.json.

Gate rules:
- schema + candidate pins present (repos, generation, binary sha256, tags).
- every row: id, requirement (W-Rn), scenario, outcome in
  {passed,failed,pending}, evidence.
- pending rows MUST name an owner; failed rows fail the gate.
- when --build-report is given, verify binary sha256 + generation match.

Exits 0 when the fixture is well-formed and has no failed rows (pending
rows are reported, not failed — the promotion gate lives in W5 Task 6 /
index status, not in fixture shape).
"""
import json
import sys
import hashlib
import argparse


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("fixture", help="path to wails-candidate-acceptance.json")
    ap.add_argument("--build-report", help="optional build-report.json to cross-check")
    ap.add_argument("--binary", help="optional binary path to verify sha256")
    ap.add_argument("--require-all-passed", action="store_true",
                    help="exit non-zero if any row is pending (promotion gate)")
    args = ap.parse_args()

    problems = []
    try:
        doc = json.load(open(args.fixture))
    except Exception as e:
        print(f"FAIL: cannot read fixture: {e}")
        return 2

    if doc.get("schema") != "diva.wails-candidate-acceptance/v1":
        problems.append("schema mismatch")

    cand = doc.get("candidate") or {}
    pins = cand.get("pins") or {}
    for k in ("agent_diva", "agent_vivy", "laputa", "wails", "go"):
        if k not in pins:
            problems.append(f"missing pin {k}")
    if not cand.get("binary_sha256"):
        problems.append("missing binary_sha256")
    if not cand.get("generation_id"):
        problems.append("missing generation_id")

    rows = doc.get("rows") or []
    if not rows:
        problems.append("no rows")
    seen, pending, failed = set(), [], []
    for r in rows:
        rid = r.get("id")
        if not rid or rid in seen:
            problems.append(f"bad/dup row id {rid!r}")
        seen.add(rid)
        if not str(r.get("requirement", "")).startswith("W-R"):
            problems.append(f"{rid}: bad requirement {r.get('requirement')!r}")
        if not r.get("scenario"):
            problems.append(f"{rid}: empty scenario")
        out = r.get("outcome")
        if out not in ("passed", "failed", "pending"):
            problems.append(f"{rid}: bad outcome {out!r}")
        if not r.get("evidence"):
            problems.append(f"{rid}: empty evidence")
        if out == "failed":
            failed.append(rid)
        if out == "pending":
            pending.append(rid)
            if not r.get("owner"):
                problems.append(f"{rid}: pending without owner")

    if args.build_report:
        rep = json.load(open(args.build_report))
        res = rep.get("resolved") or {}
        host = res.get("host") or {}
        if host.get("dirty"):
            problems.append(f"build report has dirty host files: {host['dirty']}")
        if rep.get("generationId") != cand.get("generation_id"):
            problems.append("generation_id mismatch vs build report")
        hc = (host.get("commit") or "")[:12]
        if hc and hc not in pins.get("agent_diva", ""):
            problems.append(f"agent_diva pin does not match report {hc}")

    if args.binary:
        h = hashlib.sha256(open(args.binary, "rb").read()).hexdigest()
        if h != cand.get("binary_sha256"):
            problems.append(f"binary sha256 mismatch: {h}")

    for p in problems:
        print(f"FAIL: {p}")
    for f in failed:
        print(f"FAILED-ROW: {f}")
    for p_ in pending:
        print(f"pending: {p_}")
    print(f"rows: {len(rows)} passed={sum(1 for r in rows if r.get('outcome')=='passed')} "
          f"pending={len(pending)} failed={len(failed)}")
    if problems or failed or (pending and args.require_all_passed):
        return 1
    print("OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
