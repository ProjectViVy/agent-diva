# Acceptance

## 用户可见

1. 长任务里工具（例如 `exec`）成功后，不应再稳定出现「已完成 N 个工具调用……但模型未返回文字总结」。
2. 正常情况第三次采样应给出模型自己的文字总结。
3. 若仍无正文，最后一层仍是原来的中文机械兜底；日志里应有 `empty text after tools; classifying upstream follow-up` 以及 `OutputTruncated` / `InputPressure` / `EmptyStop`。

## 步骤

1. 重启 gateway。
2. 在原先会空总结的会话再发一次同类长任务。
3. 观察是否出现模型总结；打开日志确认分类，而不是只有兜底句。

## 不验收

- 从 provider `/models` 拉取 `context_length`。
- 思考过程作为用户可见总结。
- 所有轮次输出预算从 4096 全局上调。
