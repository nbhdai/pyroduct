use anyhow::Result;
use pyro_artifacts::cache::CacheManager;

pub async fn init() -> Result<()> {
    let cache = CacheManager::from_env().await?;
    cache.init().await?;
    println!("Cache initialized successfully.");
    Ok(())
}

pub async fn purge() -> Result<()> {
    let cache = CacheManager::from_env().await?;
    cache.purge().await?;
    println!("Cache purged successfully.");
    Ok(())
}
