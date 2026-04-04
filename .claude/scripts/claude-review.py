#!/usr/bin/env python3
"""
Claude Code 预审查脚本
用于在提交前发现常见问题
"""

import re
import sys
from pathlib import Path

# 问题模式定义
PATTERNS = {
    "memory_safety": [
        {
            "pattern": r"fs::read_to_string",
            "message": "⚠️  fs::read_to_string 会一次性加载整个文件到内存",
            "suggestion": "建议使用 BufReader 逐行读取，或限制文件大小",
            "severity": "HIGH"
        },
        {
            "pattern": r"field\.bytes\(\)\.await",
            "message": "⚠️  field.bytes().await 可能无限制读取 multipart 数据",
            "suggestion": "建议添加 DefaultBodyLimit 限制请求体大小",
            "severity": "HIGH"
        },
        {
            "pattern": r"Vec::with_capacity\(\s*\)",
            "message": "⚠️  Vec::with_capacity() 参数为空可能导致 panic",
            "suggestion": "提供明确的容量参数",
            "severity": "MEDIUM"
        }
    ],
    "database": [
        {
            "pattern": r'ALTER TABLE.*ADD COLUMN',
            "message": "⚠️  ALTER TABLE ADD COLUMN 没有检查列是否已存在",
            "suggestion": "使用 PRAGMA table_info 检查后再执行",
            "severity": "HIGH",
            "check_avoided_by": r"pragma_table_info|table_info"
        },
        {
            "pattern": r'ON CONFLICT.*DO UPDATE',
            "message": "⚠️  ON CONFLICT UPDATE 可能没有清除软删除标记",
            "suggestion": "确保设置 deleted_at = NULL, deleted_by = NULL",
            "severity": "HIGH",
            "check_avoided_by": r"deleted_at\s*=\s*NULL"
        },
        {
            "pattern": r'execute\(.*\$.*\)',
            "message": "⚠️  发现字符串拼接 SQL",
            "suggestion": "使用参数化查询防止 SQL 注入",
            "severity": "CRITICAL"
        }
    ],
    "error_handling": [
        {
            "pattern": r'\.unwrap\(\)',
            "message": "⚠️  使用 unwrap() 可能导致 panic",
            "suggestion": "使用 ? 或 match 处理错误",
            "severity": "MEDIUM"
        },
        {
            "pattern": r'\.expect\("[^"]*"\)',
            "message": "⚠️  使用 expect() 可能导致 panic",
            "suggestion": "使用 ? 或 match 处理错误",
            "severity": "MEDIUM"
        }
    ],
    "security": [
        {
            "pattern": r'\.starts_with\("/"\)|\.starts_with\(\\\'\\\'\\\'\)',
            "message": "⚠️  路径验证可能不完整",
            "suggestion": "使用 canonicalize 或安全的路径拼接方法",
            "severity": "MEDIUM"
        }
    ]
}


def check_file(filepath: Path) -> list:
    """检查单个文件"""
    issues = []

    try:
        content = filepath.read_text(encoding='utf-8')
    except Exception as e:
        return [{"line": 0, "message": f"无法读取文件: {e}", "severity": "ERROR"}]

    lines = content.split('\n')

    for category, patterns in PATTERNS.items():
        for pattern_def in patterns:
            pattern = pattern_def["pattern"]

            # 检查是否有避免该问题的代码
            if "check_avoided_by" in pattern_def:
                if re.search(pattern_def["check_avoided_by"], content):
                    continue

            for line_num, line in enumerate(lines, 1):
                if re.search(pattern, line):
                    # 跳过注释行
                    stripped = line.strip()
                    if stripped.startswith('//') or stripped.startswith('*'):
                        continue

                    issues.append({
                        "line": line_num,
                        "category": category,
                        "message": pattern_def["message"],
                        "suggestion": pattern_def["suggestion"],
                        "severity": pattern_def["severity"],
                        "code": line.strip()[:80]
                    })

    return issues


def print_report(filepath: Path, issues: list):
    """打印审查报告"""
    print(f"\n🔍 审查文件: {filepath}")
    print("=" * 60)

    if not issues:
        print("✅ 未发现明显问题")
        return

    # 按严重程度分组
    severity_order = ["CRITICAL", "HIGH", "MEDIUM", "LOW"]

    for severity in severity_order:
        severity_issues = [i for i in issues if i.get("severity") == severity]
        if severity_issues:
            emoji = "🚨" if severity == "CRITICAL" else "⚠️" if severity == "HIGH" else "💡"
            print(f"\n{emoji} {severity} ({len(severity_issues)} 项)")
            print("-" * 60)

            for issue in severity_issues:
                print(f"  行 {issue['line']}: {issue['message']}")
                print(f"    代码: {issue['code']}")
                print(f"    建议: {issue['suggestion']}")
                print()

    # 统计
    print("=" * 60)
    critical = len([i for i in issues if i.get("severity") == "CRITICAL"])
    high = len([i for i in issues if i.get("severity") == "HIGH"])

    if critical > 0:
        print(f"🚨 发现 {critical} 个 CRITICAL 问题，建议修复后再提交")
    elif high > 0:
        print(f"⚠️  发现 {high} 个 HIGH 问题，建议检查")
    else:
        print(f"💡 发现 {len(issues)} 个建议项")


def main():
    if len(sys.argv) < 2:
        print("用法: python claude-review.py <文件路径>")
        print("例如: python claude-review.py src/main.rs")
        sys.exit(1)

    filepath = Path(sys.argv[1])

    if not filepath.exists():
        print(f"错误: 文件不存在: {filepath}")
        sys.exit(1)

    issues = check_file(filepath)
    print_report(filepath, issues)

    # 如果有 CRITICAL 问题，返回非零退出码
    critical_count = len([i for i in issues if i.get("severity") == "CRITICAL"])
    if critical_count > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
