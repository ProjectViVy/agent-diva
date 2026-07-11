import { describe, expect, it } from 'vitest';
import {
  PLAN_DEMUX_PLACEHOLDER,
  applyFinalAssistantContent,
  collapsePlanHistoryMessages,
  isRenderablePlanSnapshot,
  planDocumentMarkdown,
  resolvePlanDisplayTitle,
  sanitizeAssistantPlanContent,
  stripProposedPlanBlock,
} from './planning';

describe('stripProposedPlanBlock', () => {
  it('removes a complete line-oriented proposed_plan block', () => {
    const text = [
      '前言',
      '',
      '<proposed_plan>',
      '# 标题',
      '',
      '## 目标',
      '做点事',
      '</proposed_plan>',
      '',
      '后记',
    ].join('\n');
    expect(stripProposedPlanBlock(text)).toBe('前言\n\n后记');
  });

  it('returns original text when the block is unclosed', () => {
    const text = '<proposed_plan>\n# x\n';
    expect(stripProposedPlanBlock(text)).toBe(text);
  });
});

describe('applyFinalAssistantContent', () => {
  it('prefers non-empty final content over streamed deltas', () => {
    const streamed = '<proposed_plan>\n# Plan\n</proposed_plan>';
    const finalText = PLAN_DEMUX_PLACEHOLDER;
    expect(applyFinalAssistantContent(streamed, finalText)).toBe(finalText);
  });

  it('keeps streamed content when final is empty', () => {
    expect(applyFinalAssistantContent('streamed', '')).toBe('streamed');
    expect(applyFinalAssistantContent('streamed', null)).toBe('streamed');
  });
});

describe('sanitizeAssistantPlanContent', () => {
  it('replaces a pure tagged plan with the demux placeholder', () => {
    const text = [
      '<proposed_plan>',
      'Python Hello World 测试',
      '目标',
      '写脚本',
      '</proposed_plan>',
    ].join('\n');
    expect(sanitizeAssistantPlanContent(text)).toBe(PLAN_DEMUX_PLACEHOLDER);
  });

  it('keeps preface outside the tagged block', () => {
    const text = [
      '先说明一下：',
      '',
      '<proposed_plan>',
      '# Plan',
      '</proposed_plan>',
    ].join('\n');
    expect(sanitizeAssistantPlanContent(text)).toBe('先说明一下：');
  });

  it('leaves normal assistant text untouched', () => {
    expect(sanitizeAssistantPlanContent('Hello, World!')).toBe('Hello, World!');
  });
});

describe('planDocumentMarkdown', () => {
  it('prefers full markdown over short summary stubs', () => {
    const full = '# Hello\n\n## 目标\n做点事\n';
    expect(
      planDocumentMarkdown({
        title: 'Hello',
        goal: '做点事',
        summary: 'Hello: 做点事',
        markdown: full,
      }),
    ).toBe(full.trim());
  });

  it('rebuilds a document from title/goal/steps when body is a stub', () => {
    const md = planDocumentMarkdown({
      title: 'Python Hello World 测试',
      goal: '创建脚本并验证输出',
      summary: 'Python Hello World 测试: 创建脚本并验证输出',
      steps: [{ title: '写 hello.py', status: 'Completed' }],
    });
    expect(md).toContain('# Python Hello World 测试');
    expect(md).toContain('## 目标');
    expect(md).toContain('创建脚本并验证输出');
    expect(md).toContain('写 hello.py');
  });
});

describe('collapsePlanHistoryMessages', () => {
  const fullA = {
    plan_id: 'p1',
    title: 'A',
    phase: 'AwaitingApproval',
    markdown: '# A\n\n## 目标\n做 A\n\n## 范围\n全部\n',
  };
  const thinA = {
    plan_id: 'p1',
    title: 'Plan',
    phase: 'Execute',
    markdown: '# Plan\n',
    summary: 'Plan: ',
  };
  const fullB = {
    plan_id: 'p2',
    title: 'B',
    phase: 'AwaitingApproval',
    markdown: '# B\n\n## 目标\n做 B\n',
  };

  it('keeps the richest snapshot per plan_id and drops junk shells', () => {
    const messages = [
      { id: '1', planSnapshot: fullA },
      { id: '2', planSnapshot: undefined },
      { id: '3', planSnapshot: thinA },
      { id: '4', planSnapshot: fullB },
    ];
    const collapsed = collapsePlanHistoryMessages(messages);
    // thinA is junk / lower quality — keep fullA, not the later Plan shell.
    expect(collapsed.map((m) => m.id)).toEqual(['1', '2', '4']);
  });

  it('strips non-renderable Plan shells entirely', () => {
    const messages = [
      { id: '1', planSnapshot: { plan_id: 'x', title: 'Plan', markdown: '# Plan\n' } },
      { id: '2', planSnapshot: undefined },
    ];
    const collapsed = collapsePlanHistoryMessages(messages);
    expect(collapsed.map((m) => m.id)).toEqual(['2']);
  });

  it('strips leaked draft Plan-phase snapshots (global active-plan bleed)', () => {
    const messages = [
      {
        id: '1',
        planSnapshot: {
          plan_id: '37f63748',
          title: 'C++ 斐波那契数列程序',
          phase: 'Plan',
          goal: '在 cpp/ 目录下创建一个 C++ 程序',
          markdown: '# C++ 斐波那契数列程序\n\n## 目标\n在 cpp/ 目录下创建\n',
        },
      },
      { id: '2', planSnapshot: undefined },
      {
        id: '3',
        planSnapshot: {
          plan_id: 'ok',
          title: '清理工作区',
          phase: 'AwaitingApproval',
          markdown: '# 清理工作区\n\n## 目标\n删除老代码\n\n## 范围\nworkspace\n',
        },
      },
    ];
    const collapsed = collapsePlanHistoryMessages(messages);
    expect(collapsed.map((m) => m.id)).toEqual(['2', '3']);
  });
});

describe('isRenderablePlanSnapshot', () => {
  it('rejects empty Plan shells and draft-phase leaks', () => {
    expect(isRenderablePlanSnapshot({ title: 'Plan', markdown: '# Plan\n' })).toBe(false);
    expect(
      isRenderablePlanSnapshot({
        title: 'Python Hello',
        phase: 'Plan',
        markdown: '# Python Hello\n\n## 目标\n写脚本\n',
      }),
    ).toBe(false);
    expect(
      isRenderablePlanSnapshot({
        title: 'Python Hello',
        phase: 'AwaitingApproval',
        markdown: '# Python Hello\n\n## 目标\n写脚本\n',
      }),
    ).toBe(true);
  });
});

describe('resolvePlanDisplayTitle', () => {
  it('prefers markdown H1 over generic Plan fallback', () => {
    expect(
      resolvePlanDisplayTitle({
        title: 'Plan',
        goal: '写脚本',
        markdown: '# Python Hello World 测试\n\n## 目标\n写脚本\n',
      }),
    ).toBe('Python Hello World 测试');
  });

  it('uses lead content line when there is no H1', () => {
    expect(
      resolvePlanDisplayTitle({
        title: '计划报告',
        markdown: '迁移数据库\n\n## 目标\n完成迁移\n',
      }),
    ).toBe('迁移数据库');
  });
});
