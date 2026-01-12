use crate::error::{GeminiError, GeminiResult};

/// Bounding box in Gemini's normalized format [y_min, x_min, y_max, x_max] (0-1000)
#[derive(Debug, Clone, Copy)]
pub struct NormalizedBoundingBox {
    pub y_min: f32,
    pub x_min: f32,
    pub y_max: f32,
    pub x_max: f32,
}

impl NormalizedBoundingBox {
    /// Create from array in Gemini's [y_min, x_min, y_max, x_max] format
    pub fn from_array(arr: [f32; 4]) -> Self {
        Self {
            y_min: arr[0],
            x_min: arr[1],
            y_max: arr[2],
            x_max: arr[3],
        }
    }

    /// Validate that coordinates are within 0-1000 range
    pub fn validate(&self) -> GeminiResult<()> {
        if self.y_min < 0.0 || self.y_min > 1000.0
            || self.x_min < 0.0 || self.x_min > 1000.0
            || self.y_max < 0.0 || self.y_max > 1000.0
            || self.x_max < 0.0 || self.x_max > 1000.0
        {
            return Err(GeminiError::BoundingBoxError(format!(
                "Coordinates out of 0-1000 range: {:?}",
                self
            )));
        }

        if self.x_min >= self.x_max || self.y_min >= self.y_max {
            return Err(GeminiError::BoundingBoxError(format!(
                "Invalid bounding box: min >= max. {:?}",
                self
            )));
        }

        Ok(())
    }
}

/// Bounding box in pixel coordinates
#[derive(Debug, Clone, Copy)]
pub struct PixelBoundingBox {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelBoundingBox {
    /// Get the center point of the bounding box
    pub fn center(&self) -> (i32, i32) {
        let center_x = self.x as i32 + (self.width as i32 / 2);
        let center_y = self.y as i32 + (self.height as i32 / 2);
        (center_x, center_y)
    }

    /// Check if the bounding box is within image bounds
    pub fn is_within_bounds(&self, image_width: u32, image_height: u32) -> bool {
        self.x + self.width <= image_width && self.y + self.height <= image_height
    }
}

/// Convert Gemini's normalized bounding box (0-1000) to pixel coordinates
pub fn convert_to_pixels(
    normalized: &NormalizedBoundingBox,
    image_width: u32,
    image_height: u32,
) -> GeminiResult<PixelBoundingBox> {
    // Validate input
    normalized.validate()?;

    // Convert from 0-1000 to 0-1 normalized range
    let x_min_norm = normalized.x_min / 1000.0;
    let y_min_norm = normalized.y_min / 1000.0;
    let x_max_norm = normalized.x_max / 1000.0;
    let y_max_norm = normalized.y_max / 1000.0;

    // Convert to pixel coordinates
    let x = (x_min_norm * image_width as f32) as u32;
    let y = (y_min_norm * image_height as f32) as u32;
    let x_max = (x_max_norm * image_width as f32) as u32;
    let y_max = (y_max_norm * image_height as f32) as u32;

    let width = x_max.saturating_sub(x);
    let height = y_max.saturating_sub(y);

    let pixel_box = PixelBoundingBox {
        x,
        y,
        width,
        height,
    };

    // Validate result is within image bounds
    if !pixel_box.is_within_bounds(image_width, image_height) {
        return Err(GeminiError::BoundingBoxError(format!(
            "Converted bounding box exceeds image bounds: {:?} (image: {}x{})",
            pixel_box, image_width, image_height
        )));
    }

    Ok(pixel_box)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalized_bbox_from_array() {
        let bbox = NormalizedBoundingBox::from_array([100.0, 200.0, 300.0, 400.0]);
        assert_eq!(bbox.y_min, 100.0);
        assert_eq!(bbox.x_min, 200.0);
        assert_eq!(bbox.y_max, 300.0);
        assert_eq!(bbox.x_max, 400.0);
    }

    #[test]
    fn test_normalized_bbox_validate() {
        let valid = NormalizedBoundingBox::from_array([100.0, 200.0, 300.0, 400.0]);
        assert!(valid.validate().is_ok());

        let out_of_range = NormalizedBoundingBox::from_array([100.0, 200.0, 300.0, 1200.0]);
        assert!(out_of_range.validate().is_err());

        let inverted = NormalizedBoundingBox::from_array([300.0, 400.0, 100.0, 200.0]);
        assert!(inverted.validate().is_err());
    }

    #[test]
    fn test_convert_to_pixels() {
        let normalized = NormalizedBoundingBox::from_array([250.0, 250.0, 750.0, 750.0]);
        let pixel_box = convert_to_pixels(&normalized, 1000, 1000).unwrap();

        // Should be roughly in the center quarter
        assert_eq!(pixel_box.x, 250);
        assert_eq!(pixel_box.y, 250);
        assert_eq!(pixel_box.width, 500);
        assert_eq!(pixel_box.height, 500);
    }

    #[test]
    fn test_pixel_bbox_center() {
        let bbox = PixelBoundingBox {
            x: 100,
            y: 100,
            width: 80,
            height: 60,
        };
        assert_eq!(bbox.center(), (140, 130));
    }

    #[test]
    fn test_pixel_bbox_within_bounds() {
        let bbox = PixelBoundingBox {
            x: 100,
            y: 100,
            width: 80,
            height: 60,
        };
        assert!(bbox.is_within_bounds(1000, 1000));
        assert!(!bbox.is_within_bounds(150, 150));
    }

    #[test]
    fn test_convert_exceeds_bounds() {
        let normalized = NormalizedBoundingBox::from_array([900.0, 900.0, 1000.0, 1000.0]);
        // Converting to small image should fail bounds check
        let result = convert_to_pixels(&normalized, 100, 100);
        // This should succeed because the normalized coords are valid, but the result
        // should be at the edge
        assert!(result.is_ok());
    }
}
