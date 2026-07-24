use validator_engine::harness::{load_exam_equality_golden_cases, run_exam_equality_benchmark};

fn main() {
    let cases = load_exam_equality_golden_cases().expect("exam equality golden cases");
    let metrics = run_exam_equality_benchmark(&cases);

    println!("Exam LLM-equality benchmark (mock markers, Type H golden set)");
    println!("Total cases: {}", metrics.total_cases);
    println!(
        "Mode A false-fails: {} ({:.1}%)",
        metrics.mode_a_false_fails,
        metrics.mode_a_false_fail_rate * 100.0
    );
    println!(
        "Mode A+B false-fails: {} ({:.1}%)",
        metrics.mode_ab_false_fails,
        metrics.mode_ab_false_fail_rate * 100.0
    );
    println!(
        "Mode A precision/recall: {:.3} / {:.3}",
        metrics.mode_a_precision, metrics.mode_a_recall
    );
    println!(
        "Mode A+B precision/recall: {:.3} / {:.3}",
        metrics.mode_ab_precision, metrics.mode_ab_recall
    );

    let pass_labeled = cases
        .iter()
        .filter(|c| c.label.eq_ignore_ascii_case("pass"))
        .count();
    let fail_labeled = metrics.total_cases - pass_labeled;
    println!("Labeled pass: {pass_labeled}, labeled fail: {fail_labeled}");

    println!("\n=== Per-case (A vs A+B) ===");
    for (case, result) in cases.iter().zip(metrics.case_results.iter()) {
        let a = if result.mode_a_passed { "PASS" } else { "FAIL" };
        let ab = if result.mode_ab_passed {
            "PASS"
        } else {
            "FAIL"
        };
        let mode = case
            .answer_verification_mode
            .as_deref()
            .unwrap_or("exact_then_llm");
        println!(
            "  {} label={} mode={} exact={} llm={} | A={} A+B={}",
            case.id, case.label, mode, case.expected_exact, case.expected_llm, a, ab
        );
    }

    if metrics.mode_ab_false_fail_rate < metrics.mode_a_false_fail_rate {
        println!("\nRecommendation: A+B reduces false-fail rate on golden set.");
        println!(
            "Default EXAM_LLM_EQUALITY=0 remains safe; enable per-environment after manual real-LLM smoke."
        );
    } else if metrics.mode_ab_false_fail_rate == metrics.mode_a_false_fail_rate {
        println!("\nRecommendation: A+B matches A on this set; keep EXAM_LLM_EQUALITY=0 default.");
    } else {
        println!(
            "\nRecommendation: A+B did not improve metrics; keep EXAM_LLM_EQUALITY=0 default."
        );
    }
}
