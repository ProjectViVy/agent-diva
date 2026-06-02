# Release Summary: v0.4.10-gitignore-cleanup

## 变更概述

增强 `.gitignore` 文件，添加缺失的忽略规则来清理项目中的临时文件、构建产物和其他噪声数据。

## 变更范围

### 新增忽略规则

1. **AI/IDE 工具缓存** (约 96MB+):
   - `.agent/`、`.agents/` (保留 `skills/` 子目录)
   - `.codebuddy/`、`.gemini/`、`.iflow/`、`.trae/`
   - `.opencode/` (保留 `skills/` 子目录)

2. **Qoder IDE 优化**:
   - 将 `.qoder/` 改为选择性忽略，保留 `.qoder/skills/` 目录

3. **BMad 构建产物**:
   - `_bmad/`、`_bmad-output/`

4. **测试工作区**:
   - `tui-test/target/`、`tui-test/Cargo.lock`

5. **Windows  artifacts**:
   - `nul` (Windows 命令重定向产物)

6. **构建元数据**:
   - `/build.json` (与 `/build_utf8.json` 一起忽略)

## 影响分析

- **无破坏性变更**: 仅影响 `.gitignore` 文件
- **源码安全**: 所有 skills 目录 (`.cursor/skills/`、`.qoder/skills/` 等) 仍可被正确跟踪
- **构建不受影响**: 仅忽略构建产物，不影响源代码或构建配置

## 验证结果

- 所有新忽略规则通过 `git check-ignore -v` 验证
- 无已跟踪文件被新规则误忽略
- Skills 目录例外规则正确工作
