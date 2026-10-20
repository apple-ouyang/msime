# Agent Note: 把二三候选接进共享运行时并让 macOS 可用

Status: proposed

## Problem

#6501 要在组字时用 `;` 选当前页第 2 个候选、`'` 选第 3 个。共享偏好 `second_third_candidate { enabled, keys }` 和共享 TypeScript 类型早已存在（`crates/client-core/src/preferences.rs:1098-1103`、`packages/ui/src/core/preferences-model.ts:81`），默认关闭、关闭时不写进文档（`crates/client-core/src/preferences/tests.rs:2274`）。目前只有 Windows 接入（[用分号和单引号选第二、第三个候选](../../implemented/feature/2026-10-09-second-third-candidate-keys.md)），macOS 侧的现状是：

- 共享 React 设置页不渲染这一项；`packages/ui/src/core/preferences-model.ts:81` 的注释写明「目前只有 Windows 接入，本页不显示这一项」。
- `platforms/macos/tests/settings/preference_coverage.py:47` 把它列进 `NOT_APPLICABLE`，理由正是「只有 Windows 的 TIP 和 Server 路由这两个键，共享页不显示开关，macOS 用户没法打开一个宿主会忽略的东西」。

2026-10-21 核实了 macOS 运行时是否支持这个偏好，结论是**运行时也不支持**：

- **引擎**：`crates/engine/src/session/input.rs` 的 `handle_candidate_key`（986-1016）只把 `b'1'..=b'9'` 映射成候选序号，其他字符一律 `unhandled`。`'` 在引擎里是拼音音节分隔符（`SchemeKey::Apostrophe`，373），`;` 是微软双拼 `ing` 的韵母（340-341），都不是选词键。
- **共享运行时**：`crates/input-runtime/src/runtime.rs` 的 `candidate_shortcut`（639-646）是选词键的统一拦截点，但只查 `wubi_candidate_selection`（`wubi_candidate_shortcut`，648-671）；通用 `second_third_candidate` 没有分支。
- **macOS 宿主**：`platforms/macos/src/input/InputController.mm` 只把物理数字键映射到候选（`PhysicalCandidateDigitSlot` → `selectGeneration:index:`，6334、6352-6373），`;` 和 `'` 走普通字符路线 `typeASCII:`（6708-6713）→ `shared/apple/MSIMEClientSession.mm:660` → `msime_client_character`（`crates/host-api/src/ffi/input.rs:181`）→ `Action::Character`。`platforms/macos/src/input/InputControllerPhysicalKeys.h` 里没有二三候选的物理键映射。

关键点：macOS 宿主**已经**把 `;`/`'` 作为 `Action::Character` 送进共享运行时，而 `candidate_shortcut` 正是为这类选词键准备的拦截点；缺的只是通用偏好那一支。换句话说，这不是「宿主完全碰不到这两个键」，而是「运行时没有按通用偏好解释它们」。

## Proposal

在**共享运行时**接线，而不是照抄 Windows 的宿主级策略。

1. `crates/input-runtime/src/runtime.rs`
   - 在 `wubi_candidate_selection` 字段旁（105-106）加 `second_third_candidate: SecondThirdCandidatePreferences`，并加 `set_second_third_candidate` / `second_third_candidate()`（对照 625-636）。
   - 把 `candidate_shortcut`（639）推广成通用入口：先按现有的 `wubi_candidate_shortcut` 走五笔，再按新偏好把 `;` 映射成当前页 slot 1、`'` 映射成 slot 2，返回 `Action::Select(CandidateId)`。边界与 `wubi_candidate_shortcut` 一致：`snapshot_valid`、`editing_text` 非空、`local_mode == "none"`、非 `dedicated_english`、方案与拼写符号排除（见下 1c）。键位名只认枚举里唯一的 `semicolon_quote`。
   - 建议把「哪些方案/模式不接管」抽成一个小函数，照 Windows 的排除表：日文、由宿主自己组字的方案（韩文、注音、越南文、藏文）、英文模式、网址/V 模式的拼写符号、微软双拼声母后的韵母 `ing`（`self.profile.uses_semicolon_key()` 已能判断）。
2. 偏好下发：`crates/host-api/src/lib.rs:1036` 旁加 `self.runtime.set_second_third_candidate(requested_preferences.second_third_candidate)`；`crates/host-api/src/ffi/session.rs:119` 旁同样把 applied 偏好转给 runtime。
3. 共享设置页：`packages/ui/src/settings/input-shared-settings-section.tsx` 的「选词与翻页」组（185）加一行开关，绑定 `preferences.second_third_candidate`，写 `{ enabled, keys: "semicolon_quote" }`；同步更新 `packages/ui/src/core/preferences-model.ts:81` 的注释。
4. 覆盖表：`platforms/macos/tests/settings/preference_coverage.py:47` 删掉 `second_third_candidate` 条目（脚本会强制：宿主开始读它后条目必须移除）。

## 需要维护者拍板的点

1. **`'` 与音节分隔符 / 中文引号冲突。** Windows 开启后组字中的 `'` 不再是音节分隔符，这是用户主动选择的取舍。macOS 上 `'` 还牵动引号成对补全（`MSIMEPunctuationPairs` 含 `‘`/`’`，`InputController.mm:756-765`），打开开关后中文引号输入会受影响。是否照 Windows 一样让开关优先，需要拍板。
2. **`;` 与微软双拼 `ing` 冲突。** Windows 的规则是不接管「微软双拼声母后的韵母 ing」。共享运行时能拿到 `scheme` 与 `uses_semicolon_key()`，建议照抄这条排除；若维护者认为 macOS 用户预期不同，需要明确。
3. **设置页可见范围。** Linux、HarmonyOS 也承载共享 React 设置页并驱动同一 runtime，这一行会在那里出现并生效；Windows 用原生 WinUI 设置页。是否需要新的 host capability 来门控这一行，或直接全平台放开，需要拍板。

## Alternatives considered

- **照抄 Windows，在 macOS 宿主 `InputController.mm` 里判断物理键。** 最强理由是它与 `platforms/windows/common/SecondThirdCandidatePolicy.h` 一一对应，评审熟悉，宿主也最清楚物理键位与布局。但 macOS 宿主已经把这两个键当普通字符送进共享运行时，在宿主层再判断会多复制一份「方案 / 本地模式 / 拼写符号」的排除逻辑；Linux、HarmonyOS 等同样驱动这个 runtime 的宿主之后还要各写一遍。共享运行时里已有 `candidate_shortcut` 这个现成拦截点，五笔的二三候选就是这么做的。
- **下沉到引擎，让 `handle_candidate_key` 接受 `;`/`'`。** 最强理由是改动最集中，所有宿主自动受益。但引擎收到的是字符而不是物理键，`'` 在引擎里已经是音节分隔符、`;` 是微软双拼的 `ing`，改这两个字符的语义会直接波及组字本身；[2026-10-09 的笔记](../../implemented/feature/2026-10-09-second-third-candidate-keys.md)也记录了 Windows 刻意不把判断放进引擎。共享运行时的 `candidate_shortcut` 是在引擎之上、宿主之下的一层，不属于当时被否掉的那条路。
- **不做。** 最强理由是 `wubi_candidate_selection` 已经覆盖五笔用户的快捷选词，而通用偏好只有 Windows 在用。但 #6501 的诉求是全拼/双拼用户，Windows 已用通用 `second_third_candidate`；不做会让同一个偏好在不同平台行为不一致。

## Acceptance criteria

- `cargo test -p msime-input-runtime -p msime-host-api` 全绿，并新增至少一个**先失败后通过**的 runtime 单测：`;` 选中第 2、`'` 选中第 3；关闭时两者走原标点路径；当前页候选不足时不被吞掉；被排除的方案/模式（例如微软双拼的 `ing`）仍作输入。
- `platforms/macos/tests/settings/preference_coverage.py` 不再把 `second_third_candidate` 列为 `NOT_APPLICABLE` 并通过。
- 共享设置页改动后 `pnpm --filter @msime/desktop test` 全绿，`pnpm format:check` 通过。
- 关闭开关时 macOS 行为逐字节不变（`;`、`'` 仍作为标点/音节分隔符）。
- 在 macOS 宿主路径上确认无需改 `InputController.mm`：若现有测试基建能覆盖 `typeASCII:` 集成路径，补一条断言开启后 `;`/`'` 选中第 2/3 个候选。

## Risks

- 打开后 `'` 不再是音节分隔符，并与 macOS 的引号成对补全交互；这是 Windows 已接受的取舍，但 macOS 的引号行为更复杂，需要维护者确认。
- `;` 在微软双拼里是 `ing`；排除规则若与 Windows 不一致，会造成同一偏好在两平台行为分叉。
- 接入共享运行时后，所有驱动同一 runtime 的宿主会同时生效；本次只核实了 macOS，其余宿主是否都配置并展示这个偏好需要逐一确认。
- 回滚成本低：默认关闭且不写进文档，删掉 runtime 分支与设置页那一行即可，无数据迁移。
