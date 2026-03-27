# Acceptance

- Run `agent-diva --config <config> provider list --json`.
- Confirm the OpenAI entry contains both `provider: "openai"` and `name: "openai"`.
- Confirm `default_model` remains `openai/gpt-4o`.
- Confirm the command exits successfully and no existing provider JSON consumers regress.
