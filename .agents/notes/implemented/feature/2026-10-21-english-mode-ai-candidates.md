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

**未完成，这个分支还得继续做：** 设置页开关还没有。`packages/ui/src/settings/ai-assistant-preferences.ts` 与 `ai-settings-content.tsx` 目前都不认识 `english_candidates`，用户只能手改 `preferences.json` 才能打开它。

`#9015` 评论里「中文模式中英混输（`wodegithub`、`woyongtypescript`）时 AI 从不请求」是另一件事：`ai_eligible` 要求每一段都是完整拼音，`git` 这类切不成拼音的段会被判掉。这次没有动它。

验证：`cargo test -p msime-engine -p msime-client-core -p msime-client-providers` 全绿（430 + 230 + 17，0 失败）；`cargo fmt --all --check` 与 `cargo clippy -p msime-engine -p msime-client-core -p msime-client-providers --all-targets --locked -- -D warnings` 均通过。
