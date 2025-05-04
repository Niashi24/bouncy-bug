use alloc::vec::Vec;
use bevy_playdate::jobs::{load_file_bytes, AsyncLoadCtx};
use core::fmt::{Debug, Formatter};
use core::marker::PhantomData;
use pd_asset::rkyv::api::high::HighValidator;
use pd_asset::rkyv::bytecheck::CheckBytes;
use pd_asset::rkyv::seal::Seal;
use pd_asset::rkyv::util::AlignedVec;
use pd_asset::rkyv::Portable;
use pd_asset::RkyvError;

const ALIGNMENT: usize = 16;
pub type AlignVec = AlignedVec<ALIGNMENT>;

#[derive(Clone)]
pub struct OwnedArchived<T>(AlignVec, PhantomData<T>)
where
    T: Portable + for<'a> CheckBytes<HighValidator<'a, RkyvError>>;

impl<T> Debug for OwnedArchived<T>
where
    T: Debug + Portable + for<'a> CheckBytes<HighValidator<'a, RkyvError>>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let archive = self.access();
        write!(f, "{:?}", archive)
    }
}

impl<T> OwnedArchived<T>
where
    T: Portable + for<'a> CheckBytes<HighValidator<'a, RkyvError>>,
{
    pub fn new(bytes: AlignVec) -> Result<Self, RkyvError> {
        let _ = pd_asset::rkyv::access::<T, RkyvError>(&bytes)?;

        let out = Self(bytes, PhantomData);
        out.assert_aligned();

        Ok(out)
    }

    pub fn access(&self) -> &T {
        // SAFETY: Already checked that bytes are valid in `Self::new`
        unsafe { pd_asset::rkyv::access_unchecked(&self.0) }
    }

    pub fn access_mut(&mut self) -> Seal<'_, T> {
        // SAFETY: Already checked that bytes are valid in `Self::new`
        unsafe { pd_asset::rkyv::access_unchecked_mut(&mut self.0) }
    }

    pub fn assert_aligned(&self) {
        assert_eq!(
            (self.0.as_ptr() as usize) % ALIGNMENT,
            0,
            "byte array was not aligned: {:x}",
            self.0.as_ptr() as usize
        );
    }

    pub fn bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

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
