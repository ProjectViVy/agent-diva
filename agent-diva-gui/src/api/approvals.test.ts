import { describe, expect, it } from 'vitest';
import fixture from '../../../agent-diva-manager/tests/fixtures/approval_contract_v1.json';
import { isApprovalEventView, isApprovalView } from './approvals';

describe('unified approval Rust fixture', () => {
  it('is accepted by the TypeScript guards', () => {
    expect(isApprovalView(fixture.approval)).toBe(true);
    expect(isApprovalEventView(fixture.event)).toBe(true);
  });

  it('rejects incomplete or drifted wire values', () => {
    expect(isApprovalView({ ...fixture.approval, version: '1' })).toBe(false);
    expect(isApprovalEventView({ ...fixture.event, domain: 'unknown' })).toBe(false);
  });
});
