use crate::schema::{KeywordType, get_keywords_by_type, get_properties};
use std::collections::HashMap;
use std::sync::OnceLock;

static DOCS: OnceLock<HashMap<String, String>> = OnceLock::new();

pub struct HoverEngine;

impl HoverEngine {
  pub fn new() -> Self {
    Self
  }

  fn docs() -> &'static HashMap<String, String> {
    DOCS.get_or_init(|| {
      let mut docs = HashMap::new();

      // Material properties
      for prop in get_properties() {
        let mut doc = prop.docs.to_string();
        if let Some(values) = prop.valid_values {
          doc.push_str("\n\n**Values:** ");
          doc.push_str(&values.join(", "));
        }
        docs.insert(prop.name.to_string(), doc);
      }

      // Enum values from keyword map
      Self::add_enum_docs(&mut docs, KeywordType::ShadingModel, "Shading model");
      Self::add_enum_docs(&mut docs, KeywordType::BlendingMode, "Blending mode");
      Self::add_enum_docs(&mut docs, KeywordType::CullingMode, "Culling mode");
      Self::add_enum_docs(&mut docs, KeywordType::VertexDomain, "Vertex domain");
      Self::add_enum_docs(&mut docs, KeywordType::MaterialDomain, "Material domain");
      Self::add_enum_docs(
        &mut docs,
        KeywordType::InterpolationMode,
        "Interpolation mode",
      );
      Self::add_enum_docs(&mut docs, KeywordType::RefractionMode, "Refraction mode");
      Self::add_enum_docs(&mut docs, KeywordType::RefractionType, "Refraction type");
      Self::add_enum_docs(&mut docs, KeywordType::ReflectionMode, "Reflection mode");
      Self::add_enum_docs(
        &mut docs,
        KeywordType::TransparencyMode,
        "Transparency mode",
      );
      Self::add_enum_docs(
        &mut docs,
        KeywordType::StereoscopicType,
        "Stereoscopic type",
      );
      Self::add_enum_docs(&mut docs, KeywordType::QualityLevel, "Quality level");
      Self::add_enum_docs(
        &mut docs,
        KeywordType::SpecularAmbientOcclusionMode,
        "Specular ambient occlusion mode",
      );
      Self::add_enum_docs(&mut docs, KeywordType::PrecisionValue, "Precision value");
      Self::add_enum_docs(&mut docs, KeywordType::SamplerFormat, "Sampler format");
      Self::add_enum_docs(&mut docs, KeywordType::BlendFunction, "Blend function");
      Self::add_enum_docs(&mut docs, KeywordType::VertexAttribute, "Vertex attribute");
      Self::add_enum_docs(
        &mut docs,
        KeywordType::VariantFilterValue,
        "Variant filter value",
      );
      Self::add_enum_docs(&mut docs, KeywordType::ParameterType, "Parameter type");

      // Filament Shader API
      docs.insert(
        "MaterialInputs".to_string(),
        "Filament shader input struct. Contains fields like baseColor, roughness, metallic, normal, etc.".to_string(),
      );

      // MaterialInputs fields (set on `material.<field>` inside the fragment shader).
      let material_input_fields: &[(&str, &str)] = &[
        ("baseColor", "Linear base color (RGBA). Typical range [0..1] for LDR. Premultiplied alpha is handled by Filament."),
        ("roughness", "Surface roughness in the range [0..1]. Roughness 0 is mirror-like, 1 is fully rough."),
        ("metallic", "Metallic factor in the range [0..1]. 0 = dielectric, 1 = metal."),
        ("normal", "Normal in the shading tangent frame. Usually derived from a normal map."),
        ("emissive", "Emissive color (RGB) added to the final color regardless of lighting."),
        ("ambientOcclusion", "Ambient occlusion factor in the range [0..1]. Multiplied into indirect lighting."),
        ("sheenColor", "Sheen color for cloth-like materials (multi-scatter specular)."),
        ("sheenRoughness", "Sheen roughness for cloth-like materials."),
        ("clearcoat", "Clearcoat layer strength in the range [0..1]."),
        ("clearcoatRoughness", "Roughness of the clearcoat layer in the range [0..1]."),
        ("clearcoatNormal", "Normal of the clearcoat layer in the shading tangent frame."),
        ("thickness", "Thickness of the material, used by refraction and transmission."),
        ("subsurfaceColor", "Subsurface scattering color."),
        ("subsurfacePower", "Subsurface scattering power (controls the falloff)."),
        ("specularColor", "Specular color for the F0 term (dielectric-conductor blend)."),
        ("microfacetAlpha", "Legacy alias for the microfacet distribution width (roughness)."),
        ("dispersion", "Dispersion factor for refractive materials."),
        ("anisotropy", "Anisotropy of the specular highlight, in the range [-1..1]."),
        ("anisotropyDirection", "Direction of anisotropy in the shading tangent frame."),
        ("perceptualRoughness", "Perceptual (Gamma-corrected) roughness, squared to obtain the microfacet roughness."),
        ("ior", "Index of refraction of the material, used for dielectric specular and refraction."),
        ("absorption", "Absorption coefficient used by refraction and transmission."),
        ("transmission", "Transmission factor in the range [0..1] (transparent/refractive materials)."),
        ("reflectance", "Reflectance at normal incidence, in the range [0..1] (default 0.5)."),
        ("userData", "Unused user-defined data slot that can be written from the shader."),
      ];
      for (name, field_doc) in material_input_fields {
        docs.insert(
          (*name).to_string(),
          format!("MaterialInputs field. {}", field_doc),
        );
      }
      docs.insert(
        "MaterialVertexInputs".to_string(),
        "Filament vertex shader input struct. Contains vertex attributes passed to the fragment shader.".to_string(),
      );
      docs.insert(
        "materialParams".to_string(),
        "Runtime material parameter block. Access parameters defined in the material block using materialParams.name.".to_string(),
      );
      docs.insert(
        "prepareMaterial".to_string(),
        "Filament built-in function. Must be called before modifying MaterialInputs in the fragment shader.".to_string(),
      );
      docs.insert(
        "getUV0".to_string(),
        "Returns the first UV coordinate set (uv0) for the current fragment.".to_string(),
      );
      docs.insert(
        "getUV1".to_string(),
        "Returns the second UV coordinate set (uv1) for the current fragment.".to_string(),
      );
      docs.insert(
        "getWorldPosition".to_string(),
        "Returns the world-space position of the current fragment.".to_string(),
      );
      docs.insert(
        "getWorldNormal".to_string(),
        "Returns the world-space normal of the current fragment.".to_string(),
      );
      docs.insert(
        "texture".to_string(),
        "GLSL built-in function for sampling textures.".to_string(),
      );

      // Block-level documentation for new material blocks
      docs.insert(
        "constants".to_string(),
        "Compile-time constants array. Each element is an object with `name`, `type` (int/float/bool), and optional `default` value.".to_string(),
      );
      docs.insert(
        "variables".to_string(),
        "Material variables array. List of identifiers that can be used as shader inputs.".to_string(),
      );
      docs.insert(
        "buffers".to_string(),
        "SSBO (Shader Storage Buffer Object) bindings. Each element is an object with `name`.".to_string(),
      );
      docs.insert(
        "subpasses".to_string(),
        "Subpass inputs for multi-pass rendering. Each element is an object with `name` and `type`.".to_string(),
      );
      docs.insert(
        "outputs".to_string(),
        "Fragment shader outputs. Each element is an object with `name` and `type` (e.g., float4).".to_string(),
      );

      // Field-level documentation for array object fields
      docs.insert(
        "name".to_string(),
        "Name of the parameter, constant, buffer, subpass, or output. Must be unique within its block.".to_string(),
      );
      docs.insert(
        "type".to_string(),
        "Data type. For parameters: float4, sampler2d, etc. For constants: int, float, bool. For subpasses/outputs: the data type.".to_string(),
      );
      docs.insert(
        "default".to_string(),
        "Default value for constants or parameters. Used when no runtime value is provided.".to_string(),
      );
      docs.insert(
        "precision".to_string(),
        "Numeric precision: low, medium, high. Default is medium.".to_string(),
      );
      docs.insert(
        "format".to_string(),
        "Texture format for samplers: linear, nearest, etc.".to_string(),
      );
      docs.insert(
        "filterable".to_string(),
        "Whether the sampler can be filtered. Default is true.".to_string(),
      );
      docs.insert(
        "multisample".to_string(),
        "Whether the sampler supports multisampling.".to_string(),
      );
      docs.insert(
        "stages".to_string(),
        "Shader stages where this parameter is active: vertex, fragment, or both.".to_string(),
      );
      docs.insert(
        "transformName".to_string(),
        "Name of the transform matrix for this parameter.".to_string(),
      );

      docs
    })
  }

  fn add_enum_docs(docs: &mut HashMap<String, String>, keyword_type: KeywordType, category: &str) {
    for kw in get_keywords_by_type(keyword_type) {
      if !docs.contains_key(kw) {
        docs.insert(kw.to_string(), format!("{}: {}", category, kw));
      }
    }
  }

  pub fn get_hover(&self, word: &str) -> Option<&'static String> {
    Self::docs().get(word)
  }
}

impl Default for HoverEngine {
  fn default() -> Self {
    Self::new()
  }
}
