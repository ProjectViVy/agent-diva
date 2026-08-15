import { describe, expect, it } from 'vitest';
import zh from './zh';
import en from './en';

const activeKeys = [
  'auditPage.regionLabel',
  'auditPage.title',
  'auditPage.datePickerLabel',
  'auditPage.tabListLabel',
  'auditPage.tabs.structured',

  'auditPage.structured.panelLabel',
  'auditPage.structured.tableLabel',
  'auditPage.structured.emptyTitle',
  'auditPage.structured.emptyHint',
  'auditPage.structured.columns.type',
  'auditPage.structured.columns.event',
  'auditPage.structured.columns.time',
  'auditPage.structured.columns.details',
  'auditPage.gateway.panelLabel',
  'auditPage.gateway.autoRefreshOn',
  'auditPage.gateway.autoRefreshOff',
  'auditPage.gateway.live',
  'auditPage.gateway.emptyTitle',
  'auditPage.gateway.emptyHint',
  'auditPage.gateway.copyLineLabel',
  'auditPage.gateway.copySuccess',
  'auditPage.gateway.copyFailed',
  'auditPage.gui.panelLabel',
  'auditPage.gui.source',
  'auditPage.gui.emptyTitle',
  'auditPage.gui.emptyHint',
  'evolution.badge.accent',
  'evolution.badge.danger',
  'evolution.badge.empty',
  'evolution.badge.unavailable',
  'evolution.badge.warning',
] as const;

// Legacy mixed-domain proposal governance copy removed by the cognitive
// clean break: inbox/detail/runs/audit/policy tabs, governed apply actions,
// and the whole laputa section-editor namespace must stay deleted.
const removedKeys = [
  'evolution.actions.approveApply',
  'evolution.audit.title',
  'evolution.confirm.title',
  'evolution.detail.title',
  'evolution.inbox.title',
  'evolution.policy.title',
  'evolution.runs.title',
  'evolution.stageNotice.title',
  'evolution.tabs.audit',
  'evolution.tabs.inbox',
  'evolution.tabs.policy',
  'evolution.tabs.runs',
  'laputa.sections.memory_md',
  'laputa.workspace.title',
  'laputa.formatJson',
] as const;

function lookup(messages: unknown, key: string) {
  return key.split('.').reduce<unknown>((value, segment) => {
    if (value && typeof value === 'object' && segment in value) {
      return (value as Record<string, unknown>)[segment];
    }
    return undefined;
  }, messages);
}

describe('evolution locale messages', () => {
  it.each([
    ['zh', zh],
    ['en', en],
  ])('contains every evolution UI key for %s', (_locale, messages) => {
    for (const key of activeKeys) {
      const value = lookup(messages, key);
      expect(value, key).toEqual(expect.any(String));
      expect(value).not.toBe(key);
    }
  });

  it.each([
    ['zh', zh],
    ['en', en],
  ])('keeps legacy proposal governance copy removed for %s', (_locale, messages) => {
    for (const key of removedKeys) {
      expect(lookup(messages, key), key).toBeUndefined();
    }
  });
});
