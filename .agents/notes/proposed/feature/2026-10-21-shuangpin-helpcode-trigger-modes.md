# Agent Note: 双拼辅助码补齐直接、间接与句中触发方式

Status: proposed

## Problem

Windows 版水杉（`msime-windows` v0.11.1）的双拼辅助码有三种触发方式，并各有可多选的触发键。出厂配置是 `installer/default_config/config.default.toml` 的 `[helpcode]` 段（`shuangpin_helpcode_direct` 起，见下表），设置页的面板与互斥规则在 `server/src/settings/native/native_page_helpcode.cpp` 的 `BuildHelpcodePage`。

macOS 侧只有四类行：开关、方案、导入、候选窗显示（`packages/ui/src/settings/pages/helpcode-page.tsx` 的 `HelpcodeSettingsGroup`）。`crates/client-core/src/preferences.rs` 的 `HelpcodePreferences` 只有 `enabled` / `schema` / `show_in_candidate_window` 三个字段，没有 `direct`、触发键、`mid_sentence` 这些键。因此 macOS 用户无法选择「不敲触发键就不认辅助码」，也没有反引号 / 分号 / 大写触发键，更没有句中辅助码。

### Windows 的配置组 ↔ macOS 现状

| Windows `[helpcode]` 键 | 默认 | 语义 | macOS 现状 |
| --- | --- | --- | --- |
| `shuangpin_helpcode` | `false` | 双拼辅助码总开关 | 有：`shuangpin_helpcode.enabled` |
| `shuangpin_helpcode_schema` | `"lantian"` | 码表方案 | 有 |
| `show_sp_helpcode_in_candidate_window` | `true` | 候选窗显示辅助码 | 有 |
| `shuangpin_helpcode_direct` | `true` | **末尾直接**：完整音节后直接接辅码，多一键是单码（`ulpbx`），末尾两键带大写是双码（`ulpbxY`）。关闭后改由下面的触发键引导 | **无字段，恒按「直接」处理**。`ShuangpinEngine::active_helpcode_query` 先试 `full_helpcode_query`（两码带大写）再试 `single_helpcode_query`（奇数长度的末位字母），都只在完整音节基座上成立 |
| `shuangpin_helpcode_trigger_backtick` | `false` | **末尾间接**触发键：反引号 | 无 |
| `shuangpin_helpcode_trigger_semicolon` | `true` | **末尾间接**触发键：分号 | 无 |
| `shuangpin_helpcode_trigger_uppercase` | `false` | **末尾间接**触发键：大写字母 | 无。大写现在只用作「双码第二码」的判别（`is_full_help_mode`），不是段首引导键 |
| `shuangpin_mid_sentence_helpcode` | `false` | **句中（间接）**：完整音节后敲触发键，后面的字母筛这个音节上的字，之后可以接着打 | 无 |
| `shuangpin_mid_sentence_helpcode_backtick` | `true` | 句中触发键：反引号 | 无 |
| `shuangpin_mid_sentence_helpcode_semicolon` | `false` | 句中触发键：分号（微软 / 搜狗 / 紫光里分号还是 `ing` 韵母，只在完整音节后才当触发键） | 无 |
| `shuangpin_mid_sentence_helpcode_uppercase` | `false` | 句中触发键：完整音节后的大写字母，等价于「反引号 + 这个字母」 | 无 |
| `shuangpin_direct_helpcode` | `false` | **句中（直接）「万象式」**：不要引导键，逐音节解析，三码自动识别、四码单独敲时直接定字，句中要补 `/` | **有等价实现，但没有独立开关**：`ShuangpinEngine::direct_helpcode_paths` / `direct_helpcodes_own_capitals`（[移动端直接辅助码](../../implemented/feature/2026-10-17-mobile-direct-helpcode.md)）。只要 `enabled` 为真就生效，且与末尾直接同时生效，先匹配先返回 |
| `shuangpin_direct_helpcode_slash` | `true` | 万象式四码句中用 `/` 结束 | 无。macOS 的 `/` 目前不进拼音组字 |
| `shuangpin_direct_helpcode_uppercase` | `false` | 万象式四码句中用第二位辅码大写结束 | 部分：`direct_helpcode_paths` 认「小写第一码 + 大写第二码」，但没有「用大写标记四码结束」这一层开关 |

Windows 侧这些键的详细语义见 `msime-windows` v0.11.1 的 `engine/contracts/mid_sentence_helpcode.h`、`engine/core/syllable_helpcode.h`、`engine/contracts/direct_helpcode.h`；末尾间接与句中间接共用同一套扫描规则（触发键在输入串里统一记成反引号 / `kEndMarker`），因为 TSF 的同步吃键、Server 的 raw 串和引擎解析三处必须看到同一种串。

## Proposal

在共享偏好、引擎和设置页三层补齐，使 macOS 与 Windows v0.11.1 的触发语义对齐：

1. **偏好**：在 `shuangpin_helpcode` 下新增双拼专属字段（`quanpin_helpcode` 不读）：`direct`（末尾直接）、`trigger_backtick` / `trigger_semicolon` / `trigger_uppercase`（末尾间接）、`mid_sentence` 与 `mid_sentence_backtick` / `mid_sentence_semicolon` / `mid_sentence_uppercase`（句中间接）、`direct_helpcode` 与 `direct_helpcode_slash` / `direct_helpcode_uppercase`（万象式直接）。默认值与 Windows v0.11.1 一致。互斥规则在写入端归一：末尾间接与句中间接的同一个触发键只能归一边；`mid_sentence` 与 `direct_helpcode` 不能同时为真（两个都真时按万象式直接）。`HelpcodePreferences` 现在是全拼 / 双拼共用类型，双拼专属字段不能直接摊进去，需先拆出双拼专属结构或加 `#[serde(default)]` 的可选子节。
2. **引擎**：`SessionOptions` / `QueryRequest` 带上这些开关；`ShuangpinEngine` 按模式选择判定：
   - 末尾直接关：`active_helpcode_query` 不把「完整基座 + 末尾裸字母」当辅助码；只有触发键开启的段才认。
   - 末尾间接：扫描输入串末尾的触发键段（反引号 / 分号 / 段首大写），后接 1~2 个辅码，大小写均算；一码重排、两码筛选；段不在输入串末尾时只当分隔符。
   - 句中：扫描每个触发键段并绑定它前面那个完整音节，作为约束带进整句解码；约束后仍能接着打，后续按键继续走普通查询。
   - 万象式直接：把 `direct_helpcode_paths` 从「恒开」改为受 `direct_helpcode` 控制，并补 `/` 与「第二位大写」两种四码结束标记。
3. **设置页**：`helpcode-page.tsx` 增加「末尾辅助码」直接开关 + 间接触发键多选，以及「句中辅助码」区（句中间接开关 + 触发键多选、万象式直接开关 + 四码标记多选）；`helpcode-preferences.ts` 补类型和默认值。

### 要动的文件与函数

| 层 | 文件 | 函数 / 类型 |
| --- | --- | --- |
| 偏好 | `crates/client-core/src/preferences.rs` | `HelpcodePreferences`、`default_shuangpin_helpcode`、`active_helpcode`，`Preferences` 的 serde / 字段校验与写回 |
| 偏好测试 | `crates/client-core/src/preferences/tests.rs` | 新键的默认值、往返、旧文档缺键、互斥归一 |
| 设置模型 | `packages/ui/src/settings/helpcode-preferences.ts` | `HelpcodePreferences`、`defaultHelpcode` |
| 设置页 | `packages/ui/src/settings/pages/helpcode-page.tsx` | `HelpcodeSettingsGroup`、`enabledRow` / `schemaRow` / `displayRow` |
| 会话选项 | `crates/engine/src/session/options.rs` | `SessionOptions` |
| 按键入口 | `crates/engine/src/session/input.rs` | `handle_character`（接受反引号 / 分号触发键）、`set_shuangpin_helpcode_enabled`、`Session::new` 应用选项 |
| 方案按键 | `crates/engine/src/shuangpin/scheme.rs`、`crates/engine/src/types.rs` | `ShuangpinScheme::handle_key`、`accepts_ing_key`、`SchemeKey` |
| 触发段扫描 | `crates/engine/src/shuangpin/query.rs` | `detect_active_double_helpcode_length`、新增与 Windows `ScanBlocks` 对应的解析 |
| 查询 | `crates/engine/src/shuangpin/engine.rs` | `active_helpcode_query`、`full_helpcode_query`、`single_helpcode_query`、`direct_helpcode_paths`、`ShuangpinEngine::query` |
| 组字基座 | `crates/engine/src/session/composition.rs` | `resolve_shuangpin_composition_base`、`active_shuangpin_helpcode_length`、`has_active_helpcode` |
| 编辑 / 字面 | `crates/engine/src/session/editing.rs`、`crates/engine/src/session/literal.rs` | `segment_raw_boundaries`、尾随辅助码判定与退格 / Ctrl+方向键分段 |
| 解码 | `crates/engine/src/ime/mod.rs` | `apply_shuangpin_helpcode_segmentation`、整句 / 词网格解码入口 |
| 词典 | `crates/engine/src/shuangpin/dictionary.rs` | `generate_constrained_sentences`、`generate_with_helpcodes`、按约束的缓存键 |
| 运行时 | `crates/input-runtime/src/engine.rs`、`crates/input-runtime/src/runtime.rs` | 新选项透传 |
| 网页引擎（可选） | `crates/engine-wasm/src/host.rs` | `set_helpcode` 附近的选项映射 |

规模判断：只加「末尾直接」一个开关约几十行；末尾间接要新增触发段解析并让反引号 / 分号进入组字串，约一两百行；**句中**要改「输入串 → 音节切分 → 词网格 / 束搜索」的判定，把约束带进解码并保住编辑分段、缓存键和预编辑显示，会动 `Session` 的组字状态机与原始串契约，明显超出「新增偏好字段 + 引擎读字段改判定」的 300 行范围，因此本笔记只记录方案，不改代码。

## Alternatives considered

- **只加 `shuangpin_helpcode_direct` 一个开关。** 最强的理由是改动最小、能立刻让用户关掉「末尾直接把字母当辅码」的意外。但关掉后没有任何触发键可用，等于把辅助码整个关掉；Windows 的「直接关」是「改用间接引导键」，两者必须一起做才有意义。
- **在宿主层（macOS `InputController` / `input-runtime`）拦住触发键再拼进原始串。** 可以少碰引擎。但 macOS 输入法在进程内、宿主没有音节表，判断不了「前一个键是否构成完整音节」和约束落在哪个音节；这等于把引擎已有的规则在宿主再写一份，正是[移动端直接辅助码](../../implemented/feature/2026-10-17-mobile-direct-helpcode.md)否掉的「三平台各写一套」。
- **原样移植 `msime-windows` 的 C++ 契约。** 语义最完整，注释也最全。但那份契约由 TSF 同步吃键、Server raw 串、引擎解析三处共同维护，本仓没有 TSF 这一层，必须把三处折进 Rust 的 `Session` / `Engine`，并重建「解码时按音节筛边」；移植的是设计，不是可以直接复制的代码。
- **句中辅助码改成「解完整句后再过滤候选」。** 实现小，能复用现有整句结果与缓存。但被锁住的低频字组合可能从未进入束搜索，与移动端直接辅助码遇到的是同一个问题：只有过滤，找不到目标句。
- **不做，仅靠文档说明差异。** 诚实且零风险，但 Windows 用户切到 macOS 后无法沿用已有习惯，[辅助码公共偏好](../../implemented/simplification/2026-10-16-helpcode-preferences.md)和共享设置页也会继续与来源分叉。

## Acceptance criteria

1. `crates/client-core`：新键默认值与 Windows v0.11.1 一致；旧偏好文档缺键按默认读、写回不丢其它字段；末尾 / 句中的同一触发键互斥归一有单测。
2. `crates/engine`：
   - 末尾直接关闭后，`ulpbx` 不再被拆成「完整音节 + 单码」，按纯拼音处理。
   - 末尾间接：`ulpb;xy`（分号触发）与 `ulpbXy`（大写触发）拿到与末尾直接相同的重排 / 筛选结果，触发键大小写均可，段不在末尾时只当分隔符。
   - 句中：``ulpb`xY`` 只筛第二个音节上的字，继续输入后面的拼音仍整句解码且锁住的字不丢；`;` 在微软 / 搜狗 / 紫光方案下仅当上一节是完整音节时当触发键，否则仍是 `ing` 韵母。
   - 万象式直接：开 `direct_helpcode` 后末尾直接 / 间接都让位；`uiab/` 与 `uiaB` 按配置结束四码。
   - 退格、Ctrl+←/→ 与 Ctrl+Backspace 按音节 / 辅码段整体处理，分词边界正确。
3. `cargo test -p msime-engine -p msime-client-core`、`cargo fmt --all --check`、`cargo clippy -p msime-engine -p msime-client-core --all-targets --locked -- -D warnings` 全绿；改设置页时 `pnpm --filter @msime/desktop test` 与 `pnpm format:check` 全绿。
4. 至少一个先失败后通过的 Rust 单测，覆盖「直接关 + 触发生效」这一条真实行为差异。
5. macOS 真实编辑器里手动验证四种组合；网页引擎若同步 `SessionOptions`，`crates/engine-wasm` 的 routing 用例一并覆盖。

## Risks

- **会话契约变化。** 触发键进入原始串后，`raw_input` 会包含反引号 / 分号 / 大写，候选的 `pinyin` 消耗范围、`segment_raw_boundaries`、退格与 Ctrl+方向键、缓存键都要跟着理解这些段；改动跨 `Session` 与 `Engine` 两层，回归面比单点判定大。
- **分号冲突。** 微软 / 搜狗 / 紫光双拼里 `;` 是 `ing` 韵母（`ShuangpinProfile::uses_semicolon_key`）。触发判定必须只在完整音节后成立，且不能把「奇数节 + `;`」的韵母解读挪走；非这些方案下 `;` 目前根本不进组字，要先放开按键接受。
- **大写冲突。** 现在大写是「双码第二码」的判别位；触发键大写打开后，同一位置要改判为段首，且必须与末尾双码互斥，否则会出现同一串两种读法。
- **缓存串味。** 句中约束不同的同一串拼音候选不同，词典缓存键必须带上约束（Windows 用 `syllable_helpcodes_signature`），否则会串答案。
- **跨端一致性。** `SessionOptions` 由 macOS、Windows、Linux、网页引擎和移动端共用；新增字段的默认值若与移动端直接辅助码的既有行为冲突，需要一并决定是否给移动端也加开关。触发键在预编辑里的显示（Windows 统一显示反引号）和智能标点对 `/`、`;` 的处理也要一起核对。
