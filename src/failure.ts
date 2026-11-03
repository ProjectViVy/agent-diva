export type Failure = { title: string; detail: string };

const unescape = (value: string) =>
  value.replace(/\\(["\\/nrt])/g, (_, char: string) =>
    char === "n" || char === "r" || char === "t" ? " " : char);

export function describeFailure(raw: string): Failure {
  const text = raw.replace(/^The model call failed:\s*/i, "").trim();
  if (!text) return { title: "任务失败", detail: "" };
  const providerFailure =
    /ProviderResponseError|provider_response|model_not_found|new_api_error|invalid_request_error|status [45]\d\d\b/i.test(text);
  const inner = /"message"\s*:\s*"((?:\\.|[^"\\])*)"/.exec(text);
  const status = /\bstatus ([45]\d\d)\b/i.exec(text);
  let detail = inner ? unescape(inner[1]) : text.replace(/^ProviderResponseError:\s*/i, "");
  if (status) detail = `HTTP ${status[1]} · ${detail}`;
  if (detail.length > 240) detail = `${detail.slice(0, 239)}…`;
  return { title: providerFailure ? "供应商错误" : "任务失败", detail };
}
