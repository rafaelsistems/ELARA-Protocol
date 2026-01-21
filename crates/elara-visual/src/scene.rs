//! Scene State - Environment and background as state
//!
//! This is NOT a video background or image.
//! This is the STATE of the visual environment for reality synchronization.

use elara_core::StateTime;

/// Color in RGB (0.0 - 1.0 range)
#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }
    
    pub fn black() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
    
    pub fn white() -> Self {
        Self::new(1.0, 1.0, 1.0)
    }
    
    pub fn lerp(&self, other: &Color, t: f32) -> Color {
        Color {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
        }
    }
}

/// Lighting condition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightingCondition {
    Unknown,
    Bright,
    Normal,
    Dim,
    Dark,
    Backlit,
    Spotlight,
}

/// Environment type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentType {
    Unknown,
    Indoor,
    Outdoor,
    Office,
    Home,
    Vehicle,
    Nature,
    Urban,
}

/// Background complexity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundComplexity {
    Simple,     // Solid color or minimal
    Moderate,   // Some objects/texture
    Complex,    // Busy background
    Dynamic,    // Moving background
}

/// Scene object (simplified representation)
#[derive(Debug, Clone)]
pub struct SceneObject {
    /// Object type/category
    pub category: String,
    /// Bounding box (normalized 0-1): x, y, width, height
    pub bounds: (f32, f32, f32, f32),
    /// Confidence
    pub confidence: f32,
    /// Is this object moving?
    pub moving: bool,
}

impl SceneObject {
    pub fn new(category: &str, x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            category: category.to_string(),
            bounds: (x, y, w, h),
            confidence: 1.0,
            moving: false,
        }
    }
}

/// Complete scene state
#[derive(Debug, Clone)]
pub struct SceneState {
    /// Timestamp
    pub timestamp: StateTime,
    
    /// Dominant background color
    pub background_color: Color,
    
    /// Lighting condition
    pub lighting: LightingCondition,
    
    /// Environment type
    pub environment: EnvironmentType,
    
    /// Background complexity
    pub complexity: BackgroundComplexity,
    
    /// Detected objects in scene
    pub objects: Vec<SceneObject>,
    
    /// Is there significant motion in the background?
    pub background_motion: bool,
    
    /// Blur level (0.0 = sharp, 1.0 = very blurred)
    pub blur: f32,
    
    /// Noise level (0.0 = clean, 1.0 = very noisy)
    pub noise: f32,
    
    /// Detail level (0.0 - 1.0, for degradation)
    pub detail_level: f32,
}

impl SceneState {
    /// Create a new scene state
    pub fn new(timestamp: StateTime) -> Self {
        Self {
            timestamp,
            background_color: Color::white(),
            lighting: LightingCondition::Normal,
            environment: EnvironmentType::Unknown,
            complexity: BackgroundComplexity::Simple,
            objects: Vec::new(),
            background_motion: false,
            blur: 0.0,
            noise: 0.0,
            detail_level: 1.0,
        }
    }
    
    /// Simple scene (solid color background)
    pub fn simple(timestamp: StateTime, color: Color) -> Self {
        Self {
            timestamp,
            background_color: color,
            lighting: LightingCondition::Normal,
            environment: EnvironmentType::Unknown,
            complexity: BackgroundComplexity::Simple,
            objects: Vec::new(),
            background_motion: false,
            blur: 0.0,
            noise: 0.0,
            detail_level: 1.0,
        }
    }
    
    /// Add an object to the scene
    pub fn add_object(&mut self, object: SceneObject) {
        self.objects.push(object);
        self.update_complexity();
    }
    
    /// Update complexity based on objects
    fn update_complexity(&mut self) {
        self.complexity = match self.objects.len() {
            0 => BackgroundComplexity::Simple,
            1..=3 => BackgroundComplexity::Moderate,
            _ => BackgroundComplexity::Complex,
        };
        
        if self.objects.iter().any(|o| o.moving) {
            self.complexity = BackgroundComplexity::Dynamic;
        }
    }
    
    /// Reduce detail level (for degradation)
    pub fn reduce_detail(&mut self, factor: f32) {
        self.detail_level *= factor.clamp(0.0, 1.0);
        
        // Remove low-confidence objects
        self.objects.retain(|o| o.confidence > (1.0 - self.detail_level));
        
        // Increase blur as detail decreases
        self.blur = (1.0 - self.detail_level) * 0.5;
    }
    
    /// Interpolate between two scene states
    pub fn lerp(&self, other: &SceneState, t: f32) -> SceneState {
        SceneState {
            timestamp: other.timestamp,
            background_color: self.background_color.lerp(&other.background_color, t),
            lighting: if t < 0.5 { self.lighting } else { other.lighting },
            environment: if t < 0.5 { self.environment } else { other.environment },
            complexity: if t < 0.5 { self.complexity } else { other.complexity },
            objects: if t < 0.5 { self.objects.clone() } else { other.objects.clone() },
            background_motion: if t < 0.5 { self.background_motion } else { other.background_motion },
            blur: self.blur + (other.blur - self.blur) * t,
            noise: self.noise + (other.noise - self.noise) * t,
            detail_level: self.detail_level + (other.detail_level - self.detail_level) * t,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_color_lerp() {
        let black = Color::black();
        let white = Color::white();
        
        let gray = black.lerp(&white, 0.5);
        assert!((gray.r - 0.5).abs() < 0.01);
        assert!((gray.g - 0.5).abs() < 0.01);
        assert!((gray.b - 0.5).abs() < 0.01);
    }
    
    #[test]
    fn test_scene_state() {
        let time = StateTime::from_millis(0);
        let mut scene = SceneState::new(time);
        
        assert_eq!(scene.complexity, BackgroundComplexity::Simple);
        
        scene.add_object(SceneObject::new("chair", 0.1, 0.5, 0.2, 0.3));
        scene.add_object(SceneObject::new("table", 0.5, 0.5, 0.3, 0.2));
        
        assert_eq!(scene.complexity, BackgroundComplexity::Moderate);
    }
    
    #[test]
    fn test_scene_reduce_detail() {
        let time = StateTime::from_millis(0);
        let mut scene = SceneState::new(time);
        scene.add_object(SceneObject::new("object", 0.5, 0.5, 0.1, 0.1));
        
        scene.reduce_detail(0.5);
        assert!((scene.detail_level - 0.5).abs() < 0.01);
        assert!(scene.blur > 0.0);
    }
}
