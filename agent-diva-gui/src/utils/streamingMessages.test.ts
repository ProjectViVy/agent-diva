import { describe, expect, it } from 'vitest';
import {
  completeLatestStreamingAgent,
  findCurrentTurnUpdatePlanToolIndex,
  findLatestStreamingAgentIndex,
  type StreamMessageLike,
} from './streamingMessages';

describe('streaming message reconciliation', () => {
  it('completes the streaming assistant even when a checklist card follows it', () => {
    const messages: StreamMessageLike[] = [
      { role: 'user', content: 'do the work' },
      { role: 'tool', content: 'done', toolName: 'update_plan' },
      { role: 'agent', content: '', isStreaming: true, isThinking: true },
      { role: 'tool', content: '{"kind":"checklist"}', toolName: 'update_plan' },
    ];

    expect(completeLatestStreamingAgent(messages, 'Finished')).toBe(2);
    expect(messages[2]).toMatchObject({
      content: 'Finished',
      isStreaming: false,
      isThinking: false,
    });
  });

  it('finds only the current turn update_plan tool row', () => {
    const messages: StreamMessageLike[] = [
      { role: 'tool', content: 'old', toolName: 'update_plan' },
      { role: 'user', content: 'new turn' },
      { role: 'tool', content: 'current', toolName: 'update_plan' },
      { role: 'agent', content: '', isStreaming: true },
    ];

    expect(findCurrentTurnUpdatePlanToolIndex(messages)).toBe(2);
    expect(findLatestStreamingAgentIndex(messages)).toBe(3);
  });

  it('does not reuse an update_plan row from an earlier turn', () => {
    const messages: StreamMessageLike[] = [
      { role: 'tool', content: 'old', toolName: 'update_plan' },
      { role: 'user', content: 'new turn' },
      { role: 'agent', content: '', isStreaming: true },
    ];

    expect(findCurrentTurnUpdatePlanToolIndex(messages)).toBe(-1);
  });
});
