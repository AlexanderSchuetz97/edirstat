use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use arc_swap::ArcSwap;
use crossbeam::channel::Receiver;

use super::{
    arena::{FileArenaSnapshot, FileNode, NO_INDEX, StringPool},
    traversal::{LocalId, ScanEvent},
};

pub struct SharedState {
    /// Atomic pointer to the latest immutable snapshot of the tree
    pub current_snapshot: ArcSwap<FileArenaSnapshot>,
    /// Indicates whether the scanner is actively running
    pub is_scanning: Arc<AtomicBool>,
}

impl Default for SharedState {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedState {
    #[must_use]
    pub fn new() -> Self {
        let initial_snapshot = FileArenaSnapshot {
            nodes: Arc::new(Vec::new()),
            string_pool: Arc::new(StringPool::new()),
        };
        Self {
            current_snapshot: ArcSwap::new(Arc::new(initial_snapshot)),
            is_scanning: Arc::new(AtomicBool::new(false)),
        }
    }
}

pub struct Coordinator {
    /// Lock-free channel to receive events
    event_rx: Receiver<Vec<ScanEvent>>,
    /// Shared state wrapper for swapping snapshots
    shared_state: Arc<SharedState>,
}

impl Coordinator {
    pub const fn new(event_rx: Receiver<Vec<ScanEvent>>, shared_state: Arc<SharedState>) -> Self {
        Self {
            event_rx,
            shared_state,
        }
    }

    pub fn run_coordinator_loop(&mut self, root_path_str: &str) {
        self.shared_state.is_scanning.store(true, Ordering::SeqCst);

        let mut arena = Vec::with_capacity(1024 * 1024); // Pre-allocate for ~1M nodes
        let mut string_pool = StringPool::new();

        // Local to Global ID mapping: outer index is worker_id, inner is local_id.0
        let mut id_map: Vec<Vec<u32>> = Vec::new();

        // Track the last child inserted for each parent global index to ensure O(1) appends
        let mut last_child_map: Vec<u32> = Vec::new();

        // Register root directory node (Global ID 0)
        let root_name_id = string_pool.get_or_insert(root_path_str.as_bytes());
        let root_node = FileNode::new(root_name_id, None, true, false);
        arena.push(root_node);
        last_child_map.push(NO_INDEX);

        // Map root node: LocalId(0) for worker 0 is global index 0
        register_id(&mut id_map, 0, LocalId(0), 0);

        let mut last_publish = Instant::now();
        let publish_interval = Duration::from_millis(100);
        let mut dirty = false;

        while let Ok(batch) = self.event_rx.recv() {
            for event in batch {
                match event {
                    ScanEvent::DirDiscovered {
                        parent_worker_id,
                        child_worker_id,
                        local_parent_id,
                        local_child_id,
                        name,
                    } => {
                        // Resolve parent global index using the parent's creator worker ID
                        if let Some(parent_global_id) =
                            resolve_id(&id_map, parent_worker_id, local_parent_id)
                        {
                            let name_id = string_pool.get_or_insert(name.as_bytes());
                            let child_global_id = arena.len() as u32;

                            // Create the directory node
                            let dir_node =
                                FileNode::new(name_id, Some(parent_global_id), true, false);
                            arena.push(dir_node);
                            last_child_map.push(NO_INDEX);

                            // Map worker's local child ID to our global index using the child's creator worker ID
                            register_id(
                                &mut id_map,
                                child_worker_id,
                                local_child_id,
                                child_global_id,
                            );

                            // Connect child to sibling chain in O(1) using last_child_map
                            connect_child(
                                &mut arena,
                                &mut last_child_map,
                                parent_global_id,
                                child_global_id,
                            );

                            dirty = true;
                        }
                    }
                    ScanEvent::FileDiscovered {
                        parent_worker_id,
                        local_parent_id,
                        name,
                        size,
                        is_symlink,
                    } => {
                        if name.is_empty() && size == 0 {
                            // Directory completion signal
                            continue;
                        }

                        // Resolve parent global index using the parent's creator worker ID
                        if let Some(parent_global_id) =
                            resolve_id(&id_map, parent_worker_id, local_parent_id)
                        {
                            let name_id = string_pool.get_or_insert(name.as_bytes());
                            let file_global_id = arena.len() as u32;

                            // Create file node (parent pointer is set)
                            let mut file_node =
                                FileNode::new(name_id, Some(parent_global_id), false, is_symlink);
                            file_node.size = size;
                            arena.push(file_node);
                            last_child_map.push(NO_INDEX);

                            // Connect child to sibling chain in O(1)
                            connect_child(
                                &mut arena,
                                &mut last_child_map,
                                parent_global_id,
                                file_global_id,
                            );

                            // Propagate size upwards through parent indices
                            propagate_size(&mut arena, parent_global_id, size);

                            dirty = true;
                        }
                    }
                }
            }

            // Publish snapshot if dirty and interval elapsed
            if dirty && last_publish.elapsed() >= publish_interval {
                let snapshot = FileArenaSnapshot {
                    nodes: Arc::new(arena.clone()),
                    string_pool: Arc::new(string_pool.clone()),
                };
                self.shared_state.current_snapshot.store(Arc::new(snapshot));
                last_publish = Instant::now();
                dirty = false;
            }
        }

        // Final publish at completion
        let snapshot = FileArenaSnapshot {
            nodes: Arc::new(arena),
            string_pool: Arc::new(string_pool),
        };
        self.shared_state.current_snapshot.store(Arc::new(snapshot));
        self.shared_state.is_scanning.store(false, Ordering::SeqCst);
    }
}

#[inline]
fn register_id(id_map: &mut Vec<Vec<u32>>, worker_id: u8, local_id: LocalId, global_id: u32) {
    let w_idx = worker_id as usize;
    if w_idx >= id_map.len() {
        id_map.resize(w_idx + 1, Vec::new());
    }
    let l_idx = local_id.0 as usize;
    if l_idx >= id_map[w_idx].len() {
        id_map[w_idx].resize(l_idx + 1, NO_INDEX);
    }
    id_map[w_idx][l_idx] = global_id;
}

#[inline]
fn resolve_id(id_map: &[Vec<u32>], worker_id: u8, local_id: LocalId) -> Option<u32> {
    let w_idx = worker_id as usize;
    if w_idx < id_map.len() {
        let l_idx = local_id.0 as usize;
        if l_idx < id_map[w_idx].len() {
            let gid = id_map[w_idx][l_idx];
            if gid != NO_INDEX {
                return Some(gid);
            }
        }
    }
    None
}

#[inline]
fn connect_child(
    arena: &mut [FileNode],
    last_child_map: &mut [u32],
    parent_global_id: u32,
    child_global_id: u32,
) {
    let p_idx = parent_global_id as usize;
    let last_child = last_child_map[p_idx];

    if last_child == NO_INDEX {
        // This is the first child of the parent
        arena[p_idx].first_child = child_global_id;
    } else {
        // We have a previous child, attach as its next sibling
        arena[last_child as usize].next_sibling = child_global_id;
    }

    // Update the last child pointer for this parent
    last_child_map[p_idx] = child_global_id;
}

#[inline]
fn propagate_size(arena: &mut [FileNode], start_parent_idx: u32, size: u64) {
    let mut current_idx = Some(start_parent_idx);
    while let Some(idx) = current_idx {
        let node = &mut arena[idx as usize];
        node.size += size;
        node.file_count += 1;
        current_idx = node.parent_opt();
    }
}
