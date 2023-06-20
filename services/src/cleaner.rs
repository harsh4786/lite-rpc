use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use crate::block_listenser::BlockListener;
use crate::tx_sender::TxSender;
use log::info;
use prometheus::core::GenericGauge;
use prometheus::{opts, register_int_gauge};
use solana_lite_rpc_core::block_store::BlockStore;
use tokio::task::JoinHandle;

lazy_static::lazy_static! {
    static ref BLOCKS_IN_BLOCKSTORE: GenericGauge<prometheus::core::AtomicI64> = register_int_gauge!(opts!("literpc_blocks_in_blockstore", "Number of blocks in blockstore")).unwrap();
}

/// Background worker which cleans up memory  
#[derive(Clone)]
pub struct Cleaner {
    tx_sender: TxSender,
    block_listenser: BlockListener,
    block_store: BlockStore,
}

impl Cleaner {
    pub fn new(
        tx_sender: TxSender,
        block_listenser: BlockListener,
        block_store: BlockStore,
    ) -> Self {
        Self {
            tx_sender,
            block_listenser,
            block_store,
        }
    }

    pub fn clean_tx_sender(&self, ttl_duration: Duration) {
        self.tx_sender.cleanup(ttl_duration);
    }

    /// Clean Signature Subscribers from Block Listeners
    pub fn clean_block_listeners(&self, ttl_duration: Duration) {
        self.block_listenser.clean(ttl_duration);
    }

    pub async fn clean_block_store(&self, ttl_duration: Duration) {
        self.block_store.clean(ttl_duration).await;
        BLOCKS_IN_BLOCKSTORE.set(self.block_store.number_of_blocks_in_store() as i64);
    }

    pub fn start(
        self,
        ttl_duration: Duration,
        exit_signal: Arc<AtomicBool>,
    ) -> JoinHandle<anyhow::Result<()>> {
        let mut ttl = tokio::time::interval(ttl_duration);

        tokio::spawn(async move {
            info!("Cleaning memory");

            loop {
                if exit_signal.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                ttl.tick().await;

                self.clean_tx_sender(ttl_duration);
                self.clean_block_listeners(ttl_duration);
                self.clean_block_store(ttl_duration).await;
            }
            Ok(())
        })
    }
}