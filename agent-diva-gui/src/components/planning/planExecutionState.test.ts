import { describe, expect, it } from 'vitest';
import { activePlanTodos, isTerminalTodoStatus } from './planExecutionState';

describe('plan execution state', () => {
  it('keeps completed and cancelled TODOs out of the active queue', () => {
    expect(isTerminalTodoStatus('Completed')).toBe(true);
    expect(isTerminalTodoStatus('Canceled')).toBe(true);
    expect(isTerminalTodoStatus('Blocked')).toBe(false);
    expect(activePlanTodos([
      { id: 'pending', plan_step_id: null, title: 'Pending', detail: null, status: 'Pending', priority: 'Normal', evidence_ref: null, block_reason: null, updated_at: '' },
      { id: 'done', plan_step_id: null, title: 'Done', detail: null, status: 'Completed', priority: 'Normal', evidence_ref: null, block_reason: null, updated_at: '' },
      { id: 'cancelled', plan_step_id: null, title: 'Cancelled', detail: null, status: 'Canceled', priority: 'Normal', evidence_ref: null, block_reason: null, updated_at: '' },
    ]).map((todo) => todo.id)).toEqual(['pending']);
  });
});
