use core::fmt::{Debug, Formatter};
use core::marker::PhantomData;
use bytecheck::CheckBytes;
use rkyv::api::high::HighValidator;
use rkyv::Portable;
use rkyv::seal::Seal;
use rkyv::util::AlignedVec;
use crate::RkyvError;

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
        let _ = rkyv::access::<T, RkyvError>(&bytes)?;

        let out = Self(bytes, PhantomData);
        out.assert_aligned();

        Ok(out)
    }

    pub fn access(&self) -> &T {
        // SAFETY: Already checked that bytes are valid in `Self::new`
        unsafe { rkyv::access_unchecked(&self.0) }
    }

    pub fn access_mut(&mut self) -> Seal<'_, T> {
        // SAFETY: Already checked that bytes are valid in `Self::new`
        unsafe { rkyv::access_unchecked_mut(&mut self.0) }
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