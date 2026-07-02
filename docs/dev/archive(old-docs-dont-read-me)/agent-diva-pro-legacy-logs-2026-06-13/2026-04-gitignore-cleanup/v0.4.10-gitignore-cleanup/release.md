# Release: v0.4.10-gitignore-cleanup

## 发布类型

维护性发布 (Maintenance Release) - 仅包含 `.gitignore` 文件更新。

## 发布方法

### 1. 创建 Git Tag

```bash
git tag -a v0.4.10-gitignore-cleanup -m "Enhance .gitignore with AI/IDE caches, BMad artifacts, and test workspace rules"
git push origin v0.4.10-gitignore-cleanup
```

### 2. 发布说明

本次发布为纯配置更新，不涉及代码变更。主要改进：

- 减少 `git status` 噪声（约 96MB+ 未跟踪文件）
- 统一 AI/IDE 工具缓存忽略规则
- 保护 skills 目录不被误忽略
- 添加 BMad 构建产物和测试工作区忽略规则

### 3. 回滚策略

如发现问题，可删除标签并回退到 v0.4.9：

```bash
git tag -d v0.4.10-gitignore-cleanup
git push origin :refs/tags/v0.4.10-gitignore-cleanup
```

### 4. 用户影响

- **无运行时影响**: 仅影响版本控制配置
- **无需迁移**: 现有项目配置不受影响
- **建议同步**: 建议所有开发者拉取此更新以保持 `.gitignore` 一致性
