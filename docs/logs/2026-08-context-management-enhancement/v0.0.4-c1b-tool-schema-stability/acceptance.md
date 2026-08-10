# C1b Acceptance

## 自动验收

1. 运行 T4，确认同工具集在重复调用、反向注册和独立重建时完整 schema 字节一致。
2. 确认 CORE 连续前缀和 DEFERRED 后缀均按工具名排序。
3. 确认 nested object 键规范化且 array 顺序不被重排。
4. 运行受影响 crate 测试、clippy、CLI smoke 和工作区三门。
5. 检查提交差异，确认 provider `apply_cache_control` 未变化。

## 产品观察点

- 相同 mask/phase/MCP/custom 工具集合的连续 provider 请求具有相同 `tools` 数组字节。
- MCP 或 custom tool 名即使字典序早于内置工具，也不会插入 CORE 连续前缀。
- 合法的工具集合、名称、描述或参数 schema 内容变化仍会改变请求字节。

## 完成定义

C1b 只在 T4、受影响 crate 验证、CLI smoke 和工作区三门全部通过后完成；缓存命中趋势
与 cache-control 锚点调整不属于本切片。
