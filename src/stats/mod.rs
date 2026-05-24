pub mod dir_composition;
pub mod extension_boxplot;
pub mod scatter_plot;
pub mod size_distribution;
pub mod temporal_timeline;
pub mod treemap;

pub trait StatsChart {
    type Output;

    /// Iteratively computes or updates the chart's visual/plot data
    /// using the latest thread-safe snapshot frame.
    fn compute(&mut self, snapshot: &crate::arena::FileArenaSnapshot) -> Self::Output;
}
