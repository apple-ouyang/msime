# Agent Note: 中英/全半角/标点三个切换热键可重绑

Status: proposed

## Problem

旧版 Windows 正式版（v0.11.1，`/tmp/msime-audit/msime-windows` @ `dd018f79`）有三个可重绑的输入状态切换热键，工厂值在 `installer/default_config/config.default.toml:427-429`，定义来自 `engine/contracts/input_state_shortcuts.h:21-23`：

| 键 | 出厂值 | 用途 |
| --- | --- | --- |
| `keybindings.toggle_english_candidates` | `"Ctrl+Shift+KeyE"` | 中英切换（英文候选模式） |
| `keybindings.toggle_full_width` | `"Ctrl+Shift+Space"` | 全角 / 半角 |
| `keybindings.toggle_punctuation` | `"Ctrl+Period"` | 中英文标点 |

新架构里这三个功能都在，热键却被写死，而且**五个宿主各写死一遍**。这三个键在新架构里**完全不存在**：全仓搜 `toggle_english_candidates` / `toggle_full_width` / `toggle_punctuation` 零命中，`platforms/windows/installer/config.default.toml` 的 `[keybindings]`（`:235-241`）只有 4 项，也没有它们。

| 宿主 | 位置 | 认法 |
| --- | --- | --- |
| macOS 标点 | `platforms/macos/src/input/InputController.mm:750-753` | `event.keyCode == 47` 且修饰键恰为 Control |
| macOS 全半角 | `platforms/macos/src/core/FullWidthInput.h:11-20` | `keyCode == kVK_Space` 且恰为 Control+Shift |
| macOS 中英 | `platforms/macos/src/input/InputController.mm:6225-6228` | `ShortcutLetter == 'e'` 且恰为 Control+Shift |
| Windows 中英 | `platforms/windows/tsf/Key/KeyEventSink.cpp:303-307`（`IsEnglishInputModeToggle`） | `code == 'E'` 且修饰键位掩码为 3 |
| Windows 全半角 | `platforms/windows/tsf/Key/KeyEventSink.cpp:717-721` | 无条件吞掉 `VK_SPACE` + Ctrl+Shift，不读任何开关 |
| Windows 标点 | `platforms/windows/tsf/Key/KeyEventSink.cpp:737-741` | 无条件吞掉 `VK_OEM_PERIOD` + Ctrl |
| Linux IBus | `platforms/linux/src/core/ClientEngine.cpp:6535-6537`、`:6546-6547`、`:6548` | 三个布尔，与设置页无关 |
| Linux fcitx5 | `platforms/linux/fcitx5/FcitxEngine.cpp:7859-7863`、`:7906-7909`、`:7911-7914` | 同上 |
| HarmonyOS | `platforms/harmony/entry/src/main/ets/keyboard/InputModeRouting.ts:130-161` | `KEYCODE_E` / `KEYCODE_SPACE` / `KEYCODE_PERIOD` 三个字面量 |

用户可见后果：和别的软件抢键时无法避让。`Ctrl+Shift+Space` 在 Windows 上被无条件吞掉（不看任何开关），`Ctrl+Shift+F`（简繁）有开关，同文件 `:722-735` 的 Alt+Shift+H 也是可关写法——只有这三个没有。

### 前提不成立：仓库里没有可重绑机制

任务原设「复用已有的可绑定快捷键数据结构与编辑器」。核对后这个前提是错的，仓库没有任何通用 chord 机制：

- **`keybindings` 全是布尔，没有键位串。** `crates/client-core/src/preferences.rs:2167-2189` 的 `KeybindingPreferences` 有 8 个字段，全部 `bool`，且派生 `Copy`。没有 `String` 字段，没有解析器，没有格式化器。
- **设置有「开关」，没有「绑定」。** `packages/ui/src/settings/keybindings` 只有 12 行类型 + 15 行默认值。`input-mode-shortcuts-section.tsx` 全部是 `SwitchRow`（开关）与 `SelectRow`（在**预定义**的几档里选一个），没有一个是录键控件。
- **语音的「可绑」是枚举，不是自由录入。** `packages/ui/src/settings/voice-hotkeys.ts` 的 `VOICE_HOLD_KEYS` 是三个布尔里选一个，标签也是写死的 `"按住右 Option 录音"`。
- **面板快捷键是只读速查。** `packages/ui/src/settings/panel-shortcuts-section.tsx` 用字符串模板拼出 `Ctrl+Shift+Command+K` 交给 `ShortcutRow`，没有任何写入口。
- **全仓没有 chord 类型。** 按 `KeyChord` / `KeyCombo` / `ChordBinding` / `HotkeyBinding` / `ParseChord` / `ShortcutSpec` 搜索（排除 `target/`、`docs/`、`.agents/`）零命中。`crates/client-core/src/panels.rs:52-59` 的 `ClientKeyEvent` 是**注入按键**的载荷（`virtual_key` + `modifiers` + `character`），不是绑定，也不存偏好。
- **全仓没有录键控件。** 按 `recordShortcut` / `ShortcutRecorder` / `KeyRecorder` / `按键录制` / `按下组合键` 搜索零命中。
- **历史那套只存在于 v0.11.1。** `engine/contracts/key_chord.h` 的 `FanyImeKeyChord::Parse`、`Binding{key, modifiers}`、`NamedKeys[]`（Space/Semicolon/Period/… 12 个具名键）都在旧仓，没有迁进本仓。

### 现有冲突校验只覆盖「字符组互斥」

`Preferences::validate` 里只有两处 `ConflictingKeyBindings`：

- `crates/client-core/src/preferences.rs:3037-3044`：五笔二三候选键 vs 翻页键组 / 以词定字键组。
- `crates/client-core/src/preferences.rs:3174-3180`：以词定字 vs 翻页。

两处都是「两个单字符抢同一个键组」，不是「chord 全局唯一」。UI 侧文案 `packages/ui/src/core/error-message.ts:17-18` 把 `key_conflict` 写死成「以词定字和翻页不能使用同一组快捷键。」，直接复用会给新场景报错误的句子。

## Proposal

不硬做。三个前置缺口各自都不是局部改动，需要先补齐，再谈这三项。按依赖顺序：

### 1. 先建 chord 契约（共享 Rust，`client-core`）

在 `preferences.rs` 新增独立的 `KeyChord` 类型，而不是往 `KeybindingPreferences` 里塞 `String`：

- `KeyChord { key: ChordKey, control: bool, shift: bool, option: bool, command: bool }`，`ChordKey` 是枚举（字母、数字、F1–F12、以及 `Space` / `Period` / `Comma` / `Semicolon` / `Slash` / `Backquote` / `BracketLeft` / `BracketRight` / `Backslash` / `Quote` / `Minus` / `Equal`）。
- 序列化保持旧版字符串写法（`"Ctrl+Shift+KeyE"`、`"Ctrl+Period"`），以便与 v0.11.1 的文档互读；解析与格式化同一个模块。
- `KeybindingPreferences` 去掉 `Copy`，加三个 `Option<KeyChord>` 字段（`toggle_english_candidates` / `toggle_full_width` / `toggle_punctuation`），缺键时 `None`。`Preferences` 与 `KeybindingPreferences` 都没有 `deny_unknown_fields`（`preferences.rs:846-847`、`:2167`），加字段不会让旧文档读不进来，旧版本读新文档也会忽略未知键——这条是这次能加字段的前提，要写进实现时的注释。

### 2. 冲突检测扩成一张表（同一个 crate）

新增 `PreferencesError::ConflictingToggleHotkeys`（或扩 `ConflictingKeyBindings` 的语义），在 `validate` 里统一判四件事：

- 三个 chord 两两不等；
- 不与保留键冲突：`Ctrl+Shift+F`（简繁，`:2179`）、`Ctrl+F9`（语音，`preferences.rs:1257`）、`Ctrl+Shift+F9`；
- 不与已启用的语言切换键冲突：Shift 轻点、单击 Control、`Ctrl+Alt+Space`、Caps Lock 轻点（`preferences.rs:2170-2188`）；
- 不与宿主自己的固定维护键冲突（`InputControllerPhysicalKeys.h:59-70` 的 Ctrl+Shift+Alt+C/R/T）。

UI 文案单独加一条错误码，不复用 `key_conflict`（它会报成「以词定字和翻页」）。

### 3. macOS 宿主改成按偏好匹配（这是「动按键路由」的部分）

现状是两种认法混用：字母走 `msime::mac::ShortcutLetter`（按当前键盘布局认，`InputControllerPhysicalKeys.h:50-56`），空格和句点走裸 `keyCode == 49` / `keyCode == 47`。而 `ShortcutLetter` 对空格和标点**明确返回非字母**（`layoutCharacter > ' ' && < 0x7f` 直接返回 `'\0'`，`PhysicalAnsiLetter` 只映射字母键）。所以把这三项并进 chord 表，等于把这两条路径合成一条统一的 `KeyChord::matches(keyCode, layoutLetter, modifiers)`，并让 `handleKeyEvent:` 的三处（`InputController.mm:6214-6217`、`:6225-6228`、`:6231-6236`）改读 `_appearance` 里新暴露的三个 chord。

同批要改的分支不只是匹配：`IsFullWidthInputToggle` 现在还兼管 Option+Shift+H（`fullWidthShortcut` 开关），拆开后要保留那条路，否则 Alt+Shift+H 会失效；`:6229-6230` 那条注释（「Only the Option+Shift+H arm is a preference」）也要跟着改。

### 4. 设置页与各宿主的编辑器

macOS、Linux、HarmonyOS 用共享 React 页（`docs/ARCHITECTURE.md:31-33`），Windows、iOS、Android 各自原生（`:34-36`）。所以一次功能要写四份界面：React 一份 + 三个原生设置。macOS 的 AppKit 窗口已冻结为回退、不加新功能（`docs/ARCHITECTURE.md:44`），只能进 React 页，需要宿主把 chord 经能力位导出、并把改动写回。

Windows 那两行要照 `:722-735` 的 Alt+Shift+H 写法改成「先比键、再读开关」，先定义「关掉后 `Ctrl+Shift+Space` / `Ctrl+.` 归谁」——`KeyEventSink.cpp:717-721` 现在是无条件吞掉，改成可关就必须把它交给应用。

## Alternatives considered

- **只做 macOS，其他宿主保持写死。** 最强的理由是任务书只点了 macOS，一个宿主改完就能验收，改动面小一个数量级，也不会碰 Windows TSF 的 compartment 语义。不选，因为 chord 契约与冲突检测必须落在共享 `client-core`（架构规则要求规则下沉，`docs/ARCHITECTURE.md:40`），共享层一旦有了新字段，五个宿主读到却只有 macOS 消费，正是审计里「已迁但无消费」那一类更危险的状态；而且 Windows 那两行「无条件吞掉」恰恰是审计标注的冲突点。
- **先做「只有字母键可绑」，把 Space / Period 留作固定。** 最强的理由是不用碰 `ShortcutLetter` 与 `PhysicalAnsiLetter` 的字母假设。不选，因为出厂值里就有两个非字母键（`Ctrl+Shift+Space`、`Ctrl+Period`），只支持字母会让这两项的出厂值无法表示，等于做完就与旧版不一致。
- **往 `KeybindingPreferences` 直接加三个 `String`，不做 `KeyChord` 类型。** 最省事，复用现有 8 个字段的位置。不选，因为该校验的「合法 chord」与「互不冲突」就没有单一真源，五个宿主会各自解析一遍，正是审计里 `trilingual_cycle` 那种语义漂移的来源。
- **把三个 chord 做成「开关 + 固定组合」的 SelectRow，照 `input-mode-shortcuts-section.tsx` 现在的样子。** 最强的理由是复用现成控件、零新结构，今天就能上线。不选，因为那只是把「写死」改成「在几档预设里选」，用户仍不能绑到自己想要的键，旧版的「可重绑」没有恢复。

## Acceptance criteria

- `KeyChord` 的解析 / 格式化往返覆盖三个出厂值，并与 v0.11.1 的 `"Ctrl+Shift+KeyE"` 写法互读；单测在 `client-core`。
- 冲突检测覆盖：三项互斥、与保留键、与语言切换键、与维护键；每条各一个先失败后通过的用例。
- 「改绑后按新键触发、按旧键不再触发」至少两条：判定在共享层就写 Rust 单测；纯 macOS 宿主的匹配写 `platforms/macos/tests/` 里**不依赖窗口**的 CTest（本机无图形会话，窗口类 CTest 会 abort）。
- `cargo test -p msime-client-core` 全绿、`cargo fmt --all --check`、改到的 crate `cargo clippy --all-targets --locked -- -D warnings`、`cargo check -p msime-host-api -p msime-input-runtime --all-targets`。
- 改了设置页则 `pnpm --filter @msime/desktop test` 全绿 + `pnpm format:check`。
- 五个宿主的匹配点全部改成读偏好；Windows 的 `:717-721`、`:737-741` 改成可关写法后，「关掉这个 chord 归谁」有明确断言。

## Risks

- **必须在 `KeybindingPreferences` 去掉 `Copy`。** 现在只有 `preferences.rs:2458` 一处按值构造，改成 `Clone` 的爆炸半径小；但只要漏一处，编译期就会报出来，风险可控。
- **macOS 按键路由是真改动。** 合并「按 keyCode 认」与「按布局字母认」两条路径时，容易顺手把 Option+Shift+H 或维护键弄坏。`platforms/macos/tests/input/ShortcutTest.mm` 已覆盖 `IsFullWidthInputToggle`（`:621-622`、`:2039`）与标点键（`:1434-1435`），改前先确认这两组仍能跑。
- **Windows 从「无条件吞掉」改成「可关」是行为变化。** 关掉后 `Ctrl+Shift+Space` / `Ctrl+.` 归应用，与今天的体验不同；默认值必须保持开，否则是静默回归。
- **HarmonyOS 与 Linux 的语义不完全一致。** fcitx5 在「输入法关闭」状态下也吃 `Ctrl+.`（`FcitxEngine.cpp:7866-7875`），IBus 的 `punctuation_toggle` 是纯布尔（`ClientEngine.cpp:6548`）。统一到 chord 表时，这两个「关闭状态下也生效」的分支要显式建模，不能按「只在组字时匹配」一刀切。
- **范围提示。** 这次逐项核完，缺口是「共享层没有 chord 契约 + 冲突校验没有通用表 + 五个宿主各自写死 + 四份设置界面」，比任务书预估的「把三项并进既有机制」大一个档。建议按上面四步拆成四个独立切片提交，第一步（`KeyChord` + 冲突表 + Rust 单测）可以单独验收。
