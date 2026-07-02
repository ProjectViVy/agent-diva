# Acceptance

## User Steps

1. Start the GUI with welcome wizard visible.
2. Enter a DeepSeek API key in the wizard.
3. Finish the wizard without visiting Provider settings.
4. Send a GUI chat message.

## Expected Result

The runtime provider is `deepseek`, the model is `deepseek-chat`, and the API base is `https://api.deepseek.com/v1`. The chat request should use the newly entered key immediately without requiring another provider settings save.
