use core_affinity::CoreId;
use crossbeam::channel;
use sha2::{
    digest::generic_array::{typenum::U32, GenericArray},
    Digest, Sha256,
};
use std::iter;

// hash array
pub type Hash = GenericArray<u8, U32>;

// nonce string with resulting hash value
pub type WithNonce = (Hash, String);

// Combined result with core ID, and an optional tuple of (nonce, hash).
// When the optional value is None, the core is reporting it has
// no more hashes to generate and check.
pub type Result = (CoreId, Option<WithNonce>);

// base64 alphabet
// as defined in https://datatracker.ietf.org/doc/html/rfc4648#section-4
static BASE64: [&str; 64] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z", "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l",
    "m", "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z", "0", "1", "2", "3", "4",
    "5", "6", "7", "8", "9", "+", "/",
];

pub fn worker(
    id: CoreId,
    n: usize,
    username: String,
    message: Option<String>,
    iterations: Option<u64>,
    results: Option<channel::Sender<Result>>,
) -> WithNonce {
    // allocate a string buffer with enough capacity to fill up to 64 _total_ chars
    #[rustfmt::skip]
    let capacity = if let Some(ref m) = message { 64 - m.len() } else { 64 };
    let mut noncebuf = String::with_capacity(capacity);

    // default minimum hash, set to max
    let mut min: WithNonce = (Hash::from_iter(iter::repeat(u8::MAX)), noncebuf.clone());

    // build prefix used in hash calculation
    let prefix = concat(username, message);
    let prefix: Vec<u8> = [&prefix.as_bytes(), "/".as_bytes()].concat();

    // allocate preimage buffer
    // and fix the prefix at the beginning of the buffer
    let mut preimage: Vec<u8> = Vec::with_capacity(prefix.len() + noncebuf.capacity());
    preimage.extend_from_slice(&prefix);

    let n = n as u64;
    let base = id.id as u64;

    for i in 0.. {
        // limit iterations
        if let Some(max) = iterations {
            if i == max {
                break;
            }
        }

        // calculate the base64 nonce value
        let mut nonce: u64 = base + (i * n);
        let mut fit = false;
        for _ in 0..capacity {
            let m: u64 = nonce / 64;
            let r = nonce % 64;

            noncebuf.push_str(BASE64[r as usize]);
            if m == 0 {
                fit = true;
                break;
            }

            nonce = nonce / 64;
        }

        if !fit {
            // nonce value overran buffer capacity!
            if let Some(ref r) = results {
                let _ = r.send((id, None));
            }

            return min;
        }

        let buf = noncebuf.as_bytes();
        for b in buf.iter() {
            preimage.push(*b);
        }

        let hash = Sha256::digest(&preimage[..(prefix.len() + buf.len())]);

        // compare to current minimum
        if is_less(&hash, &min.0) {
            min = (hash, noncebuf.clone());
            if let Some(ref r) = results {
                let _ = r.send((id, Some((hash, noncebuf.clone()))));
            }
        }

        // clear buffers for next iteration
        noncebuf.clear();
        preimage.truncate(prefix.len());
    }

    // all done
    if let Some(r) = results {
        let _ = r.send((id, None));
    }

    return min;
}

pub fn max() -> WithNonce {
    (Hash::from_iter(iter::repeat(u8::MAX)), "".into())
}

pub fn is_less(a: &Hash, b: &Hash) -> bool {
    a.iter().lt(b.iter())
}

pub fn concat(username: String, message: Option<String>) -> String {
    let mut result = username;
    if let Some(ref m) = message {
        result.push_str("/");
        result.push_str(m);
    }

    result
}
