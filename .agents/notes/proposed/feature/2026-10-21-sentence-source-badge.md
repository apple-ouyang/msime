# Agent Note: 整句候选来源徽章在 macOS 的取舍

Status: proposed

## Problem

旧版 Windows 正式版（v0.11.1）有「整句来源标签」设置（`sentence_source_badge`）：整句候选后面显示〔Trigram〕〔Unigram〕〔神经D〕〔神经K〕〔万象〕〔八股〕〔墨奇〕等来源，共 7 种，用户能看出每句是哪套模型给的（对照见 `/tmp/msime-audit/legacy-windows.md` §1.2、§2）。

macOS 侧既没有来源徽章 UI，`CandidateSource` 枚举里也没有对应 `Collocation`、`Google` 这类的整句来源值。照搬 Windows 之前要先回答：macOS 实际能产生的整句来源有几种？如果只有 1–2 种，标签区分不出什么，就不值得为它加设置行、wire 字段和展示链路。

本篇记录盘点结果与判定，等维护者拍板，不改代码。

## macOS 实际整句来源盘点（证据）

整句行的生产者全部走 `crates/engine/src/lattice/merge.rs` 的 `owned_sentence_row`，`source` 只有两个取值：

| 整句来源 | 引擎判别 | 何时出现 | 证据 |
| --- | --- | --- | --- |
| 词格（本地整句联想，含纠错整句、整句改字） | `CandidateSource::Generated`（wire 值 8） | `word_lattice` 开着时总是有；默认开 | `crates/engine/src/lattice/merge.rs:121-125,229-233,277-281`；纠错 `crates/engine/src/quanpin/dictionary.rs:1088-1098`、`crates/engine/src/shuangpin/dictionary.rs:389-399`；整句改字 `crates/engine/src/session/conversion.rs:287-290` |
| 神经联想（键盘模型） | `CandidateSource::NeuralKeyboard`（wire 值 11） | 只在该模型被加载且重排选中时，至多多一行 | `crates/engine/src/lattice/merge.rs:282-288`；模型加载 `crates/engine/src/quanpin/dictionary.rs:604-612` |

要点：

- 枚举 `crates/engine/src/types.rs:8-33` 里没有任何 `Collocation`、`Google` 或「神经桌面」来源值。整句只用到上面两个。
- 桌面神经联想（`settled-model`）不是引擎的来源：它由输入运行时在打字停顿后做 **落定重排**，只旋转已有候选，`candidate_sources` 元数据跟着旋转但不改值（`crates/input-runtime/src/runtime.rs:867-903`，旋转在 `:2044`）。被它排到首位的整句仍然是 `Generated`，徽章看不出区别。
- 引擎的整句开关里根本没有桌面模型这一项（`crates/engine/src/types.rs:887-893` 的 `SentenceAssociationOptions` 只有 `word_lattice` / `neural_keyboard` / `show_next_on_duplicate`）；注释明确桌面模型只跑运行时的落定重排（`crates/engine/src/types.rs:887`、`crates/engine/src/lattice/neural.rs:1-6`）。
- macOS 设置页 `mobile = false`：输入页只列「本地整句联想」和「桌面神经联想」两个开关，`neural_keyboard` 开关只在触屏宿主列出，「键盘神经联想」在 macOS 上不可见、也不默认打开（`packages/ui/src/settings/pages/input-page.tsx:396-400` 传 `mobile={mobilePlatform}`；`packages/ui/src/settings/sentence-association-section.tsx:36-47,68-80`；`crates/client-core/src/host_surface.rs:459-460`）。因此 macOS 桌面上用户实际能看到的整句来源只有 **1 种**：词格。
- 把触屏宿主的键盘神经行也算上，全平台最多 **2 种**；Windows 的 7 种里有 5 种（Unigram/Google、万象、八股、墨奇、以及独立的神经D）在 macOS 没有生产者——Google 解码器已移除（`crates/engine/src/lattice/mod.rs:1`），octagram 没有读取端。

**结论：1–2 种，低于「≥3 种且用户能看出差别」的门槛，本期不做。**

## Proposal

本期不实现整句来源徽章：不加 `sentence_source_badge` 偏好键、不加设置行、不在 `CandidateSource` 上新增 wire 值、不改候选展示。

只有满足下面任一条时才重开这项：

1. 引擎出现第三种整句来源的实际生产者（例如接入 octagram/语法搭配模型，或新增独立的桌面神经整句解码器）。
2. 桌面神经落定重排被改成会回写来源（那时 `Generated` 与「神经桌面」才在徽章上可分）。

## 需要维护者拍板的点

1. **是否接受「macOS 整句来源只有 2 种、桌面只有 1 种可达」的判定，本期不做。** 推荐：接受，本篇留在 `proposed` 作为记录。
2. **桌面神经联想是否要单独标一种来源。** 推荐：不做。落定重排不产生新候选，单独标只会把同一条词格句换个标签，却要新增冻结的 wire 值并改宿主回写链路。若维护者认为「用户看到顺序变了却不知道是神经模型干的」算问题，再单独立项。
3. **是否只在触屏宿主（Android/iOS）做来源徽章。** 推荐：不在本任务做。那边 `neural_keyboard` 开关可见、确实有 2 种来源，但 2 种标签的信息量仍低，且本任务范围是 macOS。
4. **相邻缺口「去重后补充次选」（`show_next_on_duplicate`）桌面设置行是否并入。** 审计把它列为「字段/引擎都有、只缺设置行」的真缺 UI（`/tmp/msime-audit/legacy-windows.md` §1.2）。推荐：走独立那条线（已有 `msime-sentence-next-on-duplicate` worktree），不并进徽章任务。

## Alternatives considered

- **照搬 Windows 的 7 个来源标签和 `sentence_source_badge` 开关。** 最强的理由是用户可见行为与 Windows 完全对齐、设置键名也一致。但 macOS 只有 2 个来源有生产者，7 个标签里 5 个永远是空的：Unigram/Google 随 Google 解码器一起移除（`crates/engine/src/lattice/mod.rs:1`），万象/八股/墨奇要等 octagram `.gram` 读取端接入，独立的神经D 在当前架构里不产生候选。照搬会把一个在 Windows 上有意义的开关做成 macOS 上的死设置，故否掉。
- **只把「神经」标出来（词格行不标）。** 最强的理由是用户最需要知道「这一句是神经模型挑的」，且实现只需读现有 `NeuralKeyboard`。但桌面上 `neural_keyboard` 没有 UI 开关、不默认打开，桌面神经又不改来源，所以桌面上这条标签几乎不会出现；要让它出现就得先做上面第 2 点那条改架构的事，已超出「搬徽章」的范围，故否掉。
- **只在触屏宿主做（键盘神经 vs 词格）。** 最强的理由是那 2 种来源在触屏上确实都能出现，改动面收窄到一个宿主。但标签区分价值仍由 2 种来源决定，且本任务的范围是 macOS；先把判定记录下来，避免在没有新信息的情况下为移动端单独开一套 UI，故否掉。
- **不做（复用现状）。** 最强的理由：macOS 来源 ≤2 种、桌面只有 1 种可达，徽章区分度低；默认关的开关还会多出一行设置和一层 UI 维护成本，而真正缺的整句能力（octagram 语法模型）另有专节在审。这也是本篇的结论。

## Acceptance criteria

- 维护者对上面 4 个拍板点给出结论；本篇保持 `proposed` 直到结论落定（接受则记录判定，翻转则另开一篇并互链）。
- 任何「其实有 ≥3 种来源」的反驳，必须给出 `crates/engine` 里第三种整句来源的实际生产者（新的 `CandidateSource::*`，或现枚举里被漏计的整句行）作为证据。
- 无代码改动时，本篇的验证是 `pnpm run verify-notes` 通过。

## Risks

- **判定会过时。** 一旦接入 octagram/语法搭配模型，或为桌面神经新增可区分的来源，来源数就 ≥3，本判定失效，需要重开。
- **判别口径依赖 wire 枚举。** 徽章若要落地，来源判别只能读 `CandidateSource`；将来新增来源时必须同步维护冻结的 wire 值和标签映射，漏一处就会出现无名标签或错标。
- **移动端有独立诉求。** 触屏宿主的键盘神经行确实产生 2 种来源；若只按桌面结论一刀切，可能压下移动端后续的真实需求。
