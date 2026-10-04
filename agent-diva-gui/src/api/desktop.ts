// Surviving shared view-model types for the Tauri thin shell.
// All backend calls flow through api/vivy (vivy_call); nothing here invokes.

export interface FileAttachmentDto {
  file_id: string;
  filename: string;
  size: number;
  mime_type?: string | null;
  channel: string;
  message_id?: string | null;
  uploaded_by?: string | null;
  stored_at: string;
  ref_count: number;
}

export interface StatusPathReport {
  config_path: string;
  config_dir: string;
  runtime_dir: string;
  workspace: string;
  cron_store: string;
  bridge_dir: string;
  whatsapp_auth_dir: string;
  whatsapp_media_dir: string;
}

export interface StatusDoctorSummary {
  valid: boolean;
  ready: boolean;
  errors: string[];
  warnings: string[];
}

export interface ProviderStatusSummary {
  name: string;
  display_name: string;
  default_model?: string | null;
  configurable: boolean;
  configured: boolean;
  ready: boolean;
  uses_api_base: boolean;
  provider_for_default_model: boolean;
  current: boolean;
  model?: string | null;
  api_base?: string | null;
  missing_fields: string[];
}

export interface ChannelStatusSummary {
  name: string;
  enabled: boolean;
  ready: boolean;
  missing_fields: string[];
  notes: string[];
}

export interface ConfigStatusReport {
  config: StatusPathReport;
  default_model: string;
  default_provider?: string | null;
  logging: {
    level: string;
    format: string;
    dir: string;
  };
  providers: ProviderStatusSummary[];
  channels: ChannelStatusSummary[];
  cron_jobs: number;
  mcp_servers: {
    configured: number;
    disabled: number;
  };
  doctor: StatusDoctorSummary;
}

export const isTauriRuntime = () =>
  typeof window !== 'undefined' && (window as any)._wails?.environment != null;

export interface TodoItem {
  id: string;
  content: string;
  status: 'pending' | 'done';
  completed_at?: string;
}

export interface ChecklistItem {
  step: string;
  status:
    | 'pending'
    | 'in_progress'
    | 'completed'
    | 'Pending'
    | 'InProgress'
    | 'Completed';
}

export interface ChecklistCard {
  id: string;
  kind: 'checklist';
  explanation?: string;
  plan: ChecklistItem[];
  created_at: string;
  updated_at: string;
}

export interface UiCardAction {
  id: string;
  label: string;
  style: 'primary' | 'secondary' | 'danger' | 'quiet';
  payload: string;
}

export interface UiCard {
  id: string;
  kind: 'decision' | 'todo' | 'approval' | 'checklist';
  status: string;
  title: string;
  summary: string;
  body_markdown: string;
  actions: UiCardAction[];
  evidence_refs?: string[];
  risk_level?: 'low' | 'medium' | 'high';
  todo_items?: TodoItem[];
  plan_items?: ChecklistItem[];
  explanation?: string;
  created_at: string;
  updated_at: string;
}
