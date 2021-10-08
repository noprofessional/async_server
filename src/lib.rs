#![allow(dead_code)]

pub mod epoller;
pub mod runtime;
pub mod error;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

}
