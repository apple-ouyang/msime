import Foundation

/// macOS 云剪贴板的两条路（`docs/cloud-clipboard-relay.md`）：水杉账号，或用户自己部署的中继。偏好 `cloud_clipboard_relay` 里配置了中继就只走中继，不要求登录水杉账号，失败也不退回账号——否则用户以为在用自己的中继，内容却去了官方服务。
///
/// 输入法进程里的三个入口共用这一份选路：输入法菜单启动的 Tauri 云剪贴板面板与表情面板的「发到云剪贴板」（`BackendCloudClipboardProvider`），以及未登录时退回的原生云剪贴板窗口（`MacClipboardModel`）。设置应用里的 Tauri 面板自己按同一份偏好选路（`apps/desktop` 的 `cloud_clipboard_request`），走的是同一个 C 入口。
enum BackendCloudClipboardRoute: Sendable, Equatable {
  /// 水杉账号：绑定打开入口时的账号，每个动作前后都核对账号还在；令牌轮换和重新登录不作废。
  case account(userID: String)
  /// 用户自建的中继：不要求登录。`directory` 是输入法状态目录，偏好与中继密钥串 `cloud-clipboard-relay-keystore` 都在那里。
  case relay(directory: URL)

  /// 读共享偏好决定走哪条路。配置了中继时是 `.relay`；否则要求登录，没有登录是 nil，由调用方请用户登录。偏好读不了（`storage`）或中继配置不合规（`relay_not_configured`）时抛错：这两种情况都不拿账号路径顶上。
  static func selected(rust: BackendRustAccount) async throws -> BackendCloudClipboardRoute? {
    guard let directory = BackendDesktopSessionFile.standardDirectory else { throw BackendRustClient.Failure(code: "storage") }
    switch try await rust.cloudClipboardSource(preferencesDirectory: directory) {
    case "relay":
      return .relay(directory: directory)
    case "account":
      guard let user = try await rust.status() else { return nil }
      return .account(userID: user.id)
    default:
      throw BackendRustClient.Failure.invalidResponse
    }
  }

  /// 把一次云剪贴板动作交给这条路。每个动作都会联网，不要在界面线程或输入线程上调用。
  func request(_ rust: BackendRustAccount, _ action: [String: Any], deviceName: String) async throws -> Data {
    switch self {
    case .account(let userID):
      try await requireAccount(rust, userID)
      let data = try await rust.cloudClipboard(action, deviceName: deviceName)
      try await requireAccount(rust, userID)
      return data
    case .relay(let directory):
      return try await rust.relayCloudClipboard(action, preferencesDirectory: directory, keystoreDirectory: directory,
                                                deviceName: deviceName)
    }
  }

  /// 打开入口时的账号还登录着才继续：退出或换了号时是 `account_unauthorized`，界面请用户重新登录。C 上拿不到会话代次，只按 `user.id` 绑定：同一个账号的令牌轮换和重新登录不作废。
  private func requireAccount(_ rust: BackendRustAccount, _ userID: String) async throws {
    guard try await rust.status()?.id == userID else { throw BackendRustClient.Failure(code: "account_unauthorized") }
    try Task.checkCancellation()
  }
}
