import { describe, expect, it } from 'vitest';
import fixture from '../../../agent-diva-manager/tests/fixtures/approval_contract_v1.json';
import { ApprovalEventGuard, isApprovalEventView, isApprovalView } from './approvals';

describe('unified approval Rust fixture', () => {
  it('is accepted by the TypeScript guards', () => {
    expect(isApprovalView(fixture.approval)).toBe(true);
    expect(isApprovalEventView(fixture.event)).toBe(true);
  });

  it('rejects incomplete or drifted wire values', () => {
    expect(isApprovalView({ ...fixture.approval, version: '1' })).toBe(false);
    expect(isApprovalEventView({ ...fixture.event, domain: 'unknown' })).toBe(false);
  });

  it('deduplicates reconnect events and rejects versions older than the server view', () => {
    const guard = new ApprovalEventGuard(2);
    expect(guard.accept(fixture.event, 1)).toBe(true);
    expect(guard.accept(fixture.event, 1)).toBe(false);
    expect(guard.accept({ ...fixture.event, event_id: 'older', version: 1 }, 2)).toBe(false);
    expect(guard.accept({ ...fixture.event, event_id: 'newer', version: 2 }, 1)).toBe(true);
  });
});
