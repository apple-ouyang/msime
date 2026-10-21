# Agent Note: Rime octagram `.gram` 语法模型读取端与逐边打分

Status: implemented

## Problem

旧版 Windows 正式版（`metasequoiaime/msime-windows` v0.11.1）支持下载 octagram 语法模型（`.gram`），整句解码时对每个词边叠加搭配分，能把词频串错句拉回正确搭配。新架构只有 bigram/trigram + 神经重排，没有这条路径。

## Decision

新增 `crates/engine/src/lattice/octagram.rs`：纯 Rust 只读 `.gram` 查找模块。文件头 32 字节格式名 + 校验和 + 双数组大小，`Rime::Grammar` 魔数；正文是 darts-clone 布局的 u32 单元数组，查询键按旧版自定义编码展开码点（ASCII 1 字节、0x4000–0xA000 两字节、其余 7bit 组变长，上限 8 个码点），值为 `ln(共现频次)×10000` 的 i32，负值按 -1 处理。解码侧把查询结果乘 `LOG10_E` 换到主分域。

`GramDb::shared` 经 `memmap2` 惰性映射并按路径共享 `Arc`；`load` 对缺失、截断、魔数不符、单元越界一律回 `None`，与 `msime-bigram.bin` 缺失的既有降级一致。`SentenceAssociationOptions` 新增 `collocation_model`/`collocation_weight`/`collocation_rerank`/`collocation_grammar` 四字段，全拼/双拼词典在 `set_sentence_association` 装载模型，`Option<GramDb>` 为 `None` 时打分器不装配。

解码端 `LatticeOptions` 新增 `collocation_scorer`/`collocation_weight`/`include_collocation_best`。束搜索对每个出边在 `collocation_scorer` 激活时叠加 `weight × scorer(尾窗, 词, is_rear)`；尾窗沿 `prev` 链即时回溯取最后 8 个码点，不增 `Hyp` 字段，未激活路径零开销。句尾词（`edge.end == n`）触发 `.gram` 的 `$` rear 项。`CandidateSource::Collocation` 追加在枚举末尾（wire 值 17，已有值不变）；`is_generated_or_fallback`/`is_sentence_learning`/`static_evidence` 把它按 Generated 对待，保证选中搭配句仍走句子学习。

合并端：`include_collocation_best` 开时首条词格行标 `Collocation`（它是解码最优路径，不是独立来源）；`include_lattice_best` 只在搭配开关同开时才让首行之后的备选继续上榜——无重排通道本来忽略它，基线语义不动。九键词典设置透传新字段，`neural_keyboard` 仍强制关。`series_slot_key` 的参数改为引用，避免 `SentenceAssociationOptions` 失去 `Copy` 后的无谓克隆；grammar 字段不入键——模型路径变化会触发 `set_sentence_association` 里的 `reset_cache`，同一路径下模型内容不可变。

许可：万象 LTS = CC-BY-4.0（可用）；八股文 = LGPL（可用，后续资源包切片需在 NOTICE 注明）；白霜 = GPL-3.0 且构建链许可未声明，本轮排除，进资源包前须重新评估。本切片只读 `.gram` 格式（BSD-3 的 librime-octagram + darts-clone 布局），不分发模型。

## Alternatives considered

- `Hyp` 内联 33 字节尾窗（每个假设固定 40 字节增量）：会让冻结参考实现与现行实现的内存断言全部失真，改为激活时沿 `prev` 链回溯几跳取尾窗，未激活零成本。
- 搭配分后置重排：只换顶层候选顺序，不进入束内路径选择，弱于逐边打分且多一次遍历，不采用。
- 每次查询直接 `GramDb::load`：文件映射重复开销，采用 `shared` 共享；删除模型包后的解除由后续资源包切片调 `evict_shared`。

## Testing

- `cargo test -p msime-engine` 2388 项全绿：octagram 单测覆盖编码边界、缺/损文件回 `None`、真实 darts 布局查询、上下文窗口与词首/句尾命中、弱搭配罚分；`decode_collocation_tests` 三项覆盖① 有 gram 时「一名一生」被纠正为「一名医生」、② scorer 在场但 weight=0 与无 scorer 输出逐字节一致、③ 缺/损文件经 `Option` 装配退回基线；`merge_collocation_tests` 三项覆盖 Collocation 标签、只留搭配行、双开关关时无重排基线照旧。
- `cargo fmt --all --check`、`cargo clippy -p msime-engine --all-targets --locked -- -D warnings` 干净；`cargo check -p msime-host-api -p msime-input-runtime -p msime-mcp-server --all-targets` 通过。
- 未验证：真实万象 `.gram` 文件端到端（无模型文件在手， fixture 由测试侧构造器按 darts 布局规则生成）；九键/全拼候选窗的 Collocation 行外观；偏好/资源包/设置页在后续切片。

## Consequences

模型缺失或损坏时行为与改动前逐字节一致；有模型时词格解码首选可被搭配分改写并标为 `Collocation`。`SentenceAssociationOptions` 不再 `Copy`，host-api 结构体镜像处已同步默认值。后续切片（偏好枚举、资源包 sha256 下载校验、设置页单选）依赖这里的 `collocation_model` 字段与 `evict_shared`。
