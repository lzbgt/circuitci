use super::CircuitCiApp;
use super::waveform_bundles::{
    ScopeMonteCarloYieldSummaryRow, scope_monte_carlo_yield_summaries_csv,
    scope_monte_carlo_yield_summaries_markdown, scope_monte_carlo_yield_summary_rows,
};
use crate::reports::ValidationReport;
use eframe::egui;

impl CircuitCiApp {
    pub(super) fn monte_carlo_yield_results_panel(&mut self, ui: &mut egui::Ui) {
        let Some(report) = self.report.as_ref() else {
            return;
        };
        let rows = scope_monte_carlo_yield_summary_rows(Some(report));
        if rows.is_empty() {
            return;
        }
        egui::CollapsingHeader::new(format!("Monte Carlo Yield ({})", rows.len()))
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("{} sampled-yield summary row(s)", rows.len()));
                    if ui.button("Copy CSV").clicked() {
                        ui.ctx()
                            .copy_text(scope_monte_carlo_yield_summaries_csv(&rows));
                        self.status = "Copied Monte Carlo yield table CSV.".to_string();
                    }
                    if ui.button("Copy Markdown").clicked() {
                        ui.ctx()
                            .copy_text(scope_monte_carlo_yield_summaries_markdown(&rows));
                        self.status = "Copied Monte Carlo yield table Markdown.".to_string();
                    }
                });
                monte_carlo_margin_distribution_grid(ui, &rows);
                ui.separator();
                monte_carlo_yield_results_grid(ui, &rows);
            });
    }

    pub(super) fn has_monte_carlo_yield_summaries(&self) -> bool {
        report_has_monte_carlo_yield_summaries(self.report.as_ref())
    }
}

fn monte_carlo_margin_distribution_grid(
    ui: &mut egui::Ui,
    rows: &[ScopeMonteCarloYieldSummaryRow],
) {
    ui.strong("Margin distribution");
    egui::Grid::new("monte_carlo_margin_distribution")
        .num_columns(6)
        .striped(true)
        .show(ui, |ui| {
            ui.strong("Check");
            ui.strong("Probe");
            ui.strong("Yield");
            ui.strong("Samples");
            ui.strong("Percentiles");
            ui.strong("Distribution");
            ui.end_row();
            for row in rows {
                ui.monospace(&row.assertion);
                ui.monospace(&row.probe);
                ui.monospace(format!("{}%", row.yield_percent));
                ui.monospace(format!("{}/{}", row.passed_samples, row.evaluated_samples));
                ui.monospace(format!(
                    "{} / {} / {}",
                    row.p5_margin, row.p50_margin, row.p95_margin
                ));
                monte_carlo_margin_distribution_strip(ui, row);
                ui.end_row();
            }
        });
}

fn monte_carlo_yield_results_grid(ui: &mut egui::Ui, rows: &[ScopeMonteCarloYieldSummaryRow]) {
    egui::Grid::new("monte_carlo_yield_results")
        .num_columns(15)
        .striped(true)
        .show(ui, |ui| {
            ui.strong("Scenario");
            ui.strong("Sweep");
            ui.strong("Sample");
            ui.strong("Check");
            ui.strong("Probe");
            ui.strong("Pass");
            ui.strong("Yield");
            ui.strong("Passed");
            ui.strong("Failed");
            ui.strong("Mean");
            ui.strong("Sigma");
            ui.strong("P50");
            ui.strong("Worst");
            ui.strong("P5/P95");
            ui.strong("Inputs");
            ui.end_row();
            for row in rows {
                ui.monospace(&row.scenario);
                ui.monospace(&row.sweep);
                ui.monospace(&row.limiting_sample);
                ui.monospace(&row.assertion);
                ui.monospace(&row.probe);
                ui.label(if row.passed { "pass" } else { "fail" });
                ui.monospace(format!("{}%", row.yield_percent));
                ui.monospace(format!("{}/{}", row.passed_samples, row.evaluated_samples));
                ui.monospace(row.failed_samples.to_string());
                ui.monospace(&row.mean_margin);
                ui.monospace(&row.stddev_margin);
                ui.monospace(&row.p50_margin);
                ui.monospace(&row.min_margin);
                ui.monospace(format!("{} / {}", row.p5_margin, row.p95_margin));
                ui.label(&row.inputs);
                ui.end_row();
            }
        });
}

fn monte_carlo_margin_distribution_strip(ui: &mut egui::Ui, row: &ScopeMonteCarloYieldSummaryRow) {
    let Some(distribution) = monte_carlo_margin_distribution(row) else {
        ui.label("n/a");
        return;
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(220.0, 24.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let center_y = rect.center().y;
    let x_at = |position: f32| rect.left() + rect.width() * position;
    let min_x = x_at(distribution.min);
    let max_x = x_at(distribution.max);
    let p5_x = x_at(distribution.p5);
    let p50_x = x_at(distribution.p50);
    let p95_x = x_at(distribution.p95);
    painter.rect_filled(rect, 3.0, egui::Color32::from_gray(24));
    painter.line_segment(
        [egui::pos2(min_x, center_y), egui::pos2(max_x, center_y)],
        egui::Stroke::new(1.0, egui::Color32::from_gray(120)),
    );
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(p5_x.min(p95_x), rect.top() + 6.0),
            egui::pos2(p5_x.max(p95_x), rect.bottom() - 6.0),
        ),
        2.0,
        if row.passed {
            egui::Color32::from_rgb(75, 150, 92)
        } else {
            egui::Color32::from_rgb(181, 90, 82)
        },
    );
    for x in [min_x, max_x] {
        painter.line_segment(
            [
                egui::pos2(x, rect.top() + 4.0),
                egui::pos2(x, rect.bottom() - 4.0),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_gray(168)),
        );
    }
    if let Some(zero) = distribution.zero {
        let zero_x = x_at(zero);
        painter.line_segment(
            [
                egui::pos2(zero_x, rect.top() + 2.0),
                egui::pos2(zero_x, rect.bottom() - 2.0),
            ],
            egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 196, 87)),
        );
    }
    painter.line_segment(
        [
            egui::pos2(p50_x, rect.top() + 3.0),
            egui::pos2(p50_x, rect.bottom() - 3.0),
        ],
        egui::Stroke::new(2.0, egui::Color32::from_rgb(104, 214, 255)),
    );
    response.on_hover_text(format!(
        "min {} | P5 {} | P50 {} | P95 {} | max {}; yellow = zero margin",
        row.min_margin, row.p5_margin, row.p50_margin, row.p95_margin, row.max_margin
    ));
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MonteCarloMarginDistribution {
    min: f32,
    p5: f32,
    p50: f32,
    p95: f32,
    max: f32,
    zero: Option<f32>,
}

fn monte_carlo_margin_distribution(
    row: &ScopeMonteCarloYieldSummaryRow,
) -> Option<MonteCarloMarginDistribution> {
    let min = row.min_margin_value?;
    let max = row.max_margin_value?;
    let p5 = row.p5_margin_value?;
    let p50 = row.p50_margin_value?;
    let p95 = row.p95_margin_value?;
    let range_min = min.min(0.0);
    let range_max = max.max(0.0);
    if ![range_min, range_max, p5, p50, p95]
        .iter()
        .all(|value| value.is_finite())
    {
        return None;
    }
    let span = range_max - range_min;
    if span <= f64::EPSILON {
        return None;
    }
    let normalized = |value: f64| ((value - range_min) / span).clamp(0.0, 1.0) as f32;
    Some(MonteCarloMarginDistribution {
        min: normalized(min),
        p5: normalized(p5),
        p50: normalized(p50),
        p95: normalized(p95),
        max: normalized(max),
        zero: (range_min <= 0.0 && range_max >= 0.0).then(|| normalized(0.0)),
    })
}

pub(super) fn report_has_monte_carlo_yield_summaries(report: Option<&ValidationReport>) -> bool {
    report
        .map(|report| !scope_monte_carlo_yield_summary_rows(Some(report)).is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{monte_carlo_margin_distribution, report_has_monte_carlo_yield_summaries};
    use crate::reports::{Finding, ValidationReport};
    use serde_json::json;

    #[test]
    fn monte_carlo_yield_detection_uses_loaded_report_findings() {
        let mut finding =
            Finding::critical("ANALOG_MONTE_CARLO_YIELD_SUMMARY", "mc_run", "summary");
        finding
            .measured
            .insert("assertion".to_string(), json!("gain_margin"));
        let report = ValidationReport::from_parts(
            "project".to_string(),
            "profile".to_string(),
            vec![finding],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            "validate".to_string(),
        );

        assert!(report_has_monte_carlo_yield_summaries(Some(&report)));
        assert!(!report_has_monte_carlo_yield_summaries(None));
    }

    #[test]
    fn monte_carlo_margin_distribution_normalizes_percentile_markers() {
        let mut finding = Finding::info("ANALOG_MONTE_CARLO_YIELD_SUMMARY", "mc_run", "summary");
        finding
            .measured
            .insert("assertion".to_string(), json!("gain_margin"));
        finding
            .measured
            .insert("min_margin".to_string(), json!(-2.0));
        finding
            .measured
            .insert("p5_margin".to_string(), json!(-1.0));
        finding
            .measured
            .insert("p50_margin".to_string(), json!(4.0));
        finding
            .measured
            .insert("p95_margin".to_string(), json!(7.0));
        finding
            .measured
            .insert("max_margin".to_string(), json!(8.0));
        let report = ValidationReport::from_parts(
            "project".to_string(),
            "profile".to_string(),
            vec![finding],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            "validate".to_string(),
        );
        let row = super::scope_monte_carlo_yield_summary_rows(Some(&report))
            .pop()
            .unwrap();
        let distribution = monte_carlo_margin_distribution(&row).unwrap();

        assert_eq!(distribution.min, 0.0);
        assert!((distribution.p5 - 0.1).abs() < 1.0e-6);
        assert!((distribution.p50 - 0.6).abs() < 1.0e-6);
        assert!((distribution.p95 - 0.9).abs() < 1.0e-6);
        assert_eq!(distribution.max, 1.0);
        assert_eq!(distribution.zero, Some(0.2));
    }

    #[test]
    fn monte_carlo_margin_distribution_requires_numeric_margins() {
        let mut finding = Finding::info("ANALOG_MONTE_CARLO_YIELD_SUMMARY", "mc_run", "summary");
        finding
            .measured
            .insert("assertion".to_string(), json!("gain_margin"));
        let report = ValidationReport::from_parts(
            "project".to_string(),
            "profile".to_string(),
            vec![finding],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            "validate".to_string(),
        );
        let row = super::scope_monte_carlo_yield_summary_rows(Some(&report))
            .pop()
            .unwrap();

        assert!(monte_carlo_margin_distribution(&row).is_none());
    }
}
