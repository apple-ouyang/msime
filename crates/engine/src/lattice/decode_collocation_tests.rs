//! octagram `.gram` 语法模型的逐边打分：有模型时词格解码被搭配强度纠正，
//! 模型缺失/损坏/标定系数为零时解码输出与现状逐字节一致。
use std::path::Path;
use std::sync::Arc;

use super::tests::{lookup, syllables, table, Table};
use super::*;
use crate::lattice::octagram::{encode, GramConfig, GramDb};

/// 「一名+医」强搭配的模型：词格按词频偏好「一生」，搭配分把它拉回「医生」。
fn fixture_db(dir: &Path) -> Arc<GramDb> {
    let path = crate::lattice::octagram::fixture::gram_file(
        dir,
        "collocation.gram",
        &[([encode("一名"), encode("医")].concat(), 200_000)],
    );
    GramDb::shared(&path).expect("fixture .gram loads")
}

fn rows() -> Table {
    table(&[
        ("yi'ming", &[("一名", 10_000)]),
        ("yi'sheng", &[("医生", 1_000), ("一生", 2_000)]),
    ])
}

fn decode_with(
    rows: &Table,
    scorer: Option<&CollocationScorer<'_>>,
    weight: f64,
) -> Vec<SentencePath> {
    let options = LatticeOptions {
        collocation_scorer: scorer,
        collocation_weight: weight,
        ..LatticeOptions::default()
    };
    decode_sentences(&syllables("yi'ming'yi'sheng"), &mut lookup(rows), &options)
}

#[test]
fn a_gram_model_corrects_the_frequency_winner() {
    let dir = tempfile::tempdir().unwrap();
    let rows = rows();
    let baseline = decode_with(&rows, None, 0.0);
    assert_eq!(
        baseline[0].sentence, "一名一生",
        "没有语法模型时词频首选是串错句"
    );

    let db = fixture_db(dir.path());
    let config = GramConfig::default();
    let scorer = |tail: &str, word: &str, is_rear: bool| {
        db.query(tail, word, is_rear, &config) * std::f64::consts::LOG10_E
    };
    let scored = decode_with(&rows, Some(&scorer), 1.0);
    assert_eq!(
        scored[0].sentence, "一名医生",
        "「一名+医」的搭配项应把医生顶回首位"
    );
}

#[test]
fn no_gram_model_decodes_byte_identical_to_baseline() {
    let rows = rows();
    let baseline = decode_with(&rows, None, 0.0);
    // 标定系数为 0：scorer 即便在场也被短路，分位完全一致。
    let scorer = |_: &str, _: &str, _: bool| 9.9;
    let weightless = decode_with(&rows, Some(&scorer), 0.0);
    assert_eq!(
        baseline, weightless,
        "scorer 在场但 weight=0 时输出须逐字节一致"
    );
}

#[test]
fn missing_or_corrupt_gram_falls_back_to_baseline() {
    let dir = tempfile::tempdir().unwrap();
    let rows = rows();
    let baseline = decode_with(&rows, None, 0.0);

    // 缺失路径：`GramDb::load` 回 `None`，装配方拿不到 scorer，解码退回基线。
    let missing = dir.path().join("absent.gram");
    assert!(GramDb::load(&missing).is_none());

    // 损坏的镜像同样回 `None`，不 panic。
    let corrupt = dir.path().join("corrupt.gram");
    std::fs::write(&corrupt, b"Rime::Grammar/\xFF\xFF\xFF").unwrap();
    assert!(GramDb::load(&corrupt).is_none());

    // 装配方视角：拿到的 `Option<GramDb>` 为 `None` 时 scorer 就是 `None`。
    let db = GramDb::shared(&missing);
    let config = GramConfig::default();
    let scorer = db.map(|db| {
        move |tail: &str, word: &str, is_rear: bool| {
            db.query(tail, word, is_rear, &config) * std::f64::consts::LOG10_E
        }
    });
    let scorer = scorer.as_ref().map(|f| f as &CollocationScorer<'_>);
    let fallen = decode_with(&rows, scorer, 1.0);
    assert_eq!(baseline, fallen, "模型缺失时解码输出与基线逐字节一致");
    GramDb::evict_shared(&missing);
}
