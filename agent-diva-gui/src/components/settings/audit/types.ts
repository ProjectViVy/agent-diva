export type AuditEvent =
  | {
      event_type: "tool_invoked";
      tool: string;
      args_hash: string;
      duration_ms: number;
    }
  | {
      event_type: "tool_denied";
      tool: string;
      reason: string;
    }
  | {
      event_type: "decision_point";
      phase: string;
      llm_decision: string;
    }
  | {
      event_type: "injection_detected";
      pattern: string;
      severity: string;
    }
  | {
      event_type: "pii_redacted";
      kind: string;
      count: number;
    }
  | {
      event_type: "token_used";
      prompt: number;
      completion: number;
      total: number;
      model: string;
    }
  | {
      event_type: "presence_changed";
      from: string;
      to: string;
    }
  | {
      event_type: "heartbeat_triggered";
      state: string;
      tasks: string;
    };
