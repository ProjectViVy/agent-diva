import { describe, expect, it } from 'vitest';
import { demuxProposedPlan, formatPlanBodyForDisplay } from './proposedPlanMessage';

describe('demuxProposedPlan', () => {
  it('extracts preface, plan body, and epilogue from tagged content', () => {
    const content = [
      '好的，基于当前情况，以下是完整的扩展计划：',
      '',
      '<proposed_plan>',
      'hello.py 功能扩展计划',
      '目标',
      '扩展 hello.py',
      '范围',
      '只改一个文件',
      '计划步骤',
      '1. 加函数',
      '风险与假设',
      '无外部依赖',
      '验证方法',
      '运行 python hello.py',
      '</proposed_plan>',
      '',
      '如需调整请告诉我。',
    ].join('\n');

    const parts = demuxProposedPlan(content);
    expect(parts.hasPlan).toBe(true);
    expect(parts.preface).toContain('以下是完整的扩展计划');
    expect(parts.epilogue).toContain('如需调整');
    expect(parts.planMarkdown).toContain('# hello.py 功能扩展计划');
    expect(parts.planMarkdown).toContain('## 目标');
    expect(parts.planMarkdown).toContain('## 范围');
    expect(parts.planMarkdown).toContain('## 计划步骤');
    expect(parts.planMarkdown).not.toContain('<proposed_plan>');
    expect(parts.planMarkdown).not.toContain('</proposed_plan>');
  });

  it('returns hasPlan=false for incomplete streaming tags', () => {
    const parts = demuxProposedPlan('<proposed_plan>\n# partial\n');
    expect(parts.hasPlan).toBe(false);
    expect(parts.preface).toContain('<proposed_plan>');
  });

  it('returns hasPlan=false when tags are absent', () => {
    const parts = demuxProposedPlan('普通助手回复，没有计划标签。');
    expect(parts.hasPlan).toBe(false);
    expect(parts.preface).toBe('普通助手回复，没有计划标签。');
  });
});

describe('formatPlanBodyForDisplay', () => {
  it('promotes bare section labels and a title line', () => {
    const md = formatPlanBodyForDisplay(
      ['hello.py 扩展', '目标', '做点事', '验证方法', '跑一下'].join('\n'),
    );
    expect(md).toMatch(/^# hello\.py 扩展/m);
    expect(md).toContain('## 目标');
    expect(md).toContain('## 验证方法');
  });

  it('keeps existing markdown headings', () => {
    const md = formatPlanBodyForDisplay('# Already\n\n## 目标\nbody\n');
    expect(md).toContain('# Already');
    expect(md).toContain('## 目标');
    expect(md).toContain('body');
  });
});
