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
                monte_carlo_yield_results_grid(ui, &rows);
            });
    }

    pub(super) fn has_monte_carlo_yield_summaries(&self) -> bool {
        report_has_monte_carlo_yield_summaries(self.report.as_ref())
    }
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

pub(super) fn report_has_monte_carlo_yield_summaries(report: Option<&ValidationReport>) -> bool {
    report
        .map(|report| !scope_monte_carlo_yield_summary_rows(Some(report)).is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::report_has_monte_carlo_yield_summaries;
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
}
