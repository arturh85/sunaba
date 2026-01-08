//! MP4 video capture for documentation and visualization
//!
//! Captures simulation frames as PNG sequence and encodes to MP4 using FFmpeg.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use image::{ImageBuffer, RgbImage};
use tempfile::TempDir;

use crate::headless::PixelRenderer;

/// Captures frames and encodes them as MP4 video using FFmpeg
pub struct VideoCapture {
    /// Temporary directory for PNG frames
    frames_dir: TempDir,
    /// Number of frames captured
    frame_count: usize,
    /// Frame dimensions
    width: u32,
    height: u32,
    /// Target frames per second
    fps: u32,
}

impl VideoCapture {
    /// Create a new video capture with specified dimensions
    ///
    /// # Arguments
    /// * `width` - Frame width in pixels
    /// * `height` - Frame height in pixels
    /// * `fps` - Target frames per second for output video
    pub fn new(width: u32, height: u32, fps: u32) -> Result<Self> {
        let frames_dir = TempDir::new().context("Failed to create temp directory for frames")?;

        Ok(Self {
            frames_dir,
            frame_count: 0,
            width,
            height,
            fps,
        })
    }

    /// Capture a frame from a pixel renderer
    ///
    /// Saves the frame as PNG to the temporary directory.
    pub fn capture_frame(&mut self, renderer: &PixelRenderer) -> Result<()> {
        let rgb = renderer.get_rgb_buffer();

        // Create ImageBuffer from RGB data
        let img: RgbImage = ImageBuffer::from_raw(self.width, self.height, rgb)
            .context("Failed to create image buffer from renderer data")?;

        // Save as PNG with frame number
        let frame_path = self
            .frames_dir
            .path()
            .join(format!("frame_{:05}.png", self.frame_count));
        img.save(&frame_path).with_context(|| {
            format!(
                "Failed to save frame {} to {:?}",
                self.frame_count, frame_path
            )
        })?;

        self.frame_count += 1;
        Ok(())
    }

    /// Get the number of captured frames
    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    /// Encode captured frames to MP4 using FFmpeg
    ///
    /// # Arguments
    /// * `output` - Output MP4 file path
    ///
    /// # FFmpeg Command
    /// ```bash
    /// ffmpeg -framerate {fps} -i frame_%05d.png -c:v libx264 -preset medium -crf 23 -pix_fmt yuv420p -y output.mp4
    /// ```
    ///
    /// # Quality Settings
    /// - `-crf 23`: Constant Rate Factor (lower = better quality, 18-28 range, 23 is default)
    /// - `-preset medium`: Encoding speed/compression tradeoff
    /// - `-pix_fmt yuv420p`: Compatibility with most players
    pub fn encode_to_mp4<P: AsRef<Path>>(&self, output: P) -> Result<()> {
        if self.frame_count == 0 {
            anyhow::bail!("No frames to encode");
        }

        let output_path = output.as_ref();

        // Ensure output directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create output directory: {:?}", parent))?;
        }

        log::info!(
            "Encoding {} frames to MP4: {:?} ({}fps, {}x{})",
            self.frame_count,
            output_path,
            self.fps,
            self.width,
            self.height
        );

        // Build FFmpeg command
        let status = Command::new("ffmpeg")
            .current_dir(self.frames_dir.path())
            .args(&[
                "-framerate",
                &self.fps.to_string(),
                "-i",
                "frame_%05d.png",
                "-c:v",
                "libx264",
                "-preset",
                "medium",
                "-crf",
                "23",
                "-pix_fmt",
                "yuv420p",
                "-y", // Overwrite output file
                output_path.to_str().context("Invalid output path")?,
            ])
            .status()
            .context("Failed to execute ffmpeg (is it installed?)")?;

        if !status.success() {
            anyhow::bail!(
                "FFmpeg encoding failed with exit code: {}",
                status.code().unwrap_or(-1)
            );
        }

        log::info!("Successfully encoded MP4: {:?}", output_path);
        Ok(())
    }
}

impl Drop for VideoCapture {
    /// Temporary directory is automatically cleaned up when VideoCapture is dropped
    fn drop(&mut self) {
        // TempDir automatically cleans up on drop
        log::debug!("Cleaning up {} frame PNGs", self.frame_count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_capture_creation() {
        let capture = VideoCapture::new(1280, 720, 20).expect("Failed to create VideoCapture");
        assert_eq!(capture.width, 1280);
        assert_eq!(capture.height, 720);
        assert_eq!(capture.fps, 20);
        assert_eq!(capture.frame_count(), 0);
    }

    #[test]
    fn test_frame_capture() {
        let mut capture = VideoCapture::new(128, 128, 10).expect("Failed to create VideoCapture");
        let renderer = PixelRenderer::new(128, 128);

        capture
            .capture_frame(&renderer)
            .expect("Failed to capture frame");
        assert_eq!(capture.frame_count(), 1);

        capture
            .capture_frame(&renderer)
            .expect("Failed to capture frame");
        assert_eq!(capture.frame_count(), 2);

        // Verify frame files exist
        let frame0_path = capture.frames_dir.path().join("frame_00000.png");
        let frame1_path = capture.frames_dir.path().join("frame_00001.png");
        assert!(frame0_path.exists());
        assert!(frame1_path.exists());
    }

    #[test]
    #[ignore] // Requires FFmpeg installation
    fn test_encode_to_mp4() {
        let mut capture = VideoCapture::new(128, 128, 10).expect("Failed to create VideoCapture");
        let renderer = PixelRenderer::new(128, 128);

        // Capture a few frames
        for _ in 0..10 {
            capture
                .capture_frame(&renderer)
                .expect("Failed to capture frame");
        }

        // Encode to MP4
        let output_path = std::env::temp_dir().join("test_video.mp4");
        capture
            .encode_to_mp4(&output_path)
            .expect("Failed to encode MP4");

        // Verify file exists
        assert!(output_path.exists());

        // Clean up
        std::fs::remove_file(&output_path).ok();
    }
}
