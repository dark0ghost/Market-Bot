pub trait Strategy {
    async fn run<T, E>() -> Result<T, E>;
}


