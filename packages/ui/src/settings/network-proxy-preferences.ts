/** 用户内容请求的代理设置，对应 `crates/client-core/src/network.rs` 的 `NetworkPreferences`。 */
export type NetworkProxyPreferences = {
  /** `system` 跟随系统或环境变量里的代理；`none` 一律直连；`custom` 走下面的 HTTP 代理。 */
  proxy_mode?: "system" | "none" | "custom";
  /** `custom` 模式的 HTTP 代理地址，形如 `127.0.0.1:7890`；留空时按 `system` 处理。 */
  proxy_server?: string;
};

/** 与 Windows 正式版 `installer/default_config/config.default.toml` 的出厂值一致。 */
export const defaultNetworkProxy: Required<NetworkProxyPreferences> = {
  proxy_mode: "system",
  proxy_server: "",
};

/** 把一次修改并进草稿里的 `network`；缺省值先垫底，旧文档没有这一节时也能改。 */
export function updateNetworkProxy(
  current: NetworkProxyPreferences | undefined,
  patch: Partial<NetworkProxyPreferences>,
): Required<NetworkProxyPreferences> {
  return { ...defaultNetworkProxy, ...current, ...patch };
}

/** 代理地址的本地检查，与 Rust 的 `valid_proxy_server` 同口径：`host:port`，可带 `http://` 或 `https://` 前缀。留空表示不用自定义代理。 */
export function networkProxyServerIssue(value: string): string {
  const server = value.trim();
  if (server === "") return "";
  if (server.length > 512) return "代理地址过长。";
  // eslint-disable-next-line no-control-regex
  if (/[\u0000-\u001f\u007f]/.test(server) || server.includes("\\") || server.includes("#")) {
    return "代理地址不能包含控制字符、反斜杠或 # 片段。";
  }
  const candidate = server.includes("://") ? server : `http://${server}`;
  let url: URL;
  try {
    url = new URL(candidate);
  } catch {
    return "请填写 host:port 形式的代理地址，例如 127.0.0.1:7890。";
  }
  if (url.protocol !== "http:" && url.protocol !== "https:") {
    return "代理只支持 http:// 或 https:// 地址。";
  }
  if (url.username || url.password) return "代理地址不能带用户名或密码。";
  if (!url.port) return "请写出代理端口，例如 127.0.0.1:7890。";
  if (url.pathname !== "/" && url.pathname !== "") return "代理地址不能带路径。";
  if (url.search) return "代理地址不能带查询参数。";
  return "";
}
