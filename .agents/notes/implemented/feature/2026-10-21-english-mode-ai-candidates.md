# Agent Note: 英文模式的 AI 候选（#9015）

Status: implemented

## Problem

英文模式（专用英文 `dedicated_english`，以及 `local_modes.temporary_english`）下只有本地英文词库的前缀补全，拼错一个字母就没有候选。`InputSession::online_query` 一开头就把 `dedicated_english` 和非 `None` 的 `local_mode` 判掉，`ai_eligible` 又要求每一段都是完整拼音，所以英文模式一个在线请求都发不出去。

## Decision

新增偏好 `ai_assistant.english_candidates`，默认 false。打开后英文模式的字母串走一条单独的 AI 通道：`InputSession::english_ai_query` 把它当作只有一段的查询，标 `ai_eligible` 而**不**标 `cloud_eligible`（云候选服务回答的是中文拼音）；回包由 `apply_english_online_rows` 记在「回答时的那串字母」名下，字母串一变（含组字取消）就作废，只在重建英文候选列表时并入，所以上一个词的补全不会被贴到新词上。应答行的座位规则不变：`input-runtime` 的 `normalize_online_slots` 把 `AiSuggestion` 来源的行放到第一页最后一格，不打乱本地候选。

`OnlineQuery` 与 `AiSuggestionRequest` 各加一个 `mode`（`pinyin` / `english`），宿主据此区分两类请求；内置提示词 `DEFAULT_CANDIDATE_PROMPT` 增加 `english` 分支（英文补全与拼写纠错，不生成中文）。

wire 兼容：`mode` 拼音时不写进 JSON，`#[serde(default)]` 让旧宿主读不到时按拼音处理。

开关关闭时 `online_query` 的旧路径逐字节不变——隐私边界放在宿主偏好一侧，引擎默认不发。

## Alternatives considered

**顺手把云候选也放给英文模式。** 云候选服务回答的是中文拼音，把英文串发过去既出不了有用候选、又白送一次网络请求；英文的补全与纠错只有 AI 这一条路能做。

**让英文 AI 行走引擎已有的动态候选缓存（`apply_dynamic_candidates`）。** 英文模式的候选列表不是拼音那条路径产生的，动态缓存最终只被拼音的 `update_mixed_candidates` 消费，进不了 `dedicated_english_candidates`／`local_candidates`；所以另开一份 `english_online` 暂存，在重建英文列表时并入。

**不新增偏好，直接放开英文模式。** 英文模式常用来输入网址、命令和账号；不默认关闭就改变了现有用户的隐私预期。#9015 的提出者也明确要求默认关闭。

## Consequences

收益：英文模式拿到 AI 的单词补全与拼写纠错（`accessability` → `accessibility`）；中文路径的行为逐字节不变。

代价：多一个偏好和一条 wire 字段；宿主提示词要能处理两种 `mode`。

设置页：AI 页「联想」组里新增「英文模式也请求 AI 候选」开关（`packages/ui/src/settings/ai-settings-page-section.tsx`），显示与否由 `HostCapabilities.english_ai_candidates` 决定（缺省按提供处理，英文键面不经会话组字的宿主报 `false`）；`AiAssistantPreferences.english_candidates` 是可选字段，旧文档没有它时按关闭渲染。

`#9015` 评论里「中文模式中英混输（`wodegithub`、`woyongtypescript`）时 AI 从不请求」这次**分开放**，不在这批做：`ai_eligible` 要求每一段都是完整拼音，混输里的 `git` 段会被判掉；要放行混输得让分段里允许非拼音段，提示词、分段形状和资格规则都跟「字母串 + `mode:"english"`」的英文契约不同，把两件事捆进同一个隐私敏感的切片会让审查和「开关关闭时零行为变化」的边界都说不清。维护者要做时建议在 `OnlineQueryMode` 加第三个值或复用 `pinyin` 并放宽分段资格，另开 issue。

验证：`cargo test -p msime-engine -p msime-client-core -p msime-client-providers` 全绿（2377 + 431 + 231，0 失败）；引擎侧 `dedicated_english_asks_ai_only_when_the_switch_is_on`、`temporary_english_asks_ai_only_when_the_switch_is_on`、`the_english_ai_switch_leaves_the_pinyin_online_query_unchanged` 覆盖「关→不发、开→发、拼音不变」三点；client-core 断言偏好默认关、缺键按关、恢复默认收回；client-providers 断言请求体 `"mode":"english"`/`"pinyin"` 与旧文档兼容。`cargo fmt --all --check` 与 `cargo clippy --all-targets --locked -- -D warnings`（五个改到的 crate）均通过。已知：`msime-host-api` 的 `vocabulary_boundary_imports_reviews_and_reports_one_whole_status` 用固定日期 `2026-09-24` 断言 `due`，真实日期过了窗口后基线上也会失败，与本次改动无关。
