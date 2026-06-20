use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    sync::Arc,
};

use bytemuck::{Pod, Zeroable};

use super::arena::{FileNode, StringPool};

pub const FILE_VERSION: u16 = 2;
pub const ZSTD_COMPRESSION_LEVEL: i32 = 3;

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C, align(8))]
pub struct FileHeader {
    pub magic: [u8; 4],
    pub version: u16,
    _padding: u16,
    pub uncompressed_size: u64,
    pub node_count: u64,
    pub string_pool_offset: u64,
    pub string_pool_length: u64,
    pub reserved: [u64; 4], // 32 bytes of padding for future backward compatibility
}

#[derive(Debug)]
pub struct PersistentArena {
    /// Underlying raw heap-allocated decompressed payload
    decompressed_data: Vec<u8>,
    node_count: usize,
}

impl PersistentArena {
    #[must_use]
    pub const fn new(decompressed_data: Vec<u8>, node_count: usize) -> Self {
        Self {
            decompressed_data,
            node_count,
        }
    }

    #[must_use]
    #[inline]
    pub fn nodes(&self) -> &[FileNode] {
        let start = 0;
        let end = self.node_count * std::mem::size_of::<FileNode>();
        let bytes = &self.decompressed_data[start..end];
        bytemuck::cast_slice(bytes)
    }

    #[inline]
    pub fn nodes_mut(&mut self) -> &mut [FileNode] {
        let start = 0;
        let end = self.node_count * std::mem::size_of::<FileNode>();
        let bytes = &mut self.decompressed_data[start..end];
        bytemuck::cast_slice_mut(bytes)
    }
}

pub fn save_snapshot(
    nodes: &[FileNode],
    string_pool: &StringPool,
    path: &Path,
) -> Result<(), crate::EdirstatError> {
    let mut file = File::create(path)?;

    // Calculate offsets by exporting the interner
    let (arena_string, offsets) = string_pool.interner.clone().export_arena().map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Interner handle overflow during export",
        )
    })?;

    let nodes_size = std::mem::size_of_val(nodes);
    let offsets_size = offsets.len() * std::mem::size_of::<u32>();
    let bytes_count = arena_string.len();

    // Calculate uncompressed sizes of payload data segments
    let string_pool_length = 8 + offsets_size + 8 + bytes_count;
    let uncompressed_size = nodes_size + string_pool_length;

    // Create header with version 2 and 32 bytes of future-proofing reserved space
    let header = FileHeader {
        magic: *b"EDST",
        version: FILE_VERSION,
        _padding: 0,
        uncompressed_size: uncompressed_size as u64,
        node_count: nodes.len() as u64,
        string_pool_offset: nodes_size as u64,
        string_pool_length: string_pool_length as u64,
        reserved: [0; 4],
    };

    // Pre-allocate the raw uncompressed payload
    let mut raw_payload = Vec::with_capacity(uncompressed_size);
    raw_payload.write_all(bytemuck::cast_slice(nodes))?;
    raw_payload.write_all(&(offsets.len() as u64).to_le_bytes())?;
    raw_payload.write_all(bytemuck::cast_slice(&offsets))?;
    raw_payload.write_all(&(bytes_count as u64).to_le_bytes())?;
    raw_payload.write_all(arena_string.as_bytes())?;

    // Compress the payload
    let compressed_payload = zstd::encode_all(&raw_payload[..], ZSTD_COMPRESSION_LEVEL)?;

    // Write uncompressed header first
    file.write_all(bytemuck::bytes_of(&header))?;
    // Write compressed payload second
    file.write_all(&compressed_payload)?;

    file.sync_all()?;
    Ok(())
}

pub fn load_snapshot(path: &Path) -> Result<(PersistentArena, StringPool), crate::EdirstatError> {
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;

    if metadata.len() < 72 {
        return Err(crate::EdirstatError::HeaderTooSmall);
    }

    // Read the uncompressed header safely from disk
    let mut header_bytes = [0u8; 72];
    file.read_exact(&mut header_bytes)?;

    let header: &FileHeader = bytemuck::from_bytes(&header_bytes);
    if header.magic != *b"EDST" {
        return Err(crate::EdirstatError::InvalidMagic);
    }
    if header.version != FILE_VERSION {
        return Err(crate::EdirstatError::UnsupportedVersion(header.version));
    }

    // Read remaining compressed payload
    let mut compressed_payload = Vec::with_capacity((metadata.len() - 72) as usize);
    file.read_to_end(&mut compressed_payload)?;

    // Decompress directly into a new Vec<u8> using the exact uncompressed capacity from the header
    let decompressed_data =
        zstd::bulk::decompress(&compressed_payload, header.uncompressed_size as usize)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let node_count = header.node_count as usize;
    let expected_nodes_size = node_count * std::mem::size_of::<FileNode>();

    if decompressed_data.len() < expected_nodes_size {
        return Err(crate::EdirstatError::TruncatedNodes);
    }

    // Extract StringPool
    let sp_start = header.string_pool_offset as usize;
    let sp_end = sp_start + header.string_pool_length as usize;
    if decompressed_data.len() < sp_end {
        return Err(crate::EdirstatError::TruncatedStringPool);
    }

    let sp_slice = &decompressed_data[sp_start..sp_end];

    // Parse offset count (8 bytes)
    let mut offset_count_bytes = [0u8; 8];
    offset_count_bytes.copy_from_slice(&sp_slice[0..8]);
    let offsets_count = u64::from_le_bytes(offset_count_bytes) as usize;

    // Parse u32 offsets array
    let offsets_start = 8;
    let offsets_end = offsets_start + offsets_count * std::mem::size_of::<u32>();
    if sp_slice.len() < offsets_end + 8 {
        return Err(crate::EdirstatError::TruncatedStringPool);
    }
    let offsets_bytes = &sp_slice[offsets_start..offsets_end];
    let offsets: &[u32] = bytemuck::cast_slice(offsets_bytes);

    // Parse bytes count (8 bytes)
    let mut bytes_count_bytes = [0u8; 8];
    bytes_count_bytes.copy_from_slice(&sp_slice[offsets_end..offsets_end + 8]);
    let bytes_count = u64::from_le_bytes(bytes_count_bytes) as usize;

    // Parse raw bytes
    let raw_bytes_start = offsets_end + 8;
    let raw_bytes_end = raw_bytes_start + bytes_count;
    if sp_slice.len() < raw_bytes_end {
        return Err(crate::EdirstatError::TruncatedStringPool);
    }
    let raw_bytes = &sp_slice[raw_bytes_start..raw_bytes_end];

    // Reconstruct the interner allocation-free
    let arena_data: Arc<str> = Arc::from(std::str::from_utf8(raw_bytes).unwrap_or(""));
    let mut interner = xgx_intern::Interner::new(ahash::RandomState::new());

    for i in 0..offsets.len() - 1 {
        let offset = offsets[i];
        let len = offsets[i + 1] - offset;
        let shared_str = xgx_intern::ArenaString::Shared {
            arena: arena_data.clone(),
            offset,
            len,
        };
        let _ = interner.intern_owned(shared_str);
    }

    let string_pool = StringPool { interner };
    let arena = PersistentArena::new(decompressed_data, node_count);

    Ok((arena, string_pool))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_serialization_and_compressed_load() -> Result<(), crate::EdirstatError> {
        let mut pool = StringPool::new();
        let name_root = pool.get_or_insert(b"/");
        let name_dir = pool.get_or_insert(b"target");
        let name_file = pool.get_or_insert(b"lib.rs");

        let mut nodes = vec![
            FileNode::new(name_root, None, true, false, 0, 0, 0),
            FileNode::new(name_dir, Some(0), true, false, 0, 0, 0),
            FileNode::new(name_file, Some(1), false, false, 0, 0, 0),
        ];

        // Connect nodes
        nodes[0].first_child = 1;
        nodes[1].first_child = 2;
        nodes[1].size = 12345;
        nodes[1].file_count = 1;
        nodes[2].size = 12345;

        // Use a temporary test file path inside the workspace
        let temp_dir = std::env::current_dir()?.join("target");
        let test_path = temp_dir.join("test_snapshot.edst");
        let _ = std::fs::create_dir_all(&temp_dir);

        // Save snapshot
        save_snapshot(&nodes, &pool, &test_path)?;

        // Load snapshot via safe compressed reader
        let (mut loaded_arena, loaded_pool) = load_snapshot(&test_path)?;
        let loaded_nodes = loaded_arena.nodes();

        // Validate structure size & elements
        assert_eq!(loaded_nodes.len(), 3);
        assert_eq!(loaded_nodes[0].name_id, name_root);
        assert_eq!(loaded_nodes[1].name_id, name_dir);
        assert_eq!(loaded_nodes[2].name_id, name_file);

        assert_eq!(loaded_nodes[0].first_child, 1);
        assert_eq!(loaded_nodes[1].first_child, 2);
        assert_eq!(loaded_nodes[1].size, 12345);
        assert_eq!(loaded_nodes[2].size, 12345);

        // Validate string pool contents
        assert_eq!(loaded_pool.get(name_root), Some("/"));
        assert_eq!(loaded_pool.get(name_dir), Some("target"));
        assert_eq!(loaded_pool.get(name_file), Some("lib.rs"));

        // Validate Mutability (Verify we can modify slice data safely in memory)
        let loaded_nodes_mut = loaded_arena.nodes_mut();
        loaded_nodes_mut[1].next_sibling = 999;
        assert_eq!(loaded_nodes_mut[1].next_sibling, 999);

        // Clean up temporary file
        let _ = std::fs::remove_file(&test_path);
        Ok(())
    }

    #[test]
    fn test_load_snapshot_header_too_small() -> Result<(), crate::EdirstatError> {
        let temp_dir = std::env::current_dir()?.join("target");
        let test_path = temp_dir.join("test_small.edst");
        let _ = std::fs::create_dir_all(&temp_dir);
        std::fs::write(&test_path, b"too_small")?;

        let res = load_snapshot(&test_path);
        assert!(matches!(res, Err(crate::EdirstatError::HeaderTooSmall)));

        let _ = std::fs::remove_file(&test_path);
        Ok(())
    }

    #[test]
    fn test_load_snapshot_invalid_magic() -> Result<(), crate::EdirstatError> {
        let temp_dir = std::env::current_dir()?.join("target");
        let test_path = temp_dir.join("test_invalid_magic.edst");
        let _ = std::fs::create_dir_all(&temp_dir);

        let header = FileHeader {
            magic: *b"BAD!",
            version: FILE_VERSION,
            _padding: 0,
            uncompressed_size: 0,
            node_count: 0,
            string_pool_offset: 72,
            string_pool_length: 0,
            reserved: [0; 4],
        };
        std::fs::write(&test_path, bytemuck::bytes_of(&header))?;

        let res = load_snapshot(&test_path);
        assert!(matches!(res, Err(crate::EdirstatError::InvalidMagic)));

        let _ = std::fs::remove_file(&test_path);
        Ok(())
    }

    #[test]
    fn test_load_snapshot_unsupported_version() -> Result<(), crate::EdirstatError> {
        let temp_dir = std::env::current_dir()?.join("target");
        let test_path = temp_dir.join("test_unsupported_version.edst");
        let _ = std::fs::create_dir_all(&temp_dir);

        let header = FileHeader {
            magic: *b"EDST",
            version: 99,
            _padding: 0,
            uncompressed_size: 0,
            node_count: 0,
            string_pool_offset: 72,
            string_pool_length: 0,
            reserved: [0; 4],
        };
        std::fs::write(&test_path, bytemuck::bytes_of(&header))?;

        let res = load_snapshot(&test_path);
        assert!(matches!(
            res,
            Err(crate::EdirstatError::UnsupportedVersion(99))
        ));

        let _ = std::fs::remove_file(&test_path);
        Ok(())
    }
}
