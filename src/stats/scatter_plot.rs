pub struct FileAgeSizeScatterChart {
    pub top_files: Vec<(u32, u64)>, // (node_idx, size)
    pub max_timestamp: i64,
}

impl FileAgeSizeScatterChart {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            top_files: Vec::new(),
            max_timestamp: 0,
        }
    }
}

impl Default for FileAgeSizeScatterChart {
    fn default() -> Self {
        Self::new()
    }
}

impl super::StatsChart for FileAgeSizeScatterChart {
    type Output = ();

    fn compute(&mut self, snapshot: &crate::arena::FileArenaSnapshot) -> Self::Output {
        if snapshot.nodes.is_empty() {
            self.top_files.clear();
            self.max_timestamp = 0;
            return;
        }

        // 1. Establish the modern baseline using the most recent modification time
        let mut max_time = 0i64;
        for node in snapshot.nodes.iter() {
            if !node.is_directory() && node.modified_timestamp > max_time {
                max_time = node.modified_timestamp;
            }
        }
        self.max_timestamp = max_time;

        // 2. Gather all leaf nodes with a physical size
        let mut files: Vec<(u32, u64)> = snapshot
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| !node.is_directory() && node.size > 0)
            .map(|(idx, node)| (idx as u32, node.size))
            .collect();

        // 3. Sort descending to isolate the top 5,000 items
        files.sort_by_key(|b| std::cmp::Reverse(b.1));
        files.truncate(5000);

        self.top_files = files;
    }
}
