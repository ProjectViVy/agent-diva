# v0.2.2 Evolution 视图隐藏安装类技能 — Summary

## 背景

v0.2.1 落地 `evolution_managed` 区分后,用户要求:安装的技能(市场/手动上传)不应出现
在 Evolution 视图。此前 `EvolutionView.vue` 的 `loadSkills()` 渲染 `getSkills()` 全量
结果,安装类与内置技能都会出现在 Evolution 的 Skill 列表,与“Evolution 只管自己生成的
技能”的产品逻辑不符。

## 改动(纯 GUI)

### `src/components/EvolutionView.vue`

- `loadSkills()`:仅保留 `evolution_managed === true` 的技能;选中项存在性检查同步基于
  过滤后列表,避免被隐藏技能残留选中态。
- 空态文案:“尚无可见 Skill。” → “尚无 Evolution 管理的 Skill。”

### `src/components/EvolutionView.test.ts`

- `skill()` 夹具增加 `evolution_managed` 参数(默认 `true`)。
- 新增用例:安装类技能(`evolution_managed: false` 或字段缺省)不出现在列表,
  Evolution 托管技能正常显示。

## 行为说明

- 内置技能(`evolution_managed` 恒为 false)一并隐藏——Evolution 视图从此只列
  Evolution 托管技能。
- 新建提案时若 slug 命中被隐藏技能,`createBaseHash` 回落 `'0'`,后端 CAS 会正确判
  Stale,无数据风险。
- 安装类技能的管理入口仍在 设置 → 技能管理 → 已安装(v0.2.1 的删除按钮)。

## 影响范围

仅 GUI 渲染过滤;网关与 Tauri 层无改动。
