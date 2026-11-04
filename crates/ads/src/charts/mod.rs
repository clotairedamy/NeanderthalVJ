//! Hand-drawn charts (bars, stacked bar, sparkline, line, histogram, scatter,
//! gauge, heatmap), the engagement Gantt chart, and the chart chooser.

pub mod bar;
pub mod chart_choice;
pub mod gantt;
pub mod gauge;
pub mod heatmap;
pub mod histogram;
pub mod line;
pub mod scatter;
pub mod sparkline;
pub mod stacked;

pub use bar::AdsBars;
pub use chart_choice::{ChartChoice, ChartChoiceRow};
pub use gantt::{GanttChart, GanttPhase};
pub use gauge::AdsGauge;
pub use heatmap::AdsHeatmap;
pub use histogram::AdsHistogram;
pub use line::AdsLineChart;
pub use scatter::{AdsScatter, ScatterSeries};
pub use sparkline::AdsSparkline;
pub use stacked::{AdsStackedBar, Segment};
