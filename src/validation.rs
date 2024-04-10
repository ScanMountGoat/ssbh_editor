use crate::{FileResult, TextureDimension};
use approx::relative_eq;
use nutexb::{NutexbFile, NutexbFormat};
use ssbh_data::{
    matl_data::{BlendFactor, ParamId, WrapMode},
    mesh_data::VectorData,
    prelude::*,
};
use ssbh_wgpu::{ModelFolder, ShaderDatabase};
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    path::Path,
};
use thiserror::Error;

// TODO: Add a severity level to differentiate warnings vs errors.
#[derive(Default)]
pub struct ModelValidationErrors {
    pub mesh_errors: Vec<MeshValidationError>,
    pub meshex_errors: Vec<MeshExValidationError>,
    pub skel_errors: Vec<SkelValidationError>,
    pub matl_errors: Vec<MatlValidationError>,
    pub modl_errors: Vec<ModlValidationError>,
    pub adj_errors: Vec<AdjValidationError>,
    pub anim_errors: Vec<AnimValidationError>,
    pub hlpb_errors: Vec<HlpbValidationError>,
    pub nutexb_errors: Vec<NutexbValidationError>,
}

impl ModelValidationErrors {
    pub fn from_model<'a, 'b>(
        model: &'b ModelFolder,
        shader_database: &ShaderDatabase,
        default_texture_names: impl Iterator<Item = (&'a String, TextureDimension)> + Clone,
    ) -> Self
    where
        'b: 'a,
    {
        // Each validation check may add errors to multiple related files.
        let mut validation = Self::default();

        let mesh = model.find_mesh();
        if let Some(mesh) = mesh {
            validate_mesh_subindices(&mut validation, mesh);
            validate_mesh_vertex_weights(&mut validation, mesh);
        }

        let modl = model.find_modl();
        let matl = model.find_matl();

        if let Some(modl) = modl {
            validate_modl_entries(&mut validation, modl, matl, mesh);
        }

        if let Some(matl) = matl {
            validate_required_attributes(&mut validation, matl, modl, mesh, shader_database);
            validate_shader_labels(&mut validation, matl, shader_database);
            validate_wrap_mode_tiling(&mut validation, matl, modl, mesh);
            validate_texture_format_usage(&mut validation, matl, &model.nutexbs);
            validate_premultiplied_blend(&mut validation, matl, shader_database);
            validate_texture_dimensions(
                &mut validation,
                matl,
                &model.nutexbs,
                default_texture_names.clone(),
            );
            validate_texture_assignments(
                &mut validation,
                matl,
                &model.nutexbs,
                default_texture_names,
            );
            validate_renormal_material_entries(
                &mut validation,
                matl,
                model.find_adj(),
                model.find_modl(),
                mesh,
            );
        }

        validation
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct MeshValidationError {
    pub mesh_object_index: usize,
    pub kind: MeshValidationErrorKind,
}

impl std::fmt::Display for MeshValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}

// TODO: Check for unsupported vertex attribute names?
#[derive(Debug, PartialEq, Eq, Error)]
pub enum MeshValidationErrorKind {
    #[error("Mesh {mesh_name:?} is missing attributes {missing_attributes:?} required by assigned material {material_label:?}.")]
    MissingRequiredVertexAttributes {
        mesh_name: String,
        material_label: String,
        missing_attributes: Vec<String>,
    },

    #[error("Mesh {mesh_name:?} repeats subindex {subindex}. Meshes with the same name must have unique subindices.")]
    DuplicateSubindex { mesh_name: String, subindex: u64 },

    #[error(
        "Vertex weights for mesh {mesh_name:?} are not normalized. Vertex weights should sum to 1.0."
    )]
    VertexWeightsNotNormalized { mesh_name: String },

    #[error("Mesh {mesh_name:?} has vertex weights with a weight of 0.0 that can be removed.")]
    VertexWeightsZero { mesh_name: String },
}

pub struct SkelValidationError;
impl Display for SkelValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct MatlValidationError {
    pub entry_index: usize,
    pub kind: MatlValidationErrorKind,
}

impl std::fmt::Display for MatlValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum MatlValidationErrorKind {
    #[error("Mesh {mesh_name:?} is missing attributes {missing_attributes:?} required by assigned material {material_label:?}.")]
    MissingRequiredVertexAttributes {
        material_label: String,
        mesh_name: String,
        missing_attributes: Vec<String>,
    },

    #[error("Texture {nutexb:?} for material {material_label:?} has format {format:?}, but {param_id} {} an sRGB format.",
        if expects_srgb(*param_id) {
            "expects"
        } else {
            "does not expect"
        }
    )]
    UnexpectedTextureFormat {
        material_label: String,
        param_id: ParamId,
        nutexb: String,
        format: NutexbFormat,
    },

    // TODO: Add severity levels and make this the highest severity.
    #[error("Texture {texture:?} for material {material_label:?} has dimensions {actual:?}, but {param_id} requires {expected:?}.")]
    UnexpectedTextureDimension {
        material_label: String,
        param_id: ParamId,
        texture: String,
        expected: TextureDimension,
        actual: TextureDimension,
    },

    #[error("Textures {textures:?} for material {material_label:?} are missing.")]
    MissingTextures {
        material_label: String,
        textures: Vec<String>,
    },

    #[error(
        "Mesh {mesh_name:?} has the RENORMAL material {material_label:?} but no corresponding entry in the model.adjb."
    )]
    RenormalMaterialMissingMeshAdjEntry {
        material_label: String,
        mesh_name: String,
    },

    #[error(
        "Material {material_label:?} is a RENORMAL material, but the model.adjb file is missing."
    )]
    RenormalMaterialMissingAdj { material_label: String },

    #[error(
        "Samplers {samplers:?} for material {material_label:?} will clamp UV coordinates for mesh {mesh_name:?}.
Use wrap mode Repeat if the texture should tile.",
    )]
    WrapModeClampsUvs {
        material_label: String,
        mesh_name: String,
        samplers: Vec<ParamId>,
    },

    #[error("Shader label {shader_label:?} for material {material_label:?} is not a valid shader label.")]
    InvalidShaderLabel {
        material_label: String,
        shader_label: String,
    },

    #[error("Material {material_label:?} uses Source Color \"SourceAlpha\", but shader {shader_label:?} already premultiplies alpha.
Use a Source Color of \"One\" or use a shader that does not premultiply alpha.")]
    PremultipliedShaderSrcAlpha {
        material_label: String,
        shader_label: String,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct ModlValidationError {
    pub entry_index: usize,
    pub kind: ModlValidationErrorKind,
}

impl std::fmt::Display for ModlValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum ModlValidationErrorKind {
    #[error("Modl entry assigns to mesh {mesh_object_name:?} not found in the model.numshb. Ensure the name and subindex are correct.")]
    InvalidMeshObject {
        mesh_object_name: String,
        mesh_object_subindex: usize,
    },

    #[error("Modl entry assigns a material {material_label:?} not found in the model.numatb.")]
    InvalidMaterial { material_label: String },
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum AdjValidationError {
    #[error("Missing entry for mesh {mesh_name:?} with the RENORMAL material {material_label:?}.")]
    MissingRenormalEntry {
        mesh_object_index: usize,
        mesh_name: String,
        material_label: String,
    },
}

pub struct AnimValidationError;
impl Display for AnimValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

pub struct HlpbValidationError;
impl Display for HlpbValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

// TODO: Check size of surface for unneeded padding.
// TODO: Check if footer data size matches actual data.
#[derive(Debug, PartialEq, Eq, Error)]
pub enum NutexbValidationError {
    #[error("Texture {nutexb:?} has format {format:?}, but {param} {} an sRGB format.",
        if expects_srgb(*param) {
            "expects"
        } else {
            "does not expect"
        }
    )]
    FormatInvalidForUsage {
        nutexb: String,
        format: NutexbFormat,
        param: ParamId,
    },
}

impl NutexbValidationError {
    pub fn name(&self) -> &str {
        match self {
            NutexbValidationError::FormatInvalidForUsage { nutexb, .. } => nutexb,
        }
    }
}

pub struct MeshExValidationError;
impl Display for MeshExValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

fn validate_required_attributes(
    validation: &mut ModelValidationErrors,
    matl: &MatlData,
    modl: Option<&ModlData>,
    mesh: Option<&MeshData>,
    shader_database: &ShaderDatabase,
) {
    // Both the modl and mesh should be present to determine material assignments.
    if let (Some(modl), Some(mesh)) = (modl, mesh) {
        for (entry_index, entry) in matl.entries.iter().enumerate() {
            if let Some(program) = shader_database.get(&entry.shader_label) {
                for (i, o) in mesh.objects.iter().enumerate().filter(|(_, o)| {
                    modl.entries
                        .iter()
                        .filter(|e| e.material_label == entry.material_label)
                        .any(|e| {
                            e.mesh_object_name == o.name && e.mesh_object_subindex == o.subindex
                        })
                }) {
                    // Find attributes required by the shader not present in the mesh.
                    // TODO: Avoid clone here?
                    let attribute_names: Vec<_> = o
                        .texture_coordinates
                        .iter()
                        .map(|a| a.name.clone())
                        .chain(o.color_sets.iter().map(|a| a.name.clone()))
                        .collect();

                    // This error can be fixed by modifying the material's shader or mesh's attributes.
                    // Add errors to the matl and mesh for clarity.
                    let missing_attributes = program.missing_required_attributes(&attribute_names);
                    if !missing_attributes.is_empty() {
                        let matl_error = MatlValidationError {
                            entry_index,
                            kind: MatlValidationErrorKind::MissingRequiredVertexAttributes {
                                material_label: entry.material_label.clone(),
                                mesh_name: o.name.clone(),
                                missing_attributes: missing_attributes.clone(),
                            },
                        };
                        validation.matl_errors.push(matl_error);

                        let mesh_error = MeshValidationError {
                            mesh_object_index: i,
                            kind: MeshValidationErrorKind::MissingRequiredVertexAttributes {
                                mesh_name: o.name.clone(),
                                material_label: entry.material_label.clone(),
                                missing_attributes,
                            },
                        };
                        validation.mesh_errors.push(mesh_error);
                    }
                }
            }
        }
    }
}

fn validate_shader_labels(
    validation: &mut ModelValidationErrors,
    matl: &MatlData,
    shader_database: &ShaderDatabase,
) {
    for (entry_index, entry) in matl.entries.iter().enumerate() {
        if shader_database.get(&entry.shader_label).is_none() {
            let error = MatlValidationError {
                entry_index,
                kind: MatlValidationErrorKind::InvalidShaderLabel {
                    material_label: entry.material_label.clone(),
                    shader_label: entry.shader_label.clone(),
                },
            };
            validation.matl_errors.push(error);
        }
    }
}

fn validate_premultiplied_blend(
    validation: &mut ModelValidationErrors,
    matl: &MatlData,
    shader_database: &ShaderDatabase,
) {
    for (entry_index, entry) in matl.entries.iter().enumerate() {
        if let Some(program) = shader_database.get(&entry.shader_label) {
            if let Some(blend_state) = entry.blend_states.first() {
                if program.premultiplied
                    && blend_state.data.source_color == BlendFactor::SourceAlpha
                {
                    // This will square the src alpha and probably isn't intentional.
                    let error = MatlValidationError {
                        entry_index,
                        kind: MatlValidationErrorKind::PremultipliedShaderSrcAlpha {
                            material_label: entry.material_label.clone(),
                            shader_label: entry.shader_label.clone(),
                        },
                    };
                    validation.matl_errors.push(error);
                }
            }
        }
    }
}

fn is_repeat(wrap: WrapMode) -> bool {
    wrap == WrapMode::Repeat || wrap == WrapMode::MirroredRepeat
}

fn is_cube(param: ParamId) -> bool {
    matches!(
        param,
        ParamId::Sampler2 | ParamId::Sampler7 | ParamId::Sampler8
    )
}

fn validate_wrap_mode_tiling(
    validation: &mut ModelValidationErrors,
    matl: &MatlData,
    modl: Option<&ModlData>,
    mesh: Option<&MeshData>,
) {
    // Both the modl and mesh should be present to determine material assignments.
    if let (Some(modl), Some(mesh)) = (modl, mesh) {
        for (entry_index, entry) in matl.entries.iter().enumerate() {
            for (_, o) in mesh.objects.iter().enumerate().filter(|(_, o)| {
                modl.entries
                    .iter()
                    .filter(|e| e.material_label == entry.material_label)
                    .any(|e| e.mesh_object_name == o.name && e.mesh_object_subindex == o.subindex)
            }) {
                // Combine samplers to reduce the number of errors.
                let mut samplers = Vec::new();

                // Don't check cube maps since they should use clamp to edge.
                for sampler in entry.samplers.iter().filter(|s| {
                    !is_cube(s.param_id) && (!is_repeat(s.data.wraps) || !is_repeat(s.data.wrapt))
                }) {
                    let uv_name = match sampler.param_id {
                        ParamId::Sampler0
                        | ParamId::Sampler4
                        | ParamId::Sampler5
                        | ParamId::Sampler6
                        | ParamId::Sampler10 => "map1",
                        ParamId::Sampler3 | ParamId::Sampler9 => "bake1",
                        _ => "",
                    };

                    // Ignore additional UV layers for now.
                    // This prevents unwanted errors for iris textures using uvSet.
                    if !uv_name.is_empty() {
                        // Only check the corresponding UV coordinates for each sampler.
                        for a in o.texture_coordinates.iter().filter(|a| a.name == uv_name) {
                            // It's normal to have UVs slightly outside the first quadrant.
                            // UVs well outside the first quadrant probably expect tiling.
                            let (min_u, max_u, min_v, max_v) = get_uv_range(&a.data);
                            if min_u < -0.2 || max_u > 1.2 || min_v < -0.2 || max_v > 1.2 {
                                samplers.push(sampler.param_id);
                            }
                        }
                    }
                }

                // TODO: Group by sampler instead of mesh?
                // TODO: Only validate the first layer (map1/bake1) to avoid flagging eye materials?
                if !samplers.is_empty() {
                    let matl_error = MatlValidationError {
                        entry_index,
                        kind: MatlValidationErrorKind::WrapModeClampsUvs {
                            material_label: entry.material_label.clone(),
                            mesh_name: o.name.clone(),
                            samplers,
                        },
                    };
                    validation.matl_errors.push(matl_error);
                }
            }
        }
    }
}

fn get_uv_range(data: &VectorData) -> (f32, f32, f32, f32) {
    // Check if any UV coordinates are outside the 0.0 to 1.0 range.
    let mut max_u = 0.0f32;
    let mut min_u = 0.0f32;
    let mut max_v = 0.0f32;
    let mut min_v = 0.0f32;

    let mut update_min_max = |u: &f32, v: &f32| {
        min_u = min_u.min(*u);
        max_u = max_u.max(*u);

        min_v = min_v.min(*v);
        max_v = max_v.max(*v);
    };

    // TODO: Associate the correct sampler with the correct UV map.
    // TODO: Check UV map bounds individually.
    // TODO: Add tests for this.
    match data {
        VectorData::Vector2(values) => {
            for [u, v] in values {
                update_min_max(u, v);
            }
        }
        VectorData::Vector3(values) => {
            for [u, v, _] in values {
                update_min_max(u, v);
            }
        }
        VectorData::Vector4(values) => {
            for [u, v, _, _] in values {
                update_min_max(u, v);
            }
        }
    }

    (min_u, max_u, min_v, max_v)
}

fn validate_texture_format_usage(
    validation: &mut ModelValidationErrors,
    matl: &MatlData,
    nutexbs: &[(String, FileResult<NutexbFile>)],
) {
    for (entry_index, entry) in matl.entries.iter().enumerate() {
        for texture in &entry.textures {
            if let Some((f, Ok(nutexb))) = nutexbs.iter().find(|(f, _)| {
                Path::new(f)
                    .with_extension("")
                    .as_os_str()
                    .eq_ignore_ascii_case(&texture.data)
            }) {
                // Check for sRGB mismatches.
                if expects_srgb(texture.param_id) != is_srgb(nutexb.footer.image_format) {
                    let error = MatlValidationError {
                        entry_index,
                        kind: MatlValidationErrorKind::UnexpectedTextureFormat {
                            material_label: entry.material_label.clone(),
                            param_id: texture.param_id,
                            nutexb: f.clone(),
                            format: nutexb.footer.image_format,
                        },
                    };
                    validation.matl_errors.push(error);

                    let error = NutexbValidationError::FormatInvalidForUsage {
                        nutexb: f.clone(),
                        format: nutexb.footer.image_format,
                        param: texture.param_id,
                    };
                    validation.nutexb_errors.push(error);
                }
            }
        }
    }
}

fn validate_texture_assignments<'a, 'b>(
    validation: &mut ModelValidationErrors,
    matl: &MatlData,
    nutexbs: &'b [(String, FileResult<NutexbFile>)],
    default_textures: impl Iterator<Item = (&'a String, TextureDimension)> + Clone,
) where
    'b: 'a,
{
    for (entry_index, entry) in matl.entries.iter().enumerate() {
        // If a material is unused, every texture may show up as missing.
        // Group missing textures to avoid flooding the log windows with errors.
        let textures: Vec<_> = entry
            .textures
            .iter()
            .filter_map(|texture| {
                if !nutexbs
                    .iter()
                    .map(|(f, _)| f)
                    .chain(default_textures.clone().map(|(f, _)| f))
                    .any(|f| {
                        Path::new(f)
                            .with_extension("")
                            .as_os_str()
                            .eq_ignore_ascii_case(&texture.data)
                    })
                {
                    Some(texture.data.clone())
                } else {
                    None
                }
            })
            .collect();

        if !textures.is_empty() {
            let error = MatlValidationError {
                entry_index,
                kind: MatlValidationErrorKind::MissingTextures {
                    material_label: entry.material_label.clone(),
                    textures,
                },
            };
            validation.matl_errors.push(error);
        }
    }
}

fn validate_texture_dimensions<'a>(
    validation: &mut ModelValidationErrors,
    matl: &MatlData,
    nutexbs: &'a [(String, FileResult<NutexbFile>)],
    default_textures: impl Iterator<Item = (&'a String, TextureDimension)> + Clone,
) {
    for (entry_index, entry) in matl.entries.iter().enumerate() {
        for texture in &entry.textures {
            if let Some((f, actual)) = nutexbs
                .iter()
                .filter_map(|(f, n)| Some((f, TextureDimension::from_nutexb(n.as_ref().ok()?))))
                .chain(default_textures.clone())
                .find(|(f, _)| {
                    Path::new(f)
                        .with_extension("")
                        .as_os_str()
                        .eq_ignore_ascii_case(&texture.data)
                })
            {
                let expected = TextureDimension::from_param(texture.param_id);
                if actual != expected {
                    // The dimension is a fundamental part of the texture.
                    // Add errors to the matl since users should just assign a new texture.
                    let error = MatlValidationError {
                        entry_index,
                        kind: MatlValidationErrorKind::UnexpectedTextureDimension {
                            material_label: entry.material_label.clone(),
                            param_id: texture.param_id,
                            texture: f.clone(),
                            expected,
                            actual,
                        },
                    };
                    validation.matl_errors.push(error);
                }
            }
        }
    }
}

fn validate_renormal_material_entries(
    validation: &mut ModelValidationErrors,
    matl: &MatlData,
    adj: Option<&AdjData>,
    modl: Option<&ModlData>,
    mesh: Option<&MeshData>,
) {
    // TODO: Is this check case sensitive?
    for (entry_index, entry) in matl
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.material_label.contains("RENORMAL"))
    {
        if let Some(adj) = adj {
            if let Some(modl) = modl {
                if let Some(mesh) = mesh {
                    for (mesh_index, mesh) in mesh.objects.iter().enumerate().filter(|(_, m)| {
                        modl.entries.iter().any(|e| {
                            e.mesh_object_name == m.name
                                && e.mesh_object_subindex == m.subindex
                                && e.material_label == entry.material_label
                        })
                    }) {
                        if !adj
                            .entries
                            .iter()
                            .any(|a| a.mesh_object_index == mesh_index)
                        {
                            let error = MatlValidationError {
                                entry_index,
                                kind:
                                    MatlValidationErrorKind::RenormalMaterialMissingMeshAdjEntry {
                                        material_label: entry.material_label.clone(),
                                        mesh_name: mesh.name.clone(),
                                    },
                            };
                            validation.matl_errors.push(error);

                            let error = AdjValidationError::MissingRenormalEntry {
                                mesh_object_index: mesh_index,
                                mesh_name: mesh.name.clone(),
                                material_label: entry.material_label.clone(),
                            };
                            validation.adj_errors.push(error);
                        }
                    }
                }
            }
        } else {
            let error = MatlValidationError {
                entry_index,
                kind: MatlValidationErrorKind::RenormalMaterialMissingAdj {
                    material_label: entry.material_label.clone(),
                },
            };
            validation.matl_errors.push(error);
        }
    }
}

fn expects_srgb(texture: ParamId) -> bool {
    // These textures will render inaccurately with sRGB.
    // TODO: What should Texture8 use?
    !matches!(
        texture,
        ParamId::Texture2
            | ParamId::Texture4
            | ParamId::Texture6
            | ParamId::Texture7
            | ParamId::Texture16
    )
}

fn is_srgb(format: NutexbFormat) -> bool {
    matches!(
        format,
        NutexbFormat::R8G8B8A8Srgb
            | NutexbFormat::B8G8R8A8Srgb
            | NutexbFormat::BC1Srgb
            | NutexbFormat::BC2Srgb
            | NutexbFormat::BC3Srgb
            | NutexbFormat::BC7Srgb
    )
}

fn validate_mesh_subindices(validation: &mut ModelValidationErrors, mesh: &MeshData) {
    // Subindices for mesh objects with the same name should be unique.
    // This ensures material and vertex weights can be properly assigned.
    let mut subindices_by_name = HashMap::new();
    for (i, o) in mesh.objects.iter().enumerate() {
        if !subindices_by_name
            .entry(&o.name)
            .or_insert_with(HashSet::new)
            .insert(o.subindex)
        {
            let error = MeshValidationError {
                mesh_object_index: i,
                kind: MeshValidationErrorKind::DuplicateSubindex {
                    mesh_name: o.name.clone(),
                    subindex: o.subindex,
                },
            };
            validation.mesh_errors.push(error);
        }
    }
}

fn validate_mesh_vertex_weights(validation: &mut ModelValidationErrors, mesh: &MeshData) {
    for (i, o) in mesh.objects.iter().enumerate() {
        let mut has_zero_weights = false;

        // TODO: Also validate 4 weights per vertex.
        let mut weight_total_by_vertex = HashMap::<u32, f32>::new();
        for influence in &o.bone_influences {
            for w in &influence.vertex_weights {
                *weight_total_by_vertex.entry(w.vertex_index).or_default() += w.vertex_weight;

                if w.vertex_weight == 0.0 {
                    has_zero_weights = true;
                }
            }
        }

        // Use a threshold in case weights don't sum exactly to 1.0.
        if weight_total_by_vertex
            .iter()
            .any(|(_, t)| !relative_eq!(*t, 1.0, epsilon = 0.001))
        {
            let error = MeshValidationError {
                mesh_object_index: i,
                kind: MeshValidationErrorKind::VertexWeightsNotNormalized {
                    mesh_name: o.name.clone(),
                },
            };
            validation.mesh_errors.push(error);
        }

        if has_zero_weights {
            let error = MeshValidationError {
                mesh_object_index: i,
                kind: MeshValidationErrorKind::VertexWeightsZero {
                    mesh_name: o.name.clone(),
                },
            };
            validation.mesh_errors.push(error);
        }
    }
}

fn validate_modl_entries(
    validation: &mut ModelValidationErrors,
    modl: &ModlData,
    matl: Option<&MatlData>,
    mesh: Option<&MeshData>,
) {
    if let Some(matl) = matl {
        for (entry_index, entry) in modl.entries.iter().enumerate() {
            if !matl
                .entries
                .iter()
                .any(|e| e.material_label == entry.material_label)
            {
                let error = ModlValidationError {
                    entry_index,
                    kind: ModlValidationErrorKind::InvalidMaterial {
                        material_label: entry.material_label.clone(),
                    },
                };
                validation.modl_errors.push(error);
            }
        }
    }

    if let Some(mesh) = mesh {
        for (entry_index, entry) in modl.entries.iter().enumerate() {
            if !mesh.objects.iter().any(|o| {
                o.name == entry.mesh_object_name && o.subindex == entry.mesh_object_subindex
            }) {
                let error = ModlValidationError {
                    entry_index,
                    kind: ModlValidationErrorKind::InvalidMeshObject {
                        mesh_object_name: entry.mesh_object_name.clone(),
                        mesh_object_subindex: entry.mesh_object_subindex as usize,
                    },
                };
                validation.modl_errors.push(error);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use nutexb::{NutexbFile, NutexbFooter, NutexbFormat};
    use ssbh_data::{
        adj_data::AdjEntryData,
        matl_data::{
            BlendFactor, BlendStateData, BlendStateParam, MatlEntryData, SamplerData, SamplerParam,
            TextureParam,
        },
        mesh_data::{AttributeData, BoneInfluence, MeshObjectData, VectorData, VertexWeight},
        modl_data::ModlEntryData,
    };

    use super::*;

    fn nutexb(image_format: NutexbFormat) -> NutexbFile {
        NutexbFile {
            data: Vec::new(),
            layer_mipmaps: Vec::new(),
            footer: NutexbFooter {
                string: Default::default(),
                width: 1,
                height: 1,
                depth: 1,
                image_format,
                unk2: 1,
                mipmap_count: 1,
                unk3: 1,
                layer_count: 1,
                data_size: 0,
                version: (1, 2),
            },
        }
    }

    fn nutexb_cube(image_format: NutexbFormat) -> NutexbFile {
        NutexbFile {
            data: Vec::new(),
            layer_mipmaps: Vec::new(),
            footer: NutexbFooter {
                string: Default::default(),
                width: 64,
                height: 64,
                depth: 1,
                image_format,
                unk2: 1,
                mipmap_count: 1,
                unk3: 1,
                layer_count: 6,
                data_size: 0,
                version: (1, 2),
            },
        }
    }

    #[test]
    fn required_attributes_all_missing() {
        let shader_database = ShaderDatabase::new();
        let matl = MatlData {
            major_version: 1,
            minor_version: 6,
            entries: vec![MatlEntryData {
                material_label: "a".to_owned(),
                shader_label: "SFX_PBS_010002000800824f_opaque".to_owned(),
                blend_states: Vec::new(),
                floats: Vec::new(),
                booleans: Vec::new(),
                vectors: Vec::new(),
                rasterizer_states: Vec::new(),
                samplers: Vec::new(),
                textures: Vec::new(),
            }],
        };
        let mesh = MeshData {
            major_version: 1,
            minor_version: 10,
            objects: vec![MeshObjectData {
                name: "object1".to_owned(),
                subindex: 0,
                ..Default::default()
            }],
        };
        let modl = ModlData {
            major_version: 1,
            minor_version: 0,
            model_name: String::new(),
            skeleton_file_name: String::new(),
            material_file_names: Vec::new(),
            animation_file_name: None,
            mesh_file_name: String::new(),
            entries: vec![ModlEntryData {
                mesh_object_name: "object1".to_owned(),
                mesh_object_subindex: 0,
                material_label: "a".to_owned(),
            }],
        };

        let mut validation = ModelValidationErrors::default();
        validate_required_attributes(
            &mut validation,
            &matl,
            Some(&modl),
            Some(&mesh),
            &shader_database,
        );

        assert_eq!(
            vec![MatlValidationError {
                entry_index: 0,
                kind: MatlValidationErrorKind::MissingRequiredVertexAttributes {
                    material_label: "a".to_owned(),
                    mesh_name: "object1".to_owned(),
                    missing_attributes: vec!["map1".to_owned(), "uvSet".to_owned()]
                }
            }],
            validation.matl_errors
        );

        assert_eq!(
            vec![MeshValidationError {
                mesh_object_index: 0,
                kind: MeshValidationErrorKind::MissingRequiredVertexAttributes {
                    mesh_name: "object1".to_owned(),
                    material_label: "a".to_owned(),
                    missing_attributes: vec!["map1".to_owned(), "uvSet".to_owned()]
                }
            }],
            validation.mesh_errors
        );

        assert_eq!(
            r#"Mesh "object1" is missing attributes ["map1", "uvSet"] required by assigned material "a"."#,
            format!("{}", validation.matl_errors[0])
        );

        assert_eq!(
            r#"Mesh "object1" is missing attributes ["map1", "uvSet"] required by assigned material "a"."#,
            format!("{}", validation.mesh_errors[0])
        );
    }

    #[test]
    fn renormal_material_missing_adj() {
        let matl = MatlData {
            major_version: 1,
            minor_version: 6,
            entries: vec![MatlEntryData {
                material_label: "RENORMAL_a".to_owned(),
                shader_label: "SFX_PBS_010002000800824f_opaque".to_owned(),
                blend_states: Vec::new(),
                floats: Vec::new(),
                booleans: Vec::new(),
                vectors: Vec::new(),
                rasterizer_states: Vec::new(),
                samplers: Vec::new(),
                textures: Vec::new(),
            }],
        };
        let mesh = MeshData {
            major_version: 1,
            minor_version: 10,
            objects: vec![MeshObjectData {
                name: "object1".to_owned(),
                subindex: 0,
                ..Default::default()
            }],
        };
        let modl = ModlData {
            major_version: 1,
            minor_version: 0,
            model_name: String::new(),
            skeleton_file_name: String::new(),
            material_file_names: Vec::new(),
            animation_file_name: None,
            mesh_file_name: String::new(),
            entries: vec![ModlEntryData {
                mesh_object_name: "object1".to_owned(),
                mesh_object_subindex: 0,
                material_label: "RENORMAL_a".to_owned(),
            }],
        };

        let mut validation = ModelValidationErrors::default();
        validate_renormal_material_entries(&mut validation, &matl, None, Some(&modl), Some(&mesh));

        assert_eq!(
            vec![MatlValidationError {
                entry_index: 0,
                kind: MatlValidationErrorKind::RenormalMaterialMissingAdj {
                    material_label: "RENORMAL_a".to_owned(),
                }
            }],
            validation.matl_errors
        );

        assert_eq!(
            r#"Material "RENORMAL_a" is a RENORMAL material, but the model.adjb file is missing."#,
            format!("{}", validation.matl_errors[0])
        );
    }

    #[test]
    fn renormal_material_missing_adj_entry() {
        let matl = MatlData {
            major_version: 1,
            minor_version: 6,
            entries: vec![
                MatlEntryData {
                    material_label: "RENORMAL_a".to_owned(),
                    shader_label: "SFX_PBS_010002000800824f_opaque".to_owned(),
                    blend_states: Vec::new(),
                    floats: Vec::new(),
                    booleans: Vec::new(),
                    vectors: Vec::new(),
                    rasterizer_states: Vec::new(),
                    samplers: Vec::new(),
                    textures: Vec::new(),
                },
                MatlEntryData {
                    material_label: "mat_a".to_owned(),
                    shader_label: "SFX_PBS_010002000800824f_opaque".to_owned(),
                    blend_states: Vec::new(),
                    floats: Vec::new(),
                    booleans: Vec::new(),
                    vectors: Vec::new(),
                    rasterizer_states: Vec::new(),
                    samplers: Vec::new(),
                    textures: Vec::new(),
                },
            ],
        };
        let mesh = MeshData {
            major_version: 1,
            minor_version: 10,
            objects: vec![
                MeshObjectData {
                    name: "object0".to_owned(),
                    subindex: 0,
                    ..Default::default()
                },
                MeshObjectData {
                    name: "object1".to_owned(),
                    subindex: 0,
                    ..Default::default()
                },
            ],
        };
        let modl = ModlData {
            major_version: 1,
            minor_version: 0,
            model_name: String::new(),
            skeleton_file_name: String::new(),
            material_file_names: Vec::new(),
            animation_file_name: None,
            mesh_file_name: String::new(),
            entries: vec![
                ModlEntryData {
                    mesh_object_name: "object0".to_owned(),
                    mesh_object_subindex: 0,
                    material_label: "mat_a".to_owned(),
                },
                ModlEntryData {
                    mesh_object_name: String::new(),
                    mesh_object_subindex: 0,
                    material_label: String::new(),
                },
                ModlEntryData {
                    mesh_object_name: "object1".to_owned(),
                    mesh_object_subindex: 0,
                    material_label: "RENORMAL_a".to_owned(),
                },
            ],
        };
        let adj = AdjData {
            entries: vec![AdjEntryData {
                mesh_object_index: 0,
                vertex_adjacency: Vec::new(),
            }],
        };

        let mut validation = ModelValidationErrors::default();
        validate_renormal_material_entries(
            &mut validation,
            &matl,
            Some(&adj),
            Some(&modl),
            Some(&mesh),
        );

        assert_eq!(
            vec![MatlValidationError {
                entry_index: 0,
                kind: MatlValidationErrorKind::RenormalMaterialMissingMeshAdjEntry {
                    material_label: "RENORMAL_a".to_owned(),
                    mesh_name: "object1".to_owned()
                }
            }],
            validation.matl_errors
        );

        assert_eq!(
            r#"Mesh "object1" has the RENORMAL material "RENORMAL_a" but no corresponding entry in the model.adjb."#,
            format!("{}", validation.matl_errors[0])
        );

        assert_eq!(
            vec![AdjValidationError::MissingRenormalEntry {
                mesh_object_index: 1,
                mesh_name: "object1".to_owned(),
                material_label: "RENORMAL_a".to_owned()
            }],
            validation.adj_errors
        );

        assert_eq!(
            r#"Missing entry for mesh "object1" with the RENORMAL material "RENORMAL_a"."#,
            format!("{}", validation.adj_errors[0])
        );
    }

    #[test]
    fn texture_format_usage_all_invalid() {
        let matl = MatlData {
            major_version: 1,
            minor_version: 6,
            entries: vec![MatlEntryData {
                material_label: "a".to_owned(),
                shader_label: "SFX_PBS_010002000800824f_opaque".to_owned(),
                blend_states: Vec::new(),
                floats: Vec::new(),
                booleans: Vec::new(),
                vectors: Vec::new(),
                rasterizer_states: Vec::new(),
                samplers: Vec::new(),
                textures: vec![
                    TextureParam {
                        param_id: ParamId::Texture0,
                        data: "texture0".to_owned(),
                    },
                    TextureParam {
                        param_id: ParamId::Texture4,
                        data: "texture4".to_owned(),
                    },
                ],
            }],
        };

        let textures = vec![
            ("texture0".to_owned(), Ok(nutexb(NutexbFormat::BC1Unorm))),
            ("texture4".to_owned(), Ok(nutexb(NutexbFormat::BC2Srgb))),
        ];

        let mut validation = ModelValidationErrors::default();
        validate_texture_format_usage(&mut validation, &matl, &textures);

        assert_eq!(
            vec![
                MatlValidationError {
                    entry_index: 0,
                    kind: MatlValidationErrorKind::UnexpectedTextureFormat {
                        material_label: "a".to_owned(),
                        param_id: ParamId::Texture0,
                        nutexb: "texture0".to_owned(),
                        format: NutexbFormat::BC1Unorm
                    }
                },
                MatlValidationError {
                    entry_index: 0,
                    kind: MatlValidationErrorKind::UnexpectedTextureFormat {
                        material_label: "a".to_owned(),
                        param_id: ParamId::Texture4,
                        nutexb: "texture4".to_owned(),
                        format: NutexbFormat::BC2Srgb
                    }
                }
            ],
            validation.matl_errors
        );

        assert_eq!(
            r#"Texture "texture0" for material "a" has format BC1Unorm, but Texture0 expects an sRGB format."#,
            format!("{}", validation.matl_errors[0])
        );
        assert_eq!(
            r#"Texture "texture4" for material "a" has format BC2Srgb, but Texture4 does not expect an sRGB format."#,
            format!("{}", validation.matl_errors[1])
        );

        assert_eq!(
            vec![
                NutexbValidationError::FormatInvalidForUsage {
                    nutexb: "texture0".to_owned(),
                    param: ParamId::Texture0,
                    format: NutexbFormat::BC1Unorm
                },
                NutexbValidationError::FormatInvalidForUsage {
                    nutexb: "texture4".to_owned(),
                    param: ParamId::Texture4,
                    format: NutexbFormat::BC2Srgb
                }
            ],
            validation.nutexb_errors
        );

        assert_eq!(
            r#"Texture "texture0" has format BC1Unorm, but Texture0 expects an sRGB format."#,
            format!("{}", validation.nutexb_errors[0])
        );
        assert_eq!(
            r#"Texture "texture4" has format BC2Srgb, but Texture4 does not expect an sRGB format."#,
            format!("{}", validation.nutexb_errors[1])
        );
    }

    #[test]
    fn textures_one_missing() {
        let matl = MatlData {
            major_version: 1,
            minor_version: 6,
            entries: vec![MatlEntryData {
                material_label: "a".to_owned(),
                shader_label: "SFX_PBS_010002000800824f_opaque".to_owned(),
                blend_states: Vec::new(),
                floats: Vec::new(),
                booleans: Vec::new(),
                vectors: Vec::new(),
                rasterizer_states: Vec::new(),
                samplers: Vec::new(),
                textures: vec![
                    TextureParam {
                        param_id: ParamId::Texture0,
                        data: "texture0".to_owned(),
                    },
                    TextureParam {
                        param_id: ParamId::Texture1,
                        data: "texture1".to_owned(),
                    },
                    TextureParam {
                        param_id: ParamId::Texture4,
                        data: "texture4".to_owned(),
                    },
                    TextureParam {
                        param_id: ParamId::Texture7,
                        data: "#replace_cubemap".to_owned(),
                    },
                ],
            }],
        };

        let textures = vec![
            ("texture2".to_owned(), Ok(nutexb(NutexbFormat::BC7Srgb))),
            ("texture4".to_owned(), Ok(nutexb(NutexbFormat::BC7Unorm))),
        ];

        let mut validation = ModelValidationErrors::default();
        validate_texture_assignments(
            &mut validation,
            &matl,
            &textures,
            [(
                &"#replace_cubemap".to_owned(),
                TextureDimension::TextureCube,
            )]
            .into_iter(),
        );

        assert_eq!(
            vec![MatlValidationError {
                entry_index: 0,
                kind: MatlValidationErrorKind::MissingTextures {
                    material_label: "a".to_owned(),
                    textures: vec!["texture0".to_owned(), "texture1".to_owned()],
                }
            },],
            validation.matl_errors
        );

        assert_eq!(
            r#"Textures ["texture0", "texture1"] for material "a" are missing."#,
            format!("{}", validation.matl_errors[0])
        );
    }

    #[test]
    fn texture_dimension_invalid() {
        let matl = MatlData {
            major_version: 1,
            minor_version: 6,
            entries: vec![MatlEntryData {
                material_label: "a".to_owned(),
                shader_label: "SFX_PBS_010002000800824f_opaque".to_owned(),
                blend_states: Vec::new(),
                floats: Vec::new(),
                booleans: Vec::new(),
                vectors: Vec::new(),
                rasterizer_states: Vec::new(),
                samplers: Vec::new(),
                textures: vec![
                    TextureParam {
                        param_id: ParamId::Texture0,
                        data: "texture0".to_owned(),
                    },
                    TextureParam {
                        param_id: ParamId::Texture1,
                        data: "#replace_cubemap".to_owned(),
                    },
                    TextureParam {
                        param_id: ParamId::Texture7,
                        data: "texture7".to_owned(),
                    },
                ],
            }],
        };

        let textures = vec![
            (
                "texture0".to_owned(),
                Ok(nutexb_cube(NutexbFormat::BC1Unorm)),
            ),
            ("texture7".to_owned(), Ok(nutexb(NutexbFormat::BC2Srgb))),
        ];

        let mut validation = ModelValidationErrors::default();
        validate_texture_dimensions(
            &mut validation,
            &matl,
            &textures,
            [(
                &"#replace_cubemap".to_owned(),
                TextureDimension::TextureCube,
            )]
            .into_iter(),
        );

        assert_eq!(
            vec![
                MatlValidationError {
                    entry_index: 0,
                    kind: MatlValidationErrorKind::UnexpectedTextureDimension {
                        material_label: "a".to_owned(),
                        param_id: ParamId::Texture0,
                        texture: "texture0".to_owned(),
                        expected: TextureDimension::Texture2d,
                        actual: TextureDimension::TextureCube
                    }
                },
                MatlValidationError {
                    entry_index: 0,
                    kind: MatlValidationErrorKind::UnexpectedTextureDimension {
                        material_label: "a".to_owned(),
                        param_id: ParamId::Texture1,
                        texture: "#replace_cubemap".to_owned(),
                        expected: TextureDimension::Texture2d,
                        actual: TextureDimension::TextureCube
                    }
                },
                MatlValidationError {
                    entry_index: 0,
                    kind: MatlValidationErrorKind::UnexpectedTextureDimension {
                        material_label: "a".to_owned(),
                        param_id: ParamId::Texture7,
                        texture: "texture7".to_owned(),
                        expected: TextureDimension::TextureCube,
                        actual: TextureDimension::Texture2d
                    }
                }
            ],
            validation.matl_errors
        );

        assert_eq!(
            r#"Texture "texture0" for material "a" has dimensions TextureCube, but Texture0 requires Texture2d."#,
            format!("{}", validation.matl_errors[0])
        );
        assert_eq!(
            r##"Texture "#replace_cubemap" for material "a" has dimensions TextureCube, but Texture1 requires Texture2d."##,
            format!("{}", validation.matl_errors[1])
        );
        assert_eq!(
            r#"Texture "texture7" for material "a" has dimensions Texture2d, but Texture7 requires TextureCube."#,
            format!("{}", validation.matl_errors[2])
        );
    }

    #[test]
    fn mesh_subindices_single_duplicate() {
        let mesh = MeshData {
            major_version: 1,
            minor_version: 10,
            objects: vec![
                MeshObjectData {
                    name: "a".to_owned(),
                    subindex: 0,
                    ..Default::default()
                },
                MeshObjectData {
                    name: "b".to_owned(),
                    subindex: 1,
                    ..Default::default()
                },
                MeshObjectData {
                    name: "a".to_owned(),
                    subindex: 0,
                    ..Default::default()
                },
                MeshObjectData {
                    name: "c".to_owned(),
                    subindex: 0,
                    ..Default::default()
                },
            ],
        };

        let mut validation = ModelValidationErrors::default();
        validate_mesh_subindices(&mut validation, &mesh);

        assert_eq!(
            vec![MeshValidationError {
                mesh_object_index: 2,
                kind: MeshValidationErrorKind::DuplicateSubindex {
                    mesh_name: "a".to_owned(),
                    subindex: 0
                }
            }],
            validation.mesh_errors
        );

        assert_eq!(
            r#"Mesh "a" repeats subindex 0. Meshes with the same name must have unique subindices."#,
            format!("{}", validation.mesh_errors[0])
        );
    }

    #[test]
    fn wrap_mode_tiling_clamped_uvs() {
        let matl = MatlData {
            major_version: 1,
            minor_version: 6,
            entries: vec![MatlEntryData {
                material_label: "a".to_owned(),
                shader_label: "SFX_PBS_010002000800824f_opaque".to_owned(),
                blend_states: Vec::new(),
                floats: Vec::new(),
                booleans: Vec::new(),
                vectors: Vec::new(),
                rasterizer_states: Vec::new(),
                samplers: vec![
                    SamplerParam {
                        param_id: ParamId::Sampler0,
                        data: SamplerData {
                            wraps: WrapMode::ClampToEdge,
                            wrapt: WrapMode::ClampToEdge,
                            ..Default::default()
                        },
                    },
                    SamplerParam {
                        param_id: ParamId::Sampler3,
                        data: SamplerData {
                            wraps: WrapMode::ClampToEdge,
                            wrapt: WrapMode::ClampToEdge,
                            ..Default::default()
                        },
                    },
                    SamplerParam {
                        param_id: ParamId::Sampler4,
                        data: SamplerData {
                            wraps: WrapMode::ClampToEdge,
                            wrapt: WrapMode::ClampToEdge,
                            ..Default::default()
                        },
                    },
                    SamplerParam {
                        param_id: ParamId::Sampler2,
                        data: SamplerData::default(),
                    },
                    SamplerParam {
                        param_id: ParamId::Sampler7,
                        data: SamplerData::default(),
                    },
                    SamplerParam {
                        param_id: ParamId::Sampler8,
                        data: SamplerData::default(),
                    },
                ],
                textures: Vec::new(),
            }],
        };
        let mesh = MeshData {
            major_version: 1,
            minor_version: 10,
            objects: vec![MeshObjectData {
                name: "object1".to_owned(),
                subindex: 0,
                texture_coordinates: vec![
                    AttributeData {
                        name: "map1".to_owned(),
                        data: VectorData::Vector2(vec![[0.0, 0.0], [-1.0, 1.5]]),
                    },
                    AttributeData {
                        name: "bake1".to_owned(),
                        data: VectorData::Vector2(vec![[-0.15, 0.0], [1.0, 1.15]]),
                    },
                ],
                ..Default::default()
            }],
        };
        let modl = ModlData {
            major_version: 1,
            minor_version: 0,
            model_name: String::new(),
            skeleton_file_name: String::new(),
            material_file_names: Vec::new(),
            animation_file_name: None,
            mesh_file_name: String::new(),
            entries: vec![ModlEntryData {
                mesh_object_name: "object1".to_owned(),
                mesh_object_subindex: 0,
                material_label: "a".to_owned(),
            }],
        };

        let mut validation = ModelValidationErrors::default();
        validate_wrap_mode_tiling(&mut validation, &matl, Some(&modl), Some(&mesh));

        // Sampler3 isn't included since bake1 UVs are still in range.
        assert_eq!(
            vec![MatlValidationError {
                entry_index: 0,
                kind: MatlValidationErrorKind::WrapModeClampsUvs {
                    material_label: "a".to_owned(),
                    mesh_name: "object1".to_owned(),
                    samplers: vec![ParamId::Sampler0, ParamId::Sampler4],
                }
            }],
            validation.matl_errors
        );

        assert_eq!(
            "Samplers [Sampler0, Sampler4] for material \"a\" will clamp UV coordinates for mesh \"object1\".\nUse wrap mode Repeat if the texture should tile.",
            format!("{}", validation.matl_errors[0])
        );
    }

    #[test]
    fn wrap_mode_tiling_all_repeat() {
        let matl = MatlData {
            major_version: 1,
            minor_version: 6,
            entries: vec![MatlEntryData {
                material_label: "a".to_owned(),
                shader_label: "SFX_PBS_010002000800824f_opaque".to_owned(),
                blend_states: Vec::new(),
                floats: Vec::new(),
                booleans: Vec::new(),
                vectors: Vec::new(),
                rasterizer_states: Vec::new(),
                samplers: vec![SamplerParam {
                    param_id: ParamId::Sampler0,
                    data: SamplerData {
                        wraps: WrapMode::Repeat,
                        wrapt: WrapMode::MirroredRepeat,
                        ..Default::default()
                    },
                }],
                textures: Vec::new(),
            }],
        };
        let mesh = MeshData {
            major_version: 1,
            minor_version: 10,
            objects: vec![MeshObjectData {
                name: "object1".to_owned(),
                subindex: 0,
                texture_coordinates: vec![AttributeData {
                    name: "map1".to_owned(),
                    data: VectorData::Vector2(vec![[0.0, 0.0], [-1.0, 1.5]]),
                }],
                ..Default::default()
            }],
        };
        let modl = ModlData {
            major_version: 1,
            minor_version: 0,
            model_name: String::new(),
            skeleton_file_name: String::new(),
            material_file_names: Vec::new(),
            animation_file_name: None,
            mesh_file_name: String::new(),
            entries: vec![ModlEntryData {
                mesh_object_name: "object1".to_owned(),
                mesh_object_subindex: 0,
                material_label: "a".to_owned(),
            }],
        };

        let mut validation = ModelValidationErrors::default();
        validate_wrap_mode_tiling(&mut validation, &matl, Some(&modl), Some(&mesh));

        assert!(validation.matl_errors.is_empty());
    }

    #[test]
    fn shader_label_invalid() {
        let shader_database = ShaderDatabase::new();
        let matl = MatlData {
            major_version: 1,
            minor_version: 6,
            entries: vec![
                MatlEntryData {
                    material_label: "a".to_owned(),
                    shader_label: "SFX_PBS_010002000800824f_opaque".to_owned(),
                    blend_states: Vec::new(),
                    floats: Vec::new(),
                    booleans: Vec::new(),
                    vectors: Vec::new(),
                    rasterizer_states: Vec::new(),
                    samplers: Vec::new(),
                    textures: Vec::new(),
                },
                MatlEntryData {
                    material_label: "b".to_owned(),
                    shader_label: "SFX_PBS_777002000800824f_opaque".to_owned(),
                    blend_states: Vec::new(),
                    floats: Vec::new(),
                    booleans: Vec::new(),
                    vectors: Vec::new(),
                    rasterizer_states: Vec::new(),
                    samplers: Vec::new(),
                    textures: Vec::new(),
                },
            ],
        };

        let mut validation = ModelValidationErrors::default();
        validate_shader_labels(&mut validation, &matl, &shader_database);

        assert_eq!(
            vec![MatlValidationError {
                entry_index: 1,
                kind: MatlValidationErrorKind::InvalidShaderLabel {
                    material_label: "b".to_owned(),
                    shader_label: "SFX_PBS_777002000800824f_opaque".to_owned()
                }
            }],
            validation.matl_errors
        );

        assert_eq!(
            r#"Shader label "SFX_PBS_777002000800824f_opaque" for material "b" is not a valid shader label."#,
            format!("{}", validation.matl_errors[0])
        );
    }

    #[test]
    fn premultiplied_shader_src_alpha() {
        let shader_database = ShaderDatabase::new();
        let matl = MatlData {
            major_version: 1,
            minor_version: 6,
            entries: vec![MatlEntryData {
                material_label: "a".to_owned(),
                shader_label: "SFX_PBS_0100000008018269_sort".to_owned(),
                blend_states: vec![BlendStateParam {
                    param_id: ParamId::BlendState0,
                    data: BlendStateData {
                        source_color: BlendFactor::SourceAlpha,
                        destination_color: BlendFactor::OneMinusSourceAlpha,
                        alpha_sample_to_coverage: false,
                    },
                }],
                floats: Vec::new(),
                booleans: Vec::new(),
                vectors: Vec::new(),
                rasterizer_states: Vec::new(),
                samplers: Vec::new(),
                textures: Vec::new(),
            }],
        };

        let mut validation = ModelValidationErrors::default();
        validate_premultiplied_blend(&mut validation, &matl, &shader_database);

        assert_eq!(
            vec![MatlValidationError {
                entry_index: 0,
                kind: MatlValidationErrorKind::PremultipliedShaderSrcAlpha {
                    material_label: "a".to_owned(),
                    shader_label: "SFX_PBS_0100000008018269_sort".to_owned()
                }
            }],
            validation.matl_errors
        );

        assert_eq!(
            r#"Material "a" uses Source Color "SourceAlpha", but shader "SFX_PBS_0100000008018269_sort" already premultiplies alpha.
Use a Source Color of "One" or use a shader that does not premultiply alpha."#,
            format!("{}", validation.matl_errors[0])
        );
    }

    #[test]
    fn premultiplied_shader_one() {
        let shader_database = ShaderDatabase::new();
        let matl = MatlData {
            major_version: 1,
            minor_version: 6,
            entries: vec![MatlEntryData {
                material_label: "a".to_owned(),
                shader_label: "SFX_PBS_0100000008018269_sort".to_owned(),
                blend_states: vec![BlendStateParam {
                    param_id: ParamId::BlendState0,
                    data: BlendStateData {
                        source_color: BlendFactor::One,
                        destination_color: BlendFactor::OneMinusSourceAlpha,
                        alpha_sample_to_coverage: false,
                    },
                }],
                floats: Vec::new(),
                booleans: Vec::new(),
                vectors: Vec::new(),
                rasterizer_states: Vec::new(),
                samplers: Vec::new(),
                textures: Vec::new(),
            }],
        };

        let mut validation = ModelValidationErrors::default();
        validate_premultiplied_blend(&mut validation, &matl, &shader_database);

        assert!(validation.matl_errors.is_empty());
    }

    #[test]
    fn non_premultiplied_shader_source_alpha() {
        let shader_database = ShaderDatabase::new();
        let matl = MatlData {
            major_version: 1,
            minor_version: 6,
            entries: vec![MatlEntryData {
                material_label: "a".to_owned(),
                shader_label: "SFX_PBS_0100000008008269_opaque".to_owned(),
                blend_states: vec![BlendStateParam {
                    param_id: ParamId::BlendState0,
                    data: BlendStateData {
                        source_color: BlendFactor::SourceAlpha,
                        destination_color: BlendFactor::OneMinusSourceAlpha,
                        alpha_sample_to_coverage: false,
                    },
                }],
                floats: Vec::new(),
                booleans: Vec::new(),
                vectors: Vec::new(),
                rasterizer_states: Vec::new(),
                samplers: Vec::new(),
                textures: Vec::new(),
            }],
        };

        let mut validation = ModelValidationErrors::default();
        validate_premultiplied_blend(&mut validation, &matl, &shader_database);

        assert!(validation.matl_errors.is_empty());
    }

    #[test]
    fn invalid_modl_entries() {
        let matl = MatlData {
            major_version: 1,
            minor_version: 6,
            entries: vec![MatlEntryData {
                material_label: "a".to_owned(),
                shader_label: "SFX_PBS_010002000800824f_opaque".to_owned(),
                blend_states: Vec::new(),
                floats: Vec::new(),
                booleans: Vec::new(),
                vectors: Vec::new(),
                rasterizer_states: Vec::new(),
                samplers: Vec::new(),
                textures: Vec::new(),
            }],
        };
        let mesh = MeshData {
            major_version: 1,
            minor_version: 10,
            objects: vec![MeshObjectData {
                name: "object1".to_owned(),
                subindex: 0,
                ..Default::default()
            }],
        };
        let modl = ModlData {
            major_version: 1,
            minor_version: 0,
            model_name: String::new(),
            skeleton_file_name: String::new(),
            material_file_names: Vec::new(),
            animation_file_name: None,
            mesh_file_name: String::new(),
            entries: vec![
                ModlEntryData {
                    mesh_object_name: "object1".to_owned(),
                    mesh_object_subindex: 0,
                    material_label: "b".to_owned(),
                },
                ModlEntryData {
                    mesh_object_name: "object1".to_owned(),
                    mesh_object_subindex: 2,
                    material_label: "a".to_owned(),
                },
                ModlEntryData {
                    mesh_object_name: "object2".to_owned(),
                    mesh_object_subindex: 0,
                    material_label: "a".to_owned(),
                },
            ],
        };

        let mut validation = ModelValidationErrors::default();
        validate_modl_entries(&mut validation, &modl, Some(&matl), Some(&mesh));

        // Check each kind of invalid assignment.
        assert_eq!(
            vec![
                ModlValidationError {
                    entry_index: 0,
                    kind: ModlValidationErrorKind::InvalidMaterial {
                        material_label: "b".to_owned(),
                    }
                },
                ModlValidationError {
                    entry_index: 1,
                    kind: ModlValidationErrorKind::InvalidMeshObject {
                        mesh_object_name: "object1".to_owned(),
                        mesh_object_subindex: 2
                    }
                },
                ModlValidationError {
                    entry_index: 2,
                    kind: ModlValidationErrorKind::InvalidMeshObject {
                        mesh_object_name: "object2".to_owned(),
                        mesh_object_subindex: 0
                    }
                }
            ],
            validation.modl_errors
        );

        assert_eq!(
            r#"Modl entry assigns a material "b" not found in the model.numatb."#,
            format!("{}", validation.modl_errors[0])
        );
        assert_eq!(
            r#"Modl entry assigns to mesh "object1" not found in the model.numshb. Ensure the name and subindex are correct."#,
            format!("{}", validation.modl_errors[1])
        );
        assert_eq!(
            r#"Modl entry assigns to mesh "object2" not found in the model.numshb. Ensure the name and subindex are correct."#,
            format!("{}", validation.modl_errors[2])
        );
    }

    #[test]
    fn mesh_vertex_weights_normalized() {
        let mesh = MeshData {
            major_version: 1,
            minor_version: 10,
            objects: vec![
                MeshObjectData {
                    name: "a".to_owned(),
                    subindex: 0,
                    bone_influences: vec![
                        BoneInfluence {
                            bone_name: "bone1".to_owned(),
                            vertex_weights: vec![VertexWeight {
                                vertex_index: 0,
                                vertex_weight: 0.500,
                            }],
                        },
                        BoneInfluence {
                            bone_name: "bone2".to_owned(),
                            vertex_weights: vec![VertexWeight {
                                vertex_index: 0,
                                vertex_weight: 0.4999,
                            }],
                        },
                    ],
                    ..Default::default()
                },
                MeshObjectData {
                    name: "b".to_owned(),
                    subindex: 0,
                    ..Default::default()
                },
            ],
        };

        let mut validation = ModelValidationErrors::default();
        validate_mesh_vertex_weights(&mut validation, &mesh);

        assert!(validation.mesh_errors.is_empty());
    }

    #[test]
    fn mesh_vertex_weights_not_normalized() {
        let mesh = MeshData {
            major_version: 1,
            minor_version: 10,
            objects: vec![
                MeshObjectData {
                    name: "a".to_owned(),
                    subindex: 0,
                    bone_influences: vec![BoneInfluence {
                        bone_name: "bone".to_owned(),
                        vertex_weights: vec![
                            VertexWeight {
                                vertex_index: 0,
                                vertex_weight: 0.5,
                            },
                            VertexWeight {
                                vertex_index: 0,
                                vertex_weight: 0.4,
                            },
                        ],
                    }],
                    ..Default::default()
                },
                MeshObjectData {
                    name: "b".to_owned(),
                    subindex: 0,
                    ..Default::default()
                },
            ],
        };

        let mut validation = ModelValidationErrors::default();
        validate_mesh_vertex_weights(&mut validation, &mesh);

        assert_eq!(
            vec![MeshValidationError {
                mesh_object_index: 0,
                kind: MeshValidationErrorKind::VertexWeightsNotNormalized {
                    mesh_name: "a".to_owned(),
                }
            }],
            validation.mesh_errors
        );

        assert_eq!(
            r#"Vertex weights for mesh "a" are not normalized. Vertex weights should sum to 1.0."#,
            format!("{}", validation.mesh_errors[0])
        );
    }

    #[test]
    fn mesh_vertex_weights_zero() {
        let mesh = MeshData {
            major_version: 1,
            minor_version: 10,
            objects: vec![
                MeshObjectData {
                    name: "a".to_owned(),
                    subindex: 0,
                    bone_influences: vec![BoneInfluence {
                        bone_name: "bone".to_owned(),
                        vertex_weights: vec![
                            VertexWeight {
                                vertex_index: 0,
                                vertex_weight: 1.0,
                            },
                            VertexWeight {
                                vertex_index: 0,
                                vertex_weight: 0.0,
                            },
                        ],
                    }],
                    ..Default::default()
                },
                MeshObjectData {
                    name: "b".to_owned(),
                    subindex: 0,
                    ..Default::default()
                },
            ],
        };

        let mut validation = ModelValidationErrors::default();
        validate_mesh_vertex_weights(&mut validation, &mesh);

        assert_eq!(
            vec![MeshValidationError {
                mesh_object_index: 0,
                kind: MeshValidationErrorKind::VertexWeightsZero {
                    mesh_name: "a".to_owned(),
                }
            }],
            validation.mesh_errors
        );

        assert_eq!(
            r#"Mesh "a" has vertex weights with a weight of 0.0 that can be removed."#,
            format!("{}", validation.mesh_errors[0])
        );
    }
}
