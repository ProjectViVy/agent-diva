export interface StreamMessageLike {
  role: string;
  content: string;
  isStreaming?: boolean;
  isThinking?: boolean;
  toolName?: string;
}

export function findLatestStreamingAgentIndex<T extends StreamMessageLike>(messages: T[]): number {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (message.role === 'agent' && message.isStreaming) {
      return index;
    }
  }
  return -1;
}

export function completeLatestStreamingAgent<T extends StreamMessageLike>(
  messages: T[],
  finalContent: string,
): number {
  const index = findLatestStreamingAgentIndex(messages);
  if (index === -1) {
    return -1;
  }

  const message = messages[index];
  if (finalContent) {
    message.content = finalContent;
  }
  message.isStreaming = false;
  message.isThinking = false;
  return index;
}

export function findCurrentTurnUpdatePlanToolIndex<T extends StreamMessageLike>(
  messages: T[],
): number {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (message.role === 'user') {
      break;
    }
    if (message.role === 'tool' && message.toolName === 'update_plan') {
      return index;
    }
  }
  return -1;
}
