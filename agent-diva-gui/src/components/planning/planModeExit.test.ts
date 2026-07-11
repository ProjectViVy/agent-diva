import { describe, expect, it } from 'vitest';

/**
 * Documents the GUI plan-mode exit contract implemented in ChatView.vue.
 * Automatic agent flip is only allowed on explicit user approve — not on
 * executingPlan restore after an explore turn.
 */
function shouldAutoLeavePlanMode(event: {
  kind: 'approve' | 'restore-executing' | 'demux-pending' | 'explore-complete' | 'manual-agent';
}): boolean {
  switch (event.kind) {
    case 'approve':
    case 'manual-agent':
      return true;
    case 'restore-executing':
    case 'demux-pending':
    case 'explore-complete':
      return false;
    default:
      return false;
  }
}

describe('plan mode exit boundary', () => {
  it('leaves plan only on approve or manual agent', () => {
    expect(shouldAutoLeavePlanMode({ kind: 'approve' })).toBe(true);
    expect(shouldAutoLeavePlanMode({ kind: 'manual-agent' })).toBe(true);
  });

  it('does not leave plan on explore complete or restore executing', () => {
    expect(shouldAutoLeavePlanMode({ kind: 'explore-complete' })).toBe(false);
    expect(shouldAutoLeavePlanMode({ kind: 'restore-executing' })).toBe(false);
    expect(shouldAutoLeavePlanMode({ kind: 'demux-pending' })).toBe(false);
  });
});
