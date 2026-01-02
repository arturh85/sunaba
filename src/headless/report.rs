//! HTML report generation for training results
//!
//! Generates visual reports with embedded GIFs and fitness charts.

use std::fs;

use anyhow::{Context, Result};

use super::map_elites::MapElitesGrid;
use super::scenario::ScenarioConfig;
use super::training_env::TrainingStats;

/// Generates HTML reports for training runs
pub struct ReportGenerator {
    /// Output directory
    output_dir: String,
    /// Scenario configuration
    scenario_config: ScenarioConfig,
}

impl ReportGenerator {
    /// Create a new report generator
    pub fn new(output_dir: &str, scenario_config: &ScenarioConfig) -> Self {
        Self {
            output_dir: output_dir.to_string(),
            scenario_config: scenario_config.clone(),
        }
    }

    /// Generate the final HTML report
    pub fn generate_final_report(
        &self,
        grid: &MapElitesGrid,
        stats_history: &[TrainingStats],
    ) -> Result<()> {
        // Create output directory
        fs::create_dir_all(&self.output_dir).context("Failed to create output directory")?;

        // Generate index.html
        let html = self.generate_html(grid, stats_history);
        let path = format!("{}/index.html", self.output_dir);
        fs::write(&path, html).context("Failed to write report HTML")?;

        // Generate summary JSON
        let json = self.generate_summary_json(grid, stats_history);
        let json_path = format!("{}/summary.json", self.output_dir);
        fs::write(&json_path, json).context("Failed to write summary JSON")?;

        log::info!("Report generated: {}", path);
        Ok(())
    }

    /// Generate the main HTML content
    fn generate_html(&self, grid: &MapElitesGrid, stats_history: &[TrainingStats]) -> String {
        let stats = grid.stats();
        let fitness_chart = self.generate_fitness_svg(stats_history);
        let grid_svg = self.generate_grid_svg(grid);

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Sunaba Training Report - {scenario_name}</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
            background: #1a1a2e;
            color: #eee;
        }}
        h1, h2, h3 {{ color: #4ecdc4; }}
        .header {{
            border-bottom: 2px solid #4ecdc4;
            padding-bottom: 20px;
            margin-bottom: 30px;
        }}
        .scenario-info {{
            background: #16213e;
            padding: 20px;
            border-radius: 10px;
            margin-bottom: 30px;
        }}
        .stats-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }}
        .stat-card {{
            background: #16213e;
            padding: 20px;
            border-radius: 10px;
            text-align: center;
        }}
        .stat-value {{
            font-size: 2em;
            font-weight: bold;
            color: #4ecdc4;
        }}
        .stat-label {{
            color: #888;
            font-size: 0.9em;
        }}
        .chart-container {{
            background: #16213e;
            padding: 20px;
            border-radius: 10px;
            margin-bottom: 30px;
        }}
        .grid-container {{
            background: #16213e;
            padding: 20px;
            border-radius: 10px;
            margin-bottom: 30px;
        }}
        svg {{
            display: block;
            margin: 0 auto;
        }}
        .description {{
            color: #aaa;
            line-height: 1.6;
        }}
    </style>
</head>
<body>
    <div class="header">
        <h1>🧬 Sunaba Training Report</h1>
        <p class="description">Creature evolution results</p>
    </div>

    <div class="scenario-info">
        <h2>Scenario: {scenario_name}</h2>
        <p class="description"><strong>Description:</strong> {scenario_desc}</p>
        <p class="description"><strong>Expected Behavior:</strong> {expected_behavior}</p>
        <p class="description"><strong>Evaluation Duration:</strong> {eval_duration}s per creature</p>
    </div>

    <div class="stats-grid">
        <div class="stat-card">
            <div class="stat-value">{generations}</div>
            <div class="stat-label">Generations</div>
        </div>
        <div class="stat-card">
            <div class="stat-value">{best_fitness:.2}</div>
            <div class="stat-label">Best Fitness</div>
        </div>
        <div class="stat-card">
            <div class="stat-value">{coverage:.1}%</div>
            <div class="stat-label">Grid Coverage</div>
        </div>
        <div class="stat-card">
            <div class="stat-value">{cell_count}</div>
            <div class="stat-label">Unique Behaviors</div>
        </div>
    </div>

    <div class="chart-container">
        <h3>Fitness Over Generations</h3>
        {fitness_chart}
    </div>

    <div class="grid-container">
        <h3>MAP-Elites Grid ({dim0} × {dim1})</h3>
        <p class="description">Each cell represents a unique behavioral niche. Brighter = higher fitness.</p>
        {grid_svg}
    </div>

    <footer style="text-align: center; color: #666; margin-top: 40px;">
        Generated by Sunaba Headless Training
    </footer>
</body>
</html>"#,
            scenario_name = self.scenario_config.name,
            scenario_desc = self.scenario_config.description,
            expected_behavior = self.scenario_config.expected_behavior,
            eval_duration = self.scenario_config.eval_duration,
            generations = stats_history.len(),
            best_fitness = stats.best_fitness,
            coverage = stats.coverage * 100.0,
            cell_count = stats.cell_count,
            fitness_chart = fitness_chart,
            grid_svg = grid_svg,
            dim0 = grid.dim0_name,
            dim1 = grid.dim1_name,
        )
    }

    /// Generate SVG chart for fitness over time
    fn generate_fitness_svg(&self, stats_history: &[TrainingStats]) -> String {
        if stats_history.is_empty() {
            return "<p>No data</p>".to_string();
        }

        let width = 600;
        let height = 200;
        let padding = 40;

        let max_fitness = stats_history
            .iter()
            .map(|s| s.best_fitness)
            .fold(0.0f32, f32::max);
        let max_fitness = if max_fitness > 0.0 { max_fitness } else { 1.0 };

        // Generate path for best fitness
        let mut best_path = String::new();
        let mut avg_path = String::new();

        for (i, stats) in stats_history.iter().enumerate() {
            let x = padding as f32
                + (i as f32 / stats_history.len() as f32) * (width - 2 * padding) as f32;
            let y_best = height as f32
                - padding as f32
                - (stats.best_fitness / max_fitness) * (height - 2 * padding) as f32;
            let y_avg = height as f32
                - padding as f32
                - (stats.avg_fitness / max_fitness) * (height - 2 * padding) as f32;

            if i == 0 {
                best_path.push_str(&format!("M{:.1},{:.1}", x, y_best));
                avg_path.push_str(&format!("M{:.1},{:.1}", x, y_avg));
            } else {
                best_path.push_str(&format!(" L{:.1},{:.1}", x, y_best));
                avg_path.push_str(&format!(" L{:.1},{:.1}", x, y_avg));
            }
        }

        format!(
            r#"<svg width="{width}" height="{height}" viewBox="0 0 {width} {height}">
    <!-- Grid -->
    <line x1="{padding}" y1="{padding}" x2="{padding}" y2="{y_bottom}" stroke="gray" stroke-width="1"/>
    <line x1="{padding}" y1="{y_bottom}" x2="{x_right}" y2="{y_bottom}" stroke="gray" stroke-width="1"/>

    <!-- Average fitness line -->
    <path d="{avg_path}" fill="none" stroke="gray" stroke-width="2"/>

    <!-- Best fitness line -->
    <path d="{best_path}" fill="none" stroke="cyan" stroke-width="2"/>

    <!-- Labels -->
    <text x="{padding}" y="{label_y}" fill="gray" font-size="12">0</text>
    <text x="{padding}" y="{top_label}" fill="gray" font-size="12">{max_fitness:.1}</text>
    <text x="{mid_x}" y="{xlabel_y}" fill="gray" font-size="12" text-anchor="middle">Generation</text>

    <!-- Legend -->
    <line x1="{legend_x}" y1="20" x2="{legend_x2}" y2="20" stroke="cyan" stroke-width="2"/>
    <text x="{legend_text}" y="24" fill="gray" font-size="12">Best</text>
    <line x1="{legend_x}" y1="35" x2="{legend_x2}" y2="35" stroke="gray" stroke-width="2"/>
    <text x="{legend_text}" y="39" fill="gray" font-size="12">Avg</text>
</svg>"#,
            width = width,
            height = height,
            padding = padding,
            y_bottom = height - padding,
            x_right = width - padding,
            avg_path = avg_path,
            best_path = best_path,
            label_y = height - padding + 15,
            top_label = padding - 5,
            max_fitness = max_fitness,
            mid_x = width / 2,
            xlabel_y = height - 5,
            legend_x = width - 100,
            legend_x2 = width - 80,
            legend_text = width - 75,
        )
    }

    /// Generate SVG visualization of MAP-Elites grid
    fn generate_grid_svg(&self, grid: &MapElitesGrid) -> String {
        let cell_size = 30;
        let fitness_grid = grid.as_fitness_grid();
        let resolution = fitness_grid.len();
        let width = resolution * cell_size + 60;
        let height = resolution * cell_size + 60;

        let stats = grid.stats();
        let max_fitness = if stats.best_fitness > 0.0 {
            stats.best_fitness
        } else {
            1.0
        };

        let mut cells = String::new();
        for (y, row) in fitness_grid.iter().enumerate() {
            for (x, &fitness) in row.iter().enumerate() {
                let px = 40 + x * cell_size;
                let py = 20 + (resolution - 1 - y) * cell_size; // Flip Y

                let color = if fitness < 0.0 {
                    "#222".to_string()
                } else {
                    let intensity = (fitness / max_fitness * 255.0) as u8;
                    format!("rgb({}, {}, {})", intensity / 2, intensity, intensity)
                };

                cells.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="gray" stroke-width="1"/>"#,
                    px, py, cell_size - 1, cell_size - 1, color
                ));
            }
        }

        format!(
            r#"<svg width="{width}" height="{height}" viewBox="0 0 {width} {height}">
    {cells}
    <!-- Axis labels -->
    <text x="20" y="{mid_y}" fill="gray" font-size="12" transform="rotate(-90, 20, {mid_y})">{dim1}</text>
    <text x="{mid_x}" y="{bottom}" fill="gray" font-size="12" text-anchor="middle">{dim0}</text>
</svg>"#,
            width = width,
            height = height,
            cells = cells,
            mid_y = height / 2,
            mid_x = width / 2,
            bottom = height - 5,
            dim0 = grid.dim0_name,
            dim1 = grid.dim1_name,
        )
    }

    /// Generate summary JSON
    fn generate_summary_json(
        &self,
        grid: &MapElitesGrid,
        stats_history: &[TrainingStats],
    ) -> String {
        let stats = grid.stats();

        format!(
            r#"{{
    "scenario": {{
        "name": "{}",
        "description": "{}",
        "expected_behavior": "{}",
        "eval_duration": {}
    }},
    "results": {{
        "generations": {},
        "best_fitness": {},
        "avg_fitness": {},
        "grid_coverage": {},
        "cell_count": {},
        "total_cells": {}
    }},
    "fitness_history": [{}]
}}"#,
            self.scenario_config.name,
            self.scenario_config.description,
            self.scenario_config.expected_behavior,
            self.scenario_config.eval_duration,
            stats_history.len(),
            stats.best_fitness,
            stats.avg_fitness,
            stats.coverage,
            stats.cell_count,
            stats.total_cells,
            stats_history
                .iter()
                .map(|s| format!("{:.2}", s.best_fitness))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_generator_creation() {
        let config = ScenarioConfig::default();
        let gen = ReportGenerator::new("test_output", &config);
        assert_eq!(gen.output_dir, "test_output");
    }
}
