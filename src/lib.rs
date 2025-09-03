#![allow(dead_code)]

mod arc;
mod channel;
pub mod condvar;
mod lfqueue;
pub mod mutex;
mod one_shot_ch;
pub mod rwlock;
mod spin_lock;
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
