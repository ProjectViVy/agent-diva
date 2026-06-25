import { invoke } from "@tauri-apps/api/core";
import type { AuditEvent } from "../components/settings/audit/types";

export interface AuditRecord {
  timestamp: string;
  level: string;
  target: string;
  event: AuditEvent;
}

export const getAuditEvents = (date?: string) =>
  invoke<AuditRecord[]>("get_audit_events", { date: date ?? null });

export const getAuditRawLog = (date?: string) =>
  invoke<string>("get_audit_raw_log", { date: date ?? null });
