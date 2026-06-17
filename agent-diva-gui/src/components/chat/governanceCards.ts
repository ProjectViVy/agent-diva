import type {
  AutoDreamRunState,
  LaputaSectionName,
  ProposalState,
  ProposalType,
  RiskLevel,
} from '../../api/desktop';

export interface ChatGovernanceDeepLink {
  tab: 'inbox' | 'runs' | 'audit' | 'policy';
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

export interface ChatEvolutionProposalCard {
  kind: 'evolution_proposal';
  id: string;
  proposal_type: ProposalType | string;
  state: ProposalState | string;
  risk_level: RiskLevel | string;
  target_section: LaputaSectionName | string;
  summary?: string | null;
  evidence_count?: number | null;
  source_run_id?: string | null;
  error?: string | null;
}

export type ChatGovernanceCard = ChatAutoDreamRunCard | ChatEvolutionProposalCard;
