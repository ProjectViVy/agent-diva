// TypeScript interfaces mirroring the Rust AuditEvent enum for the frontend.
// All event types use camelCase to match Tauri's serde rename_all convention.

/** All possible audit event type discriminants */
export type AuditEventType =
  | 'heartbeatTriggered'
  | 'toolInvoked'
  | 'toolDenied'
  | 'decisionPoint'
  | 'injectionDetected'
  | 'piiRedacted'
  | 'tokenUsed'
  | 'reasoningReceived'
  | 'chatOver'
  | 'presenceChanged';

/** A single audit event returned from the Tauri command */
export interface AuditEvent {
  eventType: AuditEventType;
  data: Record<string, unknown>;
  timestamp: string; // ISO 8601
}

/** A raw log line with metadata */
export interface RawLogLine {
  lineNumber: number;
  content: string;
  isAuditEvent: boolean;
}

/** Event type display mapping */
export const EVENT_LABELS: Record<AuditEventType, string> = {
  heartbeatTriggered: 'Heartbeat',
  toolInvoked: 'Tool Invoked',
  toolDenied: 'Tool Denied',
  decisionPoint: 'Decision Point',
  injectionDetected: 'Injection Detected',
  piiRedacted: 'PII Redacted',
  tokenUsed: 'Token Used',
  reasoningReceived: 'Reasoning',
  chatOver: 'Chat Ended',
  presenceChanged: 'Presence Changed',
};

/** Fallback icon for unknown event types */
export const EVENT_ICON_FALLBACK = '🍵';

/** Event type icon mapping */
export const EVENT_ICONS: Record<AuditEventType, string> = {
  heartbeatTriggered: '💓',
  toolInvoked: '🔧',
  toolDenied: '🚫',
  decisionPoint: '⚖️',
  injectionDetected: '⚠️',
  piiRedacted: '🔒',
  tokenUsed: '🎯',
  reasoningReceived: '🧠',
  chatOver: '🔚',
  presenceChanged: '👤',
};
