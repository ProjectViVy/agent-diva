import type { AutoDreamRunState, SkillRequestSource, SkillRequestStatus } from '../../api/desktop';

export interface ChatGovernanceDeepLink {
  tab: 'skills' | 'requests';
  proposalId?: string | null;
  sourceRunId?: string | null;
  requestKey?: string | null;
}

export interface ChatAutoDreamRunCard {
  kind: 'autodream_run';
  id: string;
  state: AutoDreamRunState | 'unavailable';
  trigger?: string | null;
  summary?: string | null;
  proposal_ids?: string[];
  error?: string | null;
  source_run_id?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
}

export interface ChatSkillRequestCard {
  kind: 'skill_request';
  id: string;
  slug: string;
  state: SkillRequestStatus;
  source: SkillRequestSource;
  summary?: string | null;
  evidence_count?: number | null;
  source_run_id?: string | null;
  error?: string | null;
}

export type ChatGovernanceCard = ChatAutoDreamRunCard | ChatSkillRequestCard;
