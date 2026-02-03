// rkyv dependencies (assuming rkyv 0.8/rancor based on your imports)
use rkyv::rancor::Error;
use rkyv::ser::Writer;

use crate::LenAlignedVec;


impl Writer for LenAlignedVec {
    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.extend_from_slice(bytes);
        Ok(())
    }
}