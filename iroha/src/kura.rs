//! This module contains persistence related Iroha logic.
//! `Kura` is the main entity which should be used to store new `Block`s on the blockchain.

use crate::{merkle::MerkleTree, prelude::*};
use async_std::{
    fs::{metadata, File},
    prelude::*,
};
use iroha_derive::log;
use std::{
    convert::TryFrom,
    fs,
    path::{Path, PathBuf},
};

/// High level data storage representation.
/// Provides all necessary methods to read and write data, hides implementation details.
#[derive(Debug)]
pub struct Kura {
    mode: Mode,
    blocks: Vec<ValidBlock>,
    block_store: BlockStore,
    block_sender: CommittedBlockSender,
    merkle_tree: MerkleTree,
}

impl Kura {
    /// Default `Kura` constructor.
    /// Kura will not be ready to work with before `init` method invocation.
    pub fn new(mode: Mode, block_store_path: &Path, block_sender: CommittedBlockSender) -> Self {
        Kura {
            mode,
            block_store: BlockStore::new(block_store_path),
            block_sender,
            merkle_tree: MerkleTree::new(),
            blocks: Vec::new(),
        }
    }

    /// After constructing `Kura` it should be initialized to be ready to work with it.
    pub async fn init(&mut self) -> Result<(), String> {
        let blocks = self.block_store.read_all().await;
        let blocks_refs = blocks.iter().collect::<Vec<&ValidBlock>>();
        self.merkle_tree.build(&blocks_refs);
        self.blocks = blocks;
        Ok(())
    }

    /// Methods consumes new validated block and atomically stores and caches it.
    #[log]
    pub async fn store(&mut self, mut block: ValidBlock) -> Result<Hash, String> {
        if !self.blocks.is_empty() {
            let last_block_index = self.blocks.len() - 1;
            block.header.height = last_block_index as u64 + 1;
            block.header.previous_block_hash = self.blocks.as_mut_slice()[last_block_index].hash();
        }
        let block_store_result = self.block_store.write(&block).await;
        match block_store_result {
            Ok(hash) => {
                self.block_sender.send(block.clone().commit()).await;
                self.blocks.push(block);
                Ok(hash)
            }
            Err(error) => {
                let blocks = self.block_store.read_all().await;
                let blocks_refs = blocks.iter().collect::<Vec<&ValidBlock>>();
                self.merkle_tree.build(&blocks_refs);
                Err(error)
            }
        }
    }
}

/// Kura work mode.
#[derive(Debug)]
pub enum Mode {
    /// Strict validation of all blocks.
    Strict,
    /// Fast initialization with basic checks.
    Fast,
}

/// Representation of a consistent storage.
#[derive(Debug)]
struct BlockStore {
    path: PathBuf,
}

impl BlockStore {
    fn new(path: &Path) -> BlockStore {
        if fs::read_dir(path).is_err() {
            fs::create_dir_all(path).expect("Failed to create Block Store directory.");
        }
        BlockStore {
            path: path.to_path_buf(),
        }
    }

    fn get_block_filename(block_height: u64) -> String {
        format!("{}", block_height)
    }

    fn get_block_path(&self, block_height: u64) -> PathBuf {
        self.path.join(BlockStore::get_block_filename(block_height))
    }

    async fn write(&self, block: &ValidBlock) -> Result<Hash, String> {
        //filename is its height
        let path = self.get_block_path(block.header.height);
        match File::create(path).await {
            Ok(mut file) => {
                let hash = block.hash();
                let serialized_block: Vec<u8> = block.into();
                if let Err(error) = file.write_all(&serialized_block).await {
                    return Err(format!("Failed to write to storage file {}.", error));
                }
                Ok(hash)
            }
            Err(error) => Result::Err(format!("Failed to open storage file {}.", error)),
        }
    }

    async fn read(&self, height: u64) -> Result<ValidBlock, String> {
        let path = self.get_block_path(height);
        let mut file = File::open(&path).await.map_err(|_| "No file found.")?;
        let metadata = metadata(&path)
            .await
            .map_err(|_| "Unable to read metadata.")?;
        let mut buffer = vec![0; metadata.len() as usize];
        file.read(&mut buffer)
            .await
            .map_err(|_| "Buffer overflow.")?;
        Ok(ValidBlock::try_from(buffer).expect("Failed to read block from store."))
    }

    /// Returns a sorted vector of blocks starting from 0 height to the top block.
    async fn read_all(&self) -> Vec<ValidBlock> {
        let mut height = 0;
        let mut blocks = Vec::new();
        while let Ok(block) = self.read(height).await {
            blocks.push(block);
            height += 1;
        }
        blocks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer::PeerId;
    use async_std::sync;
    use tempfile::TempDir;

    #[async_std::test]
    async fn strict_init_kura() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir.");
        let (tx, _rx) = sync::channel(100);
        assert!(Kura::new(Mode::Strict, temp_dir.path(), tx)
            .init()
            .await
            .is_ok());
    }

    #[async_std::test]
    async fn write_block_to_block_store() {
        let dir = tempfile::tempdir().unwrap();
        let block = PendingBlock::new(Vec::new())
            .chain_first()
            .sign(&[0; 32], &[0; 64])
            .expect("Failed to sign blocks.")
            .validate(&WorldStateView::new(Peer::new(
                PeerId {
                    address: "127.0.0.1:8080".to_string(),
                    public_key: [0; 32],
                },
                &Vec::new(),
            )))
            .expect("Failed to validate block.");
        assert!(BlockStore::new(dir.path()).write(&block).await.is_ok());
    }

    #[async_std::test]
    async fn read_block_from_block_store() {
        let dir = tempfile::tempdir().unwrap();
        let block = PendingBlock::new(Vec::new())
            .chain_first()
            .sign(&[0; 32], &[0; 64])
            .expect("Failed to sign blocks.")
            .validate(&WorldStateView::new(Peer::new(
                PeerId {
                    address: "127.0.0.1:8080".to_string(),
                    public_key: [0; 32],
                },
                &Vec::new(),
            )))
            .expect("Failed to validate block.");
        let block_store = BlockStore::new(dir.path());
        block_store
            .write(&block)
            .await
            .expect("Failed to write block to file.");
        assert!(block_store.read(0).await.is_ok())
    }

    #[async_std::test]
    async fn read_all_blocks_from_block_store() {
        let dir = tempfile::tempdir().unwrap();
        let block_store = BlockStore::new(dir.path());
        let n = 10;
        let mut block = PendingBlock::new(Vec::new())
            .chain_first()
            .sign(&[0; 32], &[0; 64])
            .expect("Failed to sign blocks.")
            .validate(&WorldStateView::new(Peer::new(
                PeerId {
                    address: "127.0.0.1:8080".to_string(),
                    public_key: [0; 32],
                },
                &Vec::new(),
            )))
            .expect("Failed to validate block.");
        for height in 0..n {
            let hash = block_store
                .write(&block)
                .await
                .expect("Failed to write block to file.");
            block = PendingBlock::new(Vec::new())
                .chain(height + 1, hash)
                .sign(&[0; 32], &[0; 64])
                .expect("Failed to sign blocks.")
                .validate(&WorldStateView::new(Peer::new(
                    PeerId {
                        address: "127.0.0.1:8080".to_string(),
                        public_key: [0; 32],
                    },
                    &Vec::new(),
                )))
                .expect("Failed to validate block.");
        }
        let blocks = block_store.read_all().await;
        assert_eq!(blocks.len(), n as usize)
    }

    ///Kura takes as input blocks, which comprise multiple transactions. Kura is meant to take only
    ///blocks as input that have passed stateless and stateful validation, and have been finalized
    ///by consensus. For finalized blocks, Kura simply commits the block to the block storage on
    ///the block_store and updates atomically the in-memory hashmaps that make up the key-value store that
    ///is the world-state-view. To optimize networking syncing, which works on 100 block chunks,
    ///chunks of 100 blocks each are stored in files in the block store.
    #[async_std::test]
    async fn store_block() {
        let block = PendingBlock::new(Vec::new())
            .chain_first()
            .sign(&[0; 32], &[0; 64])
            .expect("Failed to sign blocks.")
            .validate(&WorldStateView::new(Peer::new(
                PeerId {
                    address: "127.0.0.1:8080".to_string(),
                    public_key: [0; 32],
                },
                &Vec::new(),
            )))
            .expect("Failed to validate block.");
        let dir = tempfile::tempdir().unwrap();
        let (tx, _rx) = sync::channel(100);
        let mut kura = Kura::new(Mode::Strict, dir.path(), tx);
        kura.init().await.expect("Failed to init Kura.");
        kura.store(block)
            .await
            .expect("Failed to store block into Kura.");
    }
}
