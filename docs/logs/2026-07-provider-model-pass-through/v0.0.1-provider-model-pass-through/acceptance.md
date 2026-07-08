# Acceptance

1. Configure DeepSeek with `model = deepseek-chat`; outbound requests keep `model` as `deepseek-chat`.
2. Configure StepFun direct endpoint with `provider_name = stepfun`, `api_base = https://api.stepfun.com/step_plan/v1`, and `model = step-3.7-flash`; outbound requests keep `model` as `step-3.7-flash`.
3. Configure a gateway-prefixed model such as `openrouter/deepseek/deepseek-chat`; outbound requests keep the full prefixed string.
4. Configure an unknown custom OpenAI-compatible provider with a raw model string; outbound requests keep that raw model string.
