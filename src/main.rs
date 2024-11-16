use core_affinity as core;
use hex;
use sha2::{
    digest::generic_array::{typenum::U32, GenericArray},
    Digest, Sha256,
};
use std::thread;

fn main() {
    let ids = core::get_core_ids().expect("get core ids");
    let total = ids.len();

    let username = "nickmonad/AppleM4Max/";
    let handles = ids
        .into_iter()
        .map(|id| {
            thread::spawn(move || {
                let mut minimum: Option<GenericArray<u8, U32>> = None;
                let mut with = 0;

                let n = total as i32;
                let b = id.id as i32;
                for i in 0..500_000_000 {
                    let nonce = b + (i * n);
                    let preimage: Vec<u8> =
                        [username.as_bytes(), &nonce.to_string().as_bytes()].concat();

                    let hash = Sha256::digest(&preimage);

                    // compare hash against minimum, and store
                    if let Some(min) = minimum {
                        if hash.into_iter().lt(min.into_iter()) {
                            minimum = Some(hash);
                            with = nonce;
                        }
                    } else {
                        // first pass, just apply
                        minimum = Some(hash);
                        with = nonce;
                    }
                }

                // all done, print minimum
                if let Some(min) = minimum {
                    println!("{} -> {}", hex::encode(min), with);
                }
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        handle.join().expect("join handle");
    }
}
