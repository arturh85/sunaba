//! GIF capture for creature behavior visualization
//!
//! Captures simulation frames and encodes them as animated GIFs.

use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result};
use gif::{Encoder, Frame, Repeat};

use super::PixelRenderer;

/// Captures frames and encodes them as GIF
pub struct GifCapture {
    /// Collected frames (RGB data)
    pub frames: Vec<Vec<u8>>,
    /// Frame dimensions
    width: u16,
    height: u16,
    /// Delay between frames in centiseconds (100ths of a second)
    frame_delay: u16,
}

impl GifCapture {
    /// Create a new GIF capture with specified dimensions
    ///
    /// # Arguments
    /// * `width` - Frame width in pixels
    /// * `height` - Frame height in pixels
    /// * `fps` - Target frames per second (converted to delay)
    pub fn new(width: u16, height: u16, fps: u16) -> Self {
        // Convert FPS to centisecond delay
        let frame_delay = if fps > 0 { 100 / fps } else { 10 };

        Self {
            frames: Vec::new(),
            width,
            height,
            frame_delay,
        }
    }

    /// Capture a frame from a pixel renderer
    pub fn capture_frame(&mut self, renderer: &PixelRenderer) {
        let rgb = renderer.get_rgb_buffer();
        self.frames.push(rgb);
    }

    /// Get the number of captured frames
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Clear all captured frames
    pub fn clear(&mut self) {
        self.frames.clear();
    }

    /// Save captured frames as an animated GIF
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        if self.frames.is_empty() {
            anyhow::bail!("No frames to save");
        }

        let file = File::create(path.as_ref()).context("Failed to create GIF file")?;

        let mut encoder = Encoder::new(file, self.width, self.height, &[])
            .context("Failed to create GIF encoder")?;

        encoder
            .set_repeat(Repeat::Infinite)
            .context("Failed to set GIF repeat")?;

        for frame_data in &self.frames {
            // Create frame from RGB data
            let mut frame = Frame::from_rgb(self.width, self.height, frame_data);
            frame.delay = self.frame_delay;

            encoder
                .write_frame(&frame)
                .context("Failed to write GIF frame")?;
        }

        Ok(())
    }

    /// Save with custom palette for smaller file size
    pub fn save_with_palette<P: AsRef<Path>>(&self, path: P, _max_colors: usize) -> Result<()> {
        if self.frames.is_empty() {
            anyhow::bail!("No frames to save");
        }

        // For simplicity, use standard save
        // A more sophisticated implementation could use color quantization
        self.save(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gif_capture_creation() {
        let capture = GifCapture::new(128, 128, 10);
        assert_eq!(capture.width, 128);
        assert_eq!(capture.height, 128);
        assert_eq!(capture.frame_delay, 10); // 100/10 = 10 centiseconds
        assert_eq!(capture.frame_count(), 0);
    }

    #[test]
    fn test_frame_capture() {
        let mut capture = GifCapture::new(64, 64, 10);
        let renderer = PixelRenderer::new(64, 64);

        capture.capture_frame(&renderer);
        assert_eq!(capture.frame_count(), 1);

        capture.capture_frame(&renderer);
        assert_eq!(capture.frame_count(), 2);
    }

    #[test]
    fn test_clear_frames() {
        let mut capture = GifCapture::new(64, 64, 10);
        let renderer = PixelRenderer::new(64, 64);

        capture.capture_frame(&renderer);
        capture.capture_frame(&renderer);
        assert_eq!(capture.frame_count(), 2);

        capture.clear();
        assert_eq!(capture.frame_count(), 0);
    }
}
