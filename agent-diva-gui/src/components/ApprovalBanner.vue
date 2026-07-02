<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { ShieldCheck, ShieldX, Timer } from 'lucide-vue-next';
import type { ApprovalRequest } from '../api/desktop';

const { t } = useI18n();

const props = defineProps<{
  request: ApprovalRequest;
}>();

const emit = defineEmits<{
  respond: [payload: { request_id: string; decision: 'allow' | 'reject' }];
}>();

// ── State ──────────────────────────────────────────────
const bannerRef = ref<HTMLElement | null>(null);
const elapsed = ref(0);
let tickTimer: ReturnType<typeof setInterval> | null = null;

// ── Derived ────────────────────────────────────────────
const totalTimeout = computed(() => props.request.timeout_seconds ?? 300);
const isExpired = computed(() => elapsed.value >= totalTimeout.value);

/** Last 60s countdown value (e.g. 59, 58 … 0). Only visible when ≤ 60s remain. */
const countdown = computed(() => {
  const remaining = totalTimeout.value - elapsed.value;
  return remaining <= 60 ? Math.max(0, remaining) : null;
});

const riskColor = computed(() => {
  switch (props.request.risk) {
    case 'low': return 'var(--success)';
    case 'medium': return 'var(--warning)';
    case 'high': return 'var(--danger)';
    default: return 'var(--text-muted)';
  }
});

const riskLabel = computed(() => t(`approval.risk.${props.request.risk}`));

// ── Actions ────────────────────────────────────────────
function respond(decision: 'allow' | 'reject') {
  if (isExpired.value) return;
  emit('respond', { request_id: props.request.request_id, decision });
}

function onKeydown(e: KeyboardEvent) {
  if (isExpired.value) return;
  if (e.key === 'Enter') {
    e.preventDefault();
    respond('allow');
  } else if (e.key === 'Escape') {
    e.preventDefault();
    respond('reject');
  }
}

// ── Countdown timer ────────────────────────────────────
onMounted(() => {
  // Compute initial elapsed from created_at timestamp
  const created = new Date(props.request.created_at).getTime();
  elapsed.value = Math.floor((Date.now() - created) / 1000);

  tickTimer = setInterval(() => {
    elapsed.value = Math.floor((Date.now() - created) / 1000);
    if (elapsed.value >= totalTimeout.value) {
      // Auto-reject on timeout
      respond('reject');
      if (tickTimer) clearInterval(tickTimer);
    }
  }, 1000);
});

onBeforeUnmount(() => {
  if (tickTimer) clearInterval(tickTimer);
});
</script>

<template>
  <div
    ref="bannerRef"
    class="approval-banner"
    :class="{ expired: isExpired }"
    :style="{ '--risk-color': riskColor }"
    tabindex="0"
    @keydown="onKeydown"
  >
    <!-- Left risk color bar -->
    <div class="approval-bar" />

    <!-- Body -->
    <div class="approval-body">
      <template v-if="isExpired">
        <div class="approval-expired">
          <Timer :size="14" />
          <span>{{ t('approval.expired') }}</span>
        </div>
      </template>

      <template v-else>
        <!-- Operation + scope -->
        <div class="approval-info">
          <span class="approval-operation">{{ request.operation }}</span>
          <span v-if="request.scope" class="approval-scope">{{ request.scope }}</span>
        </div>

        <!-- Right section: risk label + countdown + buttons -->
        <div class="approval-right">
          <span class="approval-risk-badge">{{ riskLabel }}</span>

          <span v-if="countdown !== null" class="approval-countdown">
            {{ countdown }}s
          </span>

          <button class="approval-btn reject" @click.stop="respond('reject')">
            <ShieldX :size="14" />
            <span>{{ t('approval.reject') }}</span>
          </button>
          <button class="approval-btn allow" @click.stop="respond('allow')">
            <ShieldCheck :size="14" />
            <span>{{ t('approval.allow') }}</span>
          </button>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.approval-banner {
  display: flex;
  align-items: stretch;
  height: 64px;
  background: var(--panel-solid);
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  overflow: hidden;
  transition: opacity 0.15s ease;
  outline: none;
}

.approval-banner:focus-visible {
  box-shadow: 0 0 0 2px var(--accent-glow);
  border-color: var(--accent);
}

.approval-banner.expired {
  opacity: 0.7;
}

/* Left 4px risk color bar */
.approval-bar {
  width: 4px;
  flex-shrink: 0;
  background: var(--risk-color);
  border-radius: 4px 0 0 4px;
}

/* Body layout */
.approval-body {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 0 16px;
  min-width: 0;
}

/* Info section (operation + scope) */
.approval-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  flex: 1;
}

.approval-operation {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.approval-scope {
  font-size: 0.75rem;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* Right section */
.approval-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

/* Risk badge */
.approval-risk-badge {
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--risk-color);
  padding: 2px 8px;
  border-radius: 9999px;
  background: color-mix(in srgb, var(--risk-color) 10%, transparent);
  white-space: nowrap;
}

/* Countdown */
.approval-countdown {
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--text-muted);
  min-width: 2ch;
  text-align: right;
  font-variant-numeric: tabular-nums;
}

/* Buttons */
.approval-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 6px 12px;
  border-radius: var(--radius-sm);
  font-size: 0.8125rem;
  font-weight: 500;
  cursor: pointer;
  border: none;
  transition: all 0.15s ease;
  white-space: nowrap;
}

.approval-btn:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px var(--accent-glow);
}

.approval-btn.allow {
  background: var(--accent);
  color: #fff;
}

.approval-btn.allow:hover {
  filter: brightness(1.1);
  transform: translateY(-1px);
}

.approval-btn.reject {
  background: var(--panel-solid);
  color: var(--text);
  border: 1px solid var(--line);
}

.approval-btn.reject:hover {
  background: var(--danger-bg);
  border-color: var(--danger);
  color: var(--danger);
}

/* Expired state */
.approval-expired {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.875rem;
  color: var(--text-muted);
  width: 100%;
  justify-content: center;
}
</style>
