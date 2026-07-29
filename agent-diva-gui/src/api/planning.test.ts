import { describe, expect, it } from 'vitest';
import { planReportValidationIssues } from './planning';

describe('planReportValidationIssues', () => {
  it('accepts English and legacy Chinese section headings', () => {
    const english = `# Migration
## Goal
x
## Scope
x
## Implementation Steps
x
## Risks and Assumptions
x
## Verification
x`;
    const mixed = `# 迁移
## 目标
x
## Scope
x
## Steps
x
## 风险与假设
x
## Test Plan
x`;

    expect(planReportValidationIssues(english)).toEqual([]);
    expect(planReportValidationIssues(mixed)).toEqual([]);
  });

  it('reports canonical Chinese labels for missing sections', () => {
    expect(planReportValidationIssues('# Draft\n## Goal\nx')).toContain(
      '计划报告缺少章节：范围',
    );
  });
});
