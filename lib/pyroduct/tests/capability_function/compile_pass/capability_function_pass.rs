use pyroduct::capability;

capability! {
    #[capability]
    fn simple(x: u32) -> u32 {
        x
    }

    #[capability]
    async fn simple_async(x: u32) -> u32 {
        x
    }

    #[capability]
    fn multiple(x: u32, y: String) -> u32 {
        x
    }

    #[capability]
    fn no_args() {}
}
fn main() {
    // Just verifying compilation
}
