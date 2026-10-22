# Agent Note: 共享偏好默认值对齐 Windows v0.11.1 出厂配置

Status: implemented

## Problem

C 档对照复核（`/Users/admin/worktrees/_refs/parity-tier-c-review.md`）发现新仓共享偏好的一批默认值是按门禁 pin（`345cb87a`）猜的，和作为对齐目标的 Windows v0.11.1 出厂配置不一致，另有三个用户键在新架构里变成皮肤字段或合并开关：候选窗 fit_content 只在皮肤包 `skin.toml` 上存在、日期时间混入和 T 模式共用一个开关、左右键翻页等若干项默认开/关与出厂相反。安装后行为因此偏离 Windows 正式版，用户没有可调的入口。

## Decision

以 v0.11.1（参考 tag `dd018f7`）为唯一对齐目标，把六项默认值改成出厂值，并补齐三个缺失的用户开关。serde 缺键才读新默认，已写进文档的值原样生效，不覆盖已安装用户的显式配置。

### 默认值对照（v0.11.1 出厂 / 门禁 pin / 改前 / 现值）

| 共享字段 | 参考键 | v0.11.1 | pin | 改前 | 现值 | 依据 |
|---|---|---|---|---|---|---|
| `navigation.left_right` | `general.candidate_horizontal_arrow_navigation` | 关 | 无此键 | 开 | 关 | post-pin 键，出厂关 |
| `local_modes.expression` | `utility.v_mode` | 开 | 无此键 | 关 | 开 | post-pin 键，出厂开 |
| `local_modes.emoji` | `utility.emoji_mode` | 关 | 开 | 开 | 关 | v0.11.1 优先于 pin |
| `local_modes.kaomoji` | `utility.kaomoji_mode` | 关 | 开 | 开 | 关 | v0.11.1 优先于 pin |
| `local_modes.chaizi` | `utility.radical_mode` | 开 | `unicode_mode`（改名前身）开 | 桌面缺键读关 | 缺键读开 | v0.11.1 改名且出厂开；参考 Shift+U 拆字，本仓拆字在 Shift+C |
| `mixed_input.symbols` | `general.symbol_mixed_input` | 关 | 无此键 | 开 | 关 | post-pin 键，出厂关 |
| `quanpin_helpcode.enabled` / `shuangpin_helpcode.enabled` | `helpcode.{quanpin,shuangpin}_helpcode` | 关 | 开 | 开 | 关 | v0.11.1 优先于 pin |

### 结构拆分

- `local_modes.date_time`（Shift+T 模式）与新增的 `local_modes.date_time_inline`（Windows `utility.date_time_candidates`，出厂关）分开。Engine 侧对应 `LocalModeOptions.date_time_inline`、`EngineOptions.local_date_time_inline`，`insert_inline_date_time` 只看行内混入开关，T 模式与混入可以各自独立。`date_time_menu` 改挂 `date_time_inline`（📅 入口只出现在混入候选里），出厂仍开。
- 新增顶层 `candidate_window_fit_content`（出厂关，关着不写进文档），与皮肤 `fit_content` 是或关系。macOS 的 `AppearancePreferences` 读共享文档该键，`InputController` 三处 `_candidateLayoutFitContent = skin.fitContent || appearance.candidateWindowFitContent`；共享设置页「外观」加「收紧窗口宽度」行，只向消费它的 macOS 显示。
- `mixed_input.symbols` 从 `enabled_by_default` + `skip_serializing_if` 改成 `#[serde(default)]` 总是写出：旧版本把缺键读作开，省略新默认（关）会让降级后的文档行为跳回开，显式写出 false 保证新旧版本读法一致。
- Windows 出厂模板（`platforms/windows/installer/config.default.toml`）补六个 post-pin 键并把 emoji/颜文字/辅助码四个值改成出厂值，`test-reference-config-coverage.py` 的 MAPPING 同步登记六个映射。

### 设置页同步

共享设置页的缺省表同步：`defaultLocalModes` 的 emoji/kaomoji 改关；`localModeControlState` 把 chaizi、expression 列入缺键读开；日期格式入口的禁用条件改查 `date_time_inline`；左右键勾选缺省改关；`symbols ?? false`；外观页新增「收紧窗口宽度」开关（仅 macOS）。macOS 回退原生设置窗 `localModeEnabled` 的缺省表同步为 emoji/颜文字关、拆字与其余开。

### 判为单独立项的项

`network.proxy_*` 自定义代理只覆盖共享 Rust 栈（reqwest 客户端），不覆盖 macOS 原生 HTTP（`TranslationCache.mm` 的 NSURLSession、C++ 侧 native transport）。把自定义代理接进原生传输要动每个原生网络栈并逐平台验证，超出本批默认值改动的范围，单独立项见 `../../proposed/feature/2026-10-22-custom-proxy-native-http.md`。

## Alternatives considered

- **保留 pin 出厂值不改**：pin 早于 v0.11.1，按它对齐等于把 v0.11.1 后来做的出厂修正全部丢弃；任务书明确 v0.11.1 优先。
- **`symbols` 继续略写新默认（`skip_serializing_if = "is_false"`）**：文档更干净，但旧版本把缺键读作开，用户在新版关掉后降级回去会被静默打开。显式写出是唯一的双向一致写法。
- **`chaizi` 仍按宿主形态区分**：触屏宿主拆字缺键读开本就符合出厂值，桌面读关是出于大写 C 被截的担心，但参考出厂就是开——大写 C 进模式是参考同样存在的行为，不是新仓独有的缺陷；对齐统一读开。
- **fit_content 挂在皮肤字段不动**：六个出厂皮肤全是 false，用户永远到不了收紧宽度，等于没有这个功能。

## Consequences

- 新装和缺键文档读到的行为与 Windows v0.11.1 出厂一致。此前 `symbols` 默认开时被略写键位的文档会随出厂值改为关（那些用户没有显式开过）；写明的值（true/false）一律不变。
- Engine 默认值（`LocalModeOptions::default()`、`prepare_options`）保持「参考本地模式全开」的夹具语义：`date_time_inline` 在引擎层默认开，golden 与 session 测试不受影响；产品默认在偏好层关闭。
- 测试：每个默认值改动都有「缺键读新默认 + 写明值不覆盖」的断言（`emoji_and_kaomoji_modes_default_off`、`expression_mode_defaults_on`、`date_time_inline_defaults_off_and_is_written_only_when_on`、`left_right_navigation_defaults_off`、`mixed_symbols_defaults_off`、`helpcode_enabled_defaults_off`、`candidate_window_fit_content_defaults_off`），引擎层另有 `inline_date_time_is_a_separate_switch_from_the_mode` 钉住 T 模式与混入的分闸；host-api 的 `chaizi_defaults_on_until_the_user_flips_it` 断言缺键进入拆字。改前这批断言全部失败（缺键读到旧默认），改后全绿。
- `sentence_source_badge` 在 C 档清单里被错写为「已判定不做」，清单行更正为「proposed 待拍板」（`_refs/parity-gaps.md`），仓库里的 proposed 笔记维持。
