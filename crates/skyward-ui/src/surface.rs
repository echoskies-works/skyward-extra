//! Where the UI's frames go.

use crate::canvas::Canvas;
use anyhow::Result;

/// A present target. The device backend scans out through KMS; the host
/// preview backend encodes PNGs. Keeping the renderer behind this trait is
/// what lets the whole UI be built and reviewed without the phone.
pub trait Surface {
    fn size(&self) -> (u32, u32);
    /// Hand `canvas` to the display. It returns once the frame before it is
    /// on the panel and this one is on its way, so the next frame is drawn
    /// while this one goes out.
    fn present(&mut self, canvas: &Canvas) -> Result<()>;
    /// Wait until the frame presented last is on the panel.
    fn settle(&mut self) -> Result<()>;
}

/// The device backend: scanout through KMS.
///
/// The scanout itself lives in `skyward-kms`, which knows nothing about the
/// UI, the fonts or anything else that draws into a canvas — so the startup
/// menu can put pixels on the panel without linking any of it. What stays
/// here is only the adapter that lets the UI present a `Canvas`.
#[cfg(target_os = "android")]
pub mod kms {
    use anyhow::Result;

    pub use skyward_kms::{capture, report, Frame, KmsSurface};

    impl crate::surface::Surface for KmsSurface {
        fn size(&self) -> (u32, u32) {
            skyward_kms::KmsSurface::size(self)
        }

        fn present(&mut self, canvas: &crate::canvas::Canvas) -> Result<()> {
            skyward_kms::KmsSurface::present_frame(self, &canvas.data, canvas.w, canvas.h)
        }

        fn settle(&mut self) -> Result<()> {
            skyward_kms::KmsSurface::settle(self)
        }
    }
}
