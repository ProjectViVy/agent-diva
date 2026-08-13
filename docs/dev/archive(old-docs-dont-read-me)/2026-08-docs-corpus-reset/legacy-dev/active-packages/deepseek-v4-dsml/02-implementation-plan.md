# 实现方案

`ProviderResponseProtocol::DeepseekV4Dsml` 选择严格解码器；流式内容先缓冲，完成解析后才发出文本、reasoning 和工具调用。
