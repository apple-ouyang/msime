# Agent Note: 顿号自定义（`\` 与 `/` 输出「、」）

Status: proposed

## Problem

新架构的共享标点表只把 `\` 映射成顿号「、」，`/` 既不映射，也没有任何偏好能开关这两个键。Windows v0.11.1 有「顿号自定义」：用户选中文标点下用哪些键输出 `、`。这一项没有搬进新架构——新架构 Windows 宿主（`platforms/windows`）同样只有 `\`，所以它不是 macOS 单平台的缺口，而是四个平台共有的能力缺口。

只读审计确认的位置：

- 引擎固定表：`crates/engine/src/punctuation.rs` 的 `SIMPLE`（第 4-20 行）只有 `(b'\\', "、")`，没有 `/`；契约 `shared/contracts/punctuation/policy.json` 的 `simple` 与生成的 `shared/contracts/punctuation/policy.h` 相同。
- 没有偏好：`crates/client-core/src/preferences.rs` 的标点族只有 `chinese_punctuation`、`paired_punctuation`、`punctuation_lock`、`caps_lock_ascii_punctuation`、`quote_style`、`bracket_style` 六项（`preferences.rs:1053-1092`）。
- Windows 来源：`target/ref/msime-windows`（v0.11.1，提交 `dd018f7`）的 `installer/default_config/config.default.toml:284-286`、`engine/contracts/enumeration_comma.h`、`windows/src/Composition/CompositionProcessorEngine_Modes.cpp:69-89`、`server/src/config/ime_config.cpp:701-708`、`server/src/config/ime_config_keys.cpp:279-322`、`server/src/config/ime_config_template.cpp:111-115`、`server/src/settings/native/native_page_input.cpp:1872-1881`。

## Windows 规格（v0.11.1，已确证）

| 项 | 值 / 行为 | 证据 |
| --- | --- | --- |
| 可调项 | 两个独立布尔：`[input] enumeration_comma_backslash`、`enumeration_comma_slash` | `config.default.toml:284-286` |
| 出厂模板缺省 | 两个都 `true`（新装） | `config.default.toml:285-286` |
| 运行时缺键缺省 | 两个都 `true` | `ime_config.cpp:701-702` |
| 升级用户 | 旧配置没有 `enumeration_comma_slash` 时并成 `false`，保持「只有 `\`」；已有该键则以用户值为准 | `ime_config_template.cpp:111-115`；`server/tests/src/test_config_template_merge.cpp:36-57` |
| 至少留一个 | setter 拒绝关掉最后一个；手改成两个都关时按两个都开读 | `ime_config_keys.cpp:287-297, 306-314`；`ime_config.cpp:704-708` |
| 生效语义 | 勾选的键在中文标点下出 `、`；没勾的键不是中文标点，按原样输出 | `CompositionProcessorEngine_Modes.cpp:69-89`（`ResolveEnumerationCommaKey`）、`:98-104`（`IsPunctuation`）、`:148-153`（`GetPunctuation`） |
| 与标点开关 | 只作用于中文标点模式；`IsPunctuation` 对没勾的键返回 FALSE，键交回普通字符路径 | 同上 |
| 与智能标点 | `、` 在空格转换名单里，按空格转回 ASCII | `config.default.toml:274` |
| IPC | Worker 帧 `EnumerationCommaKeysChanged`：载荷 `'1'` 只有 `\`，`'2'` 只有 `/`，`'3'` 两个都开；`'0'`/非法退回 `'1'`（旧 Server 的历史行为只有 `\`） | `enumeration_comma.h:8-29`；`windows/src/IPC/Ipc.h:285-290`；`ime_config_keys.cpp:318-322`；`server/src/window/ime_windows_candidate.cpp:468, 619-625` |
| 设置页 | 原生「顿号自定义」段，两个勾选 `\ 反斜杠` / `/ 斜杠`，说明「中文标点状态下用哪些键输出顿号「、」，至少保留一个；未勾选的键按原样输出」 | `native_page_input.cpp:1872-1881`；`settings_app.cpp:337-338, 683-686`；`native_settings_bindings.cpp:137-167`（`AtLeastOneChecks`） |

结论：Windows 给用户的是**两个键各自开/关**，至少留一个；缺省值分新装（两个都开）和升级（只有 `\`）；未勾选的键在中文标点下退回 ASCII 字面量。它不改变成对补全，也不改变智能标点总开关。

## Proposal

按 Windows 的语义补成共享能力，一处解析、各宿主只消费：

- **偏好**：`Preferences` 加 `enumeration_comma_backslash: bool`（缺省 `true`）与 `enumeration_comma_slash: bool`（缺省待拍板，见下）。等于缺省时不写进文档（`skip_serializing_if`），与 `quote_style`、`bracket_style` 一致。
- **至少留一个**：`crates/client-core/src/punctuation.rs` 的 `punctuation_marks(&Preferences)` 是唯一的偏好解析处，在这里把「两个都关」归一成「`\` 开」，与 Windows 手改配置的归一一致。
- **契约**：把 `/` 加进 `shared/contracts/punctuation/policy.json`，并把「按键可被偏好抑制」写进契约；`contractVersion` 从 2 升到 3（`generate.py`、`policy.h`、Rust `punctuation.rs` 与 C++ `punctuation_contract.cpp` 都钉死 2，需一并改）。具体表示见「拍板点 D1」。
- **引擎**：`PunctuationPolicy` 带两个布尔，未启用的键 `translate` 返回 `None`（键按普通字符交回宿主）；两个都关在解析处已归一，引擎不会看到全关。
- **host-api 与 View**：`PunctuationMarks`（client-core）加两个布尔，`msime_client_view` 的 `punctuation_marks` 报告 `{"enumeration_comma_backslash": true, "enumeration_comma_slash": false}`，宿主画键面、做空格反查和英文模式时读它，不自己解析偏好。
- **设置页**：共享 React 页在「标点」组加「顿号自定义」两个开关，至少留一个；Windows、iOS、Android、HarmonyOS 的原生页同步。
- **账号同步**：两个键上传/应用；msime-cloud 的 `internal/account/preferences_fields.json` 要先加两个布尔字段。

## 为什么这一轮停手

按契约 README 的要求把这条设置做对，改动面不是「加偏好 + 改表 + 加设置行」：

1. **要动契约。** `policy.json` 的 `simple` 是一张无条件定长表，契约里没有「同一个键可被偏好关掉」的概念。加 `/` 是新键，必须进契约；而「可抑制」要么改 `simple` 的语义，要么新增一段，两种都要升 `contractVersion` 并重跑 `generate.py`。
2. **契约 README 明确要求宿主本地表跟着改。** `shared/contracts/punctuation/README.md:11` 写「加字或改字时这些表都要跟着改，否则会出现错位」。要跟的表分布在 6 个宿主：`shared/input/EnglishModeOutput.h:41`（macOS/Linux 英文模式）、macOS `platforms/macos/src/input/InputController.mm:957-978`、Linux `platforms/linux/src/core/SmartPunctuationSpace.h:20-28`、iOS `platforms/ios/KeyboardExtension/Sources/input/KeyboardPunctuationContext.swift:4-9`、Android `platforms/android/java/app/msime/android/keyboard/ChineseSymbolFaces.java`、HarmonyOS `platforms/harmony/keyboard/input/ChineseSymbolFaces.ts`。
3. **新架构 Windows 还要加 Server→TIP 帧。** 现有 `PunctuationMarksChanged = 31` 只带 `q<0|1>b<0|1>`（`shared/contracts/windows_ipc.h:333`）；新帧要追加在末尾（不能插队），`platforms/windows/src/ipc/ReplyCodec.cpp:236-300` 与 TSF `platforms/windows/tsf/Global/Globals.cpp:208`（`PunctuationTable` 的 `{L'\\', L"、"}`）一起改，还要保证旧 TIP 丢掉新帧后仍是「只有 `\`」。
4. **设置页不是一个。** 共享 React 页只覆盖 macOS/Linux/HarmonyOS；Windows 是原生设置页，iOS、Android、HarmonyOS 手机各有原生标点页。
5. **有一个真正的新架构冲突。** `/` 在新引擎里是本地命令模式的触发键（`crates/engine/src/session/input.rs:1736-1751`，`b'/' => (LocalInputMode::Command, options.command)`）。`enumeration_comma_slash` 打开会让 `/` 出顿号，命令模式就打不开；两者互斥，必须先拍板谁优先。这正是这项「v0.11.1 遗留设置没搬」的根因。
6. **账号同步要动外部字段表。** msime-cloud 没加字段之前，两个键只能本地生效。

六个宿主 + 契约升版 + 一个行为冲突，已经越过「加偏好 + 改表 + 加设置行」。按任务约定，本轮只交本拟案，不写代码。

## 要动的文件与函数

契约与引擎：

- `shared/contracts/punctuation/policy.json`、`generate.py`、`policy.h`（生成）、`README.md`、`shared/contracts/tests/punctuation_contract.cpp`、`scripts/test-punctuation-contract.py`。
- `crates/engine/src/punctuation.rs`：`SIMPLE`、`PunctuationMarks`、`PunctuationPolicy::translate`、`set_marks`，以及测试 `matches_the_shared_contract`、`contract_table`（`SIMPLE.len() == 15` 的断言要改为新行数）。
- `crates/engine/src/session/options.rs`（`SessionOptions::punctuation_marks`）、`crates/engine/src/session/input.rs`（`session.punctuation.set_marks`）、`crates/engine/src/host/options.rs`（`EngineOptions` 加两个字段并在 `session_options` 里映射）。

共享偏好与桥接：

- `crates/client-core/src/preferences.rs`（两个新字段 + serde 缺省 + `Default`）、`crates/client-core/src/punctuation.rs`（`PunctuationMarks` 与 `punctuation_marks`）、`crates/client-core/src/preferences/tests.rs`。
- `crates/input-runtime/src/types.rs`、`crates/input-runtime/src/runtime.rs`（`set_punctuation_marks`、View 序列化）、`crates/input-runtime/src/tests.rs`。
- `crates/host-api/src/lib.rs`（`apply_punctuation_marks`、`ffi/session.rs`）、`crates/host-api/include/msime_client.h`（View 字段说明）、`crates/host-api/src/tests.rs`。

设置页：

- `packages/ui/src/settings/punctuation-preferences.ts`、`punctuation-options.ts`、`punctuation-section.tsx`。
- `apps/desktop/tests/settings/punctuation-control-state.test.ts`、`punctuation-section.test.tsx`，以及 HarmonyOS 手机页的用例。

宿主本地表（契约 README 的清单）：

- `shared/input/EnglishModeOutput.h`：`english_mode_chinese_punctuation` 的 `case '\\'`、`PunctuationMarks`、`punctuation_marks_from_view`。
- macOS：`platforms/macos/src/input/InputController.mm` 的 `MSIMEPunctuationPairs`、`MSIMEASCIIForSmartChinesePunctuation`、`MSIMEViewPunctuationMarks`、英文模式分支；`platforms/macos/src/core/PairedPunctuation.h`。
- Linux：`platforms/linux/src/core/SmartPunctuationSpace.h`、`platforms/linux/src/candidates/PairedPunctuation.h`。
- iOS：`KeyboardPunctuationContext.swift`、`KeyboardViewController.swift`、`KeyboardLayoutTables.swift`。
- Android：`keyboard/ChineseSymbolFaces.java`、`keyboard/PairedPunctuationPolicy.java`、`keyboard/KeyboardLayout.java`。
- HarmonyOS：`keyboard/input/ChineseSymbolFaces.ts`、`keyboard/input/SmartPunctuationSpacePolicy.ts`、`keyboard/input/PairedPunctuationPolicy.ts`、`keyboard/input/NumberSymbolLayout.ts`。
- Windows 新架构：`platforms/windows/tsf/Global/Globals.cpp`、`platforms/windows/tsf/Composition/Composition.cpp`（`SmartPunctuationAsciiFor`）、`platforms/windows/src/ipc/ReplyCodec.cpp`、`shared/contracts/windows_ipc.h`、`platforms/windows/src/window/ime_windows_candidate.cpp`、原生设置页。

其它：

- `crates/client-services/src/account/settings_sync.rs`（含 `settings_sync/tests.rs`）与鸿蒙 `account::harmony_settings`；msime-cloud 的 `internal/account/preferences_fields.json`。
- `crates/input-runtime/examples/punctuation_table.rs` 的样例表。

## 拍板点

- **D1 契约怎么表达「可抑制的键」。** (a) 把 `/` 加进 `simple`，偏好只做运行时抑制；(b) 新增顶层段（如 `enumerationComma`）列出这两个键和 `、`，`simple` 保持无条件。倾向 (b)：`simple` 的语义仍是「无条件映射」，别的宿主读 `policy.h` 时不会把 `/` 当成永远的中文标点；代价是 `generate.py` 要新增段与查找函数、`version` 升 3。
- **D2 `enumeration_comma_slash` 的缺省值。** Windows 新装是 `true`、升级是 `false`。共享仓库的现有用户相当于升级用户，缺省 `false` 能让现有行为逐字不变；缺省 `true` 则所有平台上 `/` 都不再能开命令模式。倾向 `false`，与「缺省值下行为不变」的惯例一致；要跟随 Windows 新装就选 `true`，但必须同时解决 D3。
- **D3 `/` 命令模式与 `/` 出顿号的优先级。** (a) 保留命令模式优先，`enumeration_comma_slash` 只在方案不打开命令模式时生效；(b) 勾选出顿号就完全占用 `/`，命令模式让位；(c) 加超时/前缀条件区分。倾向 (b) 并让设置说明写出代价——把 `/` 用作顿号本来就是要放弃命令模式；不建议 (c)（无法稳定判定）。
- **D4 `、` 反查回哪个键。** 现状已不一致：macOS 空格转换把 `、` 转 `/`（`InputController.mm:965`），Linux `kSmartPunctuationSpaceMap` 把 `、` 对 `/`（`SmartPunctuationSpace.h:22`），iOS 符号面板把 `、` 对 `\`（`KeyboardPunctuationContext.swift:5`）。规则要定成「只看当前启用的键；两个都开时用 `\`」还是别的，并在各宿主测试里钉死。
- **D5 设置页控件形态。** 两个独立开关 + 「至少留一个」守卫（与 Windows 一致），还是单个三值选择（只有 `\` / 只有 `/` / 两个）。倾向两个开关，文案写出键位。
- **D6 Windows 帧。** 新帧 `EnumerationCommaKeysChanged`（载荷 `'1'/'2'/'3'`，与来源一致）追加在 `windows_ipc.h` 末尾，还是把两个位并进 `PunctuationMarksChanged` 的载荷。倾向新帧 + 追加，保持旧 TIP 兼容。
- **D7 至少留一个由谁守。** Windows 在三处各守一次（setter、读配置归一、设置页控件）。共享侧至少要在 `punctuation_marks()` 归一；设置页守卫是否加，以及引擎要不要再兜一层，要定。
- **D8 账号同步。** 两个布尔要不要上传；上传就要先在 msime-cloud 加字段，否则只本地生效。

## 历史检索

按 `顿号`、`enumeration`、`标点`、`punctuation` 检索 proposed + implemented + rejected，分类如下：

- [引号样式与方头括号样式](../../implemented/feature/2026-10-13-punctuation-mark-styles.md)：**部分重叠**。同一份 `policy.json`、同一批手抄表、同一族偏好；那篇持有 `styles`/`quote_style`/`bracket_style` 的行为，本篇只持有「按键可开关」这一条，互链保留。它的「手抄表清单」一节正是本篇要跟的清单。
- [共享标点偏好的叶子类型](../../implemented/simplification/2026-10-15-punctuation-preferences.md)：**部分重叠**。持有 React 偏好叶子类型与字段归属；新增两个字段走同一处，互链保留。
- [大写锁定时使用英文标点](../../implemented/feature/2026-10-09-caps-lock-ascii-punctuation.md)：**部分重叠**。同为「标点族新偏好」，它记录了「先只上 macOS、其它平台待跟进」的先例；本篇的难处正是「动了契约就不能只上 macOS」，互链保留。
- [拼音标点保留原始输入](../../implemented/bug-fix/2026-10-14-punctuation-raw-commit.md)：**无关**。那是未完成拼音的回退路径，不涉及标点表。
- 其余命中（手机逗号底行、符号面板、字母角标、九键等）与「顿号键位」无关，不逐一列出。

## Alternatives considered

- **只给 macOS 加、其它宿主「待跟进」。** 最强理由是仓库有先例（[大写锁定标点](../../implemented/feature/2026-10-09-caps-lock-ascii-punctuation.md) 就先只上了 macOS，并单列「平台接入情况」）。没采用：那次没动契约、没往表里加字，其它宿主只是少一个改道规则；本项要把新键 `/` 写进共享契约，宿主本地表不跟就会按契约 README 说的漂移，属于「半搬」而不是「增量搬」。
- **引擎里另起一张 `/`/`\` 表，完全不碰契约。** 最强理由是不用升版本、改动最小，也绕开 C++ 契约测试。没采用：契约 README 写明 `policy.json` 是唯一真源、`Nobody copies the mapping into a platform-specific table`；`matches_the_shared_contract` 的存在就是为了防这件事，另起一张表等于把真源拆成两份。
- **只做 `/` → 、，不暴露偏好。** 最强理由是缺口描述里「`/` → 、也没有」，加一行就补上，最省事。没采用：会改变所有现有用户的 `/` 行为，还丢掉了 Windows「任一个可关」的能力；与「缺省值下行为不变」相反。
- **单个三值选择代替两个布尔（D5 的备选）。** 最强理由是一个控件天然互斥、不用写「至少留一个」的守卫，也省一次跨字段归一。没采用（倾向）：与 Windows 的两个勾选和 TOML 两个键对不上，偏好迁移和设置页都要另做映射。
- **`/` 命令模式与顿号共存（D3 的备选）。** 最强理由是两边功能都保留，用户不必取舍。没采用：没有稳定的判据能区分「用户要命令模式」和「用户要顿号」，加超时或前缀会把 `/` 的语义弄复杂；来源也没有这个做法。

## Acceptance criteria

- 契约：`policy.json` 与 `policy.h` 一致（`python3 shared/contracts/punctuation/generate.py --check`），`contractVersion` 升到 3；Rust `matches_the_shared_contract` 与 `shared/contracts/tests/punctuation_contract.cpp` 同步并通过。
- 引擎：`crates/engine/src/punctuation.rs` 有**先失败后通过**的测试，至少覆盖「未启用的键返回 `None`」「启用的键与旧行为逐字相同」「两个都关归一成 `\` 开」。
- `cargo test -p msime-engine` 全绿；`cargo fmt --all --check`；`cargo clippy -p msime-engine --all-targets --locked -- -D warnings`。
- host-api / input-runtime：View 的 `punctuation_marks` 报告两个新字段；偏好往返、缺键缺省、两个都关归一各有测试。
- 设置页：`pnpm --filter @msime/desktop test` 全绿，`pnpm format:check` 通过；桌面页与手机页各有一条「两个都关时保留最后勾选的那个」的用例。
- 各宿主：`、` ↔ 启用键的反查、英文模式中文标点、符号键面各有覆盖；Windows 帧载荷为 `'1'/'2'/'3'` 且旧 TIP 丢掉新帧后仍只出 `\`。
- 账号同步：msime-cloud 字段表加上两个布尔后，往返和跳过未知值有测试。
- 笔记：`pnpm run verify-notes` 通过。

## Risks

- **降级读不了文档。** `Preferences` 带 `deny_unknown_fields`，选过非默认值的用户回退到旧版本时会拒读整份文档；与 `quote_style`、`bracket_style`、`caps_lock_ascii_punctuation` 是同样的代价。
- **缺省值取舍两难。** `slash = false` 保证现有行为逐字不变，但和 Windows 新装的「两个都开」不一致；`slash = true` 则所有平台的 `/` 立刻要让出命令模式，是可见的行为变化。
- **「至少留一个」是跨字段不变量。** serde 表达不了，必须在偏好解析、设置页、引擎各守一次；漏一处就会出现「两个都关」的文档，之后各宿主对它的解释可能不同。
- **`、` 反查在宿主间已经不一致。** 本项会把 `/`、`\` 两条反查同时引入，若不先统一（D4），错位会从一处变三处。
- **契约语义变化的外溢。** 若按 D1(a) 把 `/` 放进 `simple`，任何直接读 `policy.h` 的消费者都会看到 `/` 成了无条件中文标点；按 D1(b) 新开一段则要同步 `generate.py` 的校验、查找函数和两处测试。
- **Windows 版本错配。** 新 Server 配旧 TIP 时，旧 TIP 丢掉新帧，`/` 仍出不来顿号；正式包同时替换两者，只有开发时手动换单个 DLL 才会碰到，需在实现时写进测试。
- **命令模式回归。** 共享 React 设置页覆盖的桌面三平台里，Linux/macOS 的 `/` 命令模式是常用入口；`slash` 缺省或设置文案写错，会让用户按 `/` 打不出命令又找不到原因。
