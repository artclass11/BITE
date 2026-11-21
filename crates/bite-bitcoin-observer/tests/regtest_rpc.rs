use bite_bitcoin_observer::{
    BitcoinCoreObserver, ChainTracker, ChainUpdate, ObserverConfig, BlockRef,
};
use corepc_client::bitcoin::{BlockHash, Txid};
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the isolated BITE Bitcoin Core regtest container"]
async fn observes_blocks_transactions_confirmations_and_a_real_reorg() {
    let rpc_url = std::env::var("BITE_RPC_URL").expect("BITE_RPC_URL");
    let cookie = PathBuf::from(std::env::var("BITE_RPC_COOKIE_FILE").expect("BITE_RPC_COOKIE_FILE"));
    let cli = PathBuf::from(std::env::var("BITE_BITCOIN_CLI").expect("BITE_BITCOIN_CLI"));
    let address = std::env::var("BITE_REGTEST_ADDRESS").expect("BITE_REGTEST_ADDRESS");
    let block_height = std::env::var("BITE_TEST_BLOCK_HEIGHT")
        .expect("BITE_TEST_BLOCK_HEIGHT")
        .parse::<u64>()
        .expect("numeric block height");
    let block_hash = BlockHash::from_str(
        &std::env::var("BITE_TEST_BLOCK_HASH").expect("BITE_TEST_BLOCK_HASH"),
    )
    .expect("valid block hash");
    let txid = Txid::from_str(&std::env::var("BITE_TEST_TXID").expect("BITE_TEST_TXID"))
        .expect("valid txid");

    let observer = BitcoinCoreObserver::connect(
        ObserverConfig::new(rpc_url, cookie, 12).expect("observer config"),
    )
    .await
    .expect("connect to regtest");

    let tip_before = observer.chain_tip().await.expect("initial chain tip");
    assert!(tip_before.height >= block_height);
    let block = observer.observe_block(block_height).await.expect("observe block");
    assert_eq!(block.hash, block_hash);
    assert!(block.transaction_ids.contains(&txid));

    let tx = observer
        .observe_transaction(txid, block_hash)
        .await
        .expect("observe confirmed transaction");
    assert_eq!(tx.block_height, block_height);
    assert!(tx.confirmations >= 1);

    let mut tracker = ChainTracker::new(12).expect("tracker");
    assert!(matches!(
        observer.sync_chain(&mut tracker).await.expect("initial sync"),
        ChainUpdate::Initialized { .. }
    ));

    let old_tip = observer.chain_tip().await.expect("tip before reorg");
    assert!(old_tip.height >= block_height + 2);
    let old_tip_block = observer
        .observe_block(old_tip.height)
        .await
        .expect("old active tip block");
    let old_tip_ref = BlockRef {
        height: old_tip_block.height,
        hash: old_tip_block.hash,
    };

    let invalidate = Command::new(&cli)
        .arg("invalidateblock")
        .arg(old_tip.hash.to_string())
        .output()
        .expect("run bitcoin-cli invalidateblock");
    assert!(
        invalidate.status.success(),
        "invalidateblock failed: {}",
        String::from_utf8_lossy(&invalidate.stderr)
    );

    let mine = Command::new(&cli)
        .arg("generatetoaddress")
        .arg("2")
        .arg(&address)
        .output()
        .expect("run bitcoin-cli generatetoaddress");
    assert!(
        mine.status.success(),
        "generatetoaddress failed: {}",
        String::from_utf8_lossy(&mine.stderr)
    );

    let update = observer.sync_chain(&mut tracker).await.expect("detect reorg");
    match update {
        ChainUpdate::Reorganized {
            common_ancestor,
            removed,
            added,
            tip,
        } => {
            assert_eq!(common_ancestor.height + 1, old_tip.height);
            assert_eq!(removed, vec![old_tip_ref]);
            assert_eq!(added.len(), 2);
            assert_eq!(tip.height, old_tip.height + 1);
        }
        other => panic!("expected the observer to detect a reorg, got {other:?}"),
    }
}
