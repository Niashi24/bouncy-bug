use alloc::vec::Vec;
use bevy_playdate::jobs::{load_file_bytes, AsyncLoadCtx};
use pd_asset::archive::{AlignVec, OwnedArchived};
use pd_asset::rkyv::api::high::HighValidator;
use pd_asset::rkyv::bytecheck::CheckBytes;
use pd_asset::rkyv::Portable;
use pd_asset::RkyvError;

pub async fn load_and_decompress(
    async_load_ctx: &mut AsyncLoadCtx,
    path: &str,
) -> anyhow::Result<Vec<u8>> {
    let bytes = load_file_bytes(async_load_ctx, path).await?;
    let bytes = lz4_flex::decompress_size_prepended(&bytes)?;

    Ok(bytes)
}

pub async fn load_compressed_archive<T>(
    async_load_ctx: &mut AsyncLoadCtx,
    path: &str,
) -> anyhow::Result<OwnedArchived<T>>
where
    T: Portable + for<'a> CheckBytes<HighValidator<'a, RkyvError>>,
{
    let bytes = load_and_decompress(async_load_ctx, path).await?;
    let mut aligned = AlignVec::with_capacity(bytes.len());
    aligned.extend_from_slice(&bytes);

    Ok(OwnedArchived::new(aligned)?)
}
