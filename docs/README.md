# agent-diva-pro 文档目录

本目录包含 agent-diva-pro 项目的所有文档，按功能和用途分类组织。

## 目录结构

### 📁 architecture/
- **内容**: 项目架构设计文档
- **用途**: 记录系统架构、技术选型和设计决策

### 📁 design-notes/
- **内容**: 设计思考和概念探索文档
- **用途**: 记录项目的设计理念、架构想法和概念验证
- **文档**:
  - `creative-workbench-design.md` - Diva 工作台设计创意
  - `autonomous-activity-thoughts.md` - 自主活动架构想法

### 📁 dev/
- **内容**: 开发相关文档
- **用途**: 记录开发过程、技术决策和演进路径
- **子目录**:
  - `evo-diva/` - EVO-DIVA 活跃文档集
  - `past/` - 历史文档
  - `archive(old-docs-dont-read-me)/` - 归档文档

### 📁 logs/
- **内容**: 开发日志和迭代记录
- **用途**: 记录每个迭代的变更、验证和发布信息

### 📁 papers/
- **内容**: 学术论文参考集
- **用途**: 支持项目理论设计和架构决策的学术论文
- **论文数量**: 7 篇核心论文
- **备份**: 从 `/Users/mastwet/Desktop/morediva/papers/` 备份

### 📁 prd-report-system/
- **内容**: PRD 报告系统文档
- **用途**: 记录产品需求文档和报告系统设计

### 📁 prds/
- **内容**: 产品需求文档
- **用途**: 记录各模块的产品需求和功能规格

### 📁 prompts/
- **内容**: 代码清理提示文件
- **用途**: 记录代码清理任务的具体要求和步骤
- **文档**:
  - `main.md` - 主分支清理提示
  - `pro.md` - pro 分支清理提示
  - `sandbox.md` - sandbox 分支清理提示
  - `selfinprove.md` - 自我改进分支清理提示

### 📁 reports/
- **内容**: 代码清理报告
- **用途**: 记录代码整理和优化工作的结果
- **文档**:
  - `main.md` - 主分支清理报告
  - `pro.md` - pro 分支清理报告
  - `sandbox.md` - sandbox 分支清理报告
  - `selfinprove.md` - 自我改进分支清理报告

### 📁 research/
- **内容**: 技术研究报告
- **用途**: 记录技术调研、对比分析和集成方案
- **文档数量**: 13 篇研究报告
- **主要主题**:
  - Alife 框架研究
  - Hermes 框架研究
  - 后台任务队列管线设计
  - Harness Engineering 三方对比

### 📁 resources/
- **内容**: 资源文件
- **用途**: 存放项目相关的资源文件

## 文档整理说明

本次整理工作将散落在项目根目录的文档统一移动到 `agent-diva-pro/docs/` 目录下，并按功能分类：

### 移动的文档
1. **后台任务队列管线设计研究报告.md** → `research/`
2. **_clean_reports/*.md** → `reports/`
3. **_clean_prompts/*.md** → `prompts/`
4. **00-创意工作台设计/README.md** → `design-notes/creative-workbench-design.md`
5. **papers/** → `papers/`（备份副本）

### 删除的空目录
- `_clean_reports/`
- `_clean_prompts/`
- `00-创意工作台设计/`

## 使用说明

1. **新开发者**: 从 `dev/evo-diva/README.md` 开始了解项目文档结构
2. **架构设计**: 查看 `architecture/` 目录
3. **技术调研**: 查看 `research/` 目录
4. **产品需求**: 查看 `prds/` 目录
5. **开发日志**: 查看 `logs/` 目录
6. **学术参考**: 查看 `papers/` 目录

## 备份说明

- 论文备份位于 `papers/` 目录，原始文件位于 `/Users/mastwet/Desktop/morediva/papers/`
- 所有文档已按功能分类，便于跨设备开发时快速定位