use std::collections::HashMap;

pub struct HoverEngine {
    docs: HashMap<String, String>,
}

impl HoverEngine {
    pub fn new() -> Self {
        let mut docs = HashMap::new();
        
        // Material properties
        docs.insert("name".to_string(), "Material name identifier. Used to reference the material in code.".to_string());
        docs.insert("shadingModel".to_string(), "Shading model defines how the material interacts with light.\n\nValues: lit, unlit, subsurface, cloth, specularGlossiness".to_string());
        docs.insert("requires".to_string(), "Required vertex attributes.\n\nValues: position, normal, uv0, uv1, color, tangents, custom0-4, boneIndices, boneWeights".to_string());
        docs.insert("parameters".to_string(), "Material parameters that can be set at runtime.".to_string());
        docs.insert("constants".to_string(), "Compile-time constants for the material.".to_string());
        docs.insert("culling".to_string(), "Face culling mode.\n\nValues: front, back, none".to_string());
        docs.insert("blending".to_string(), "Blending mode for transparency.\n\nValues: opaque, transparent, fade, masked, add, custom".to_string());
        docs.insert("vertexDomain".to_string(), "Vertex domain for the material.\n\nValues: object, world, view, device".to_string());
        docs.insert("doubleSided".to_string(), "Whether the material renders on both sides of the geometry.".to_string());
        docs.insert("colorWrite".to_string(), "Enable/disable color buffer writing.".to_string());
        docs.insert("depthWrite".to_string(), "Enable/disable depth buffer writing.".to_string());
        
        // Shading models
        docs.insert("lit".to_string(), "Standard PBR shading model with full lighting support.".to_string());
        docs.insert("unlit".to_string(), "No lighting calculations. Useful for UI, debug visuals, or emissive materials.".to_string());
        docs.insert("subsurface".to_string(), "Subsurface scattering for translucent materials like skin, wax, or marble.".to_string());
        docs.insert("cloth".to_string(), "Specialized shading model for fabric and cloth materials.".to_string());
        docs.insert("specularGlossiness".to_string(), "Specular-glossiness workflow (alternative to metallic-roughness).".to_string());
        
        // Blending modes
        docs.insert("opaque".to_string(), "Fully opaque, no transparency.".to_string());
        docs.insert("transparent".to_string(), "Alpha blending for glass-like transparency.".to_string());
        docs.insert("fade".to_string(), "Fade transparency (simplified alpha blending).".to_string());
        docs.insert("masked".to_string(), "Alpha mask with a threshold (binary transparency).".to_string());
        docs.insert("add".to_string(), "Additive blending for glow effects.".to_string());
        docs.insert("custom".to_string(), "Custom blending with user-defined blend functions.".to_string());
        
        Self { docs }
    }
    
    pub fn get_hover(&self, word: &str) -> Option<&String> {
        self.docs.get(word)
    }
}

impl Default for HoverEngine {
    fn default() -> Self {
        Self::new()
    }
}
