use crate::FileResult;
use nutexb::{NutexbFile, NutexbFormat};
use ssbh_data::{matl_data::ParamId, prelude::*};
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
        default_texture_names: impl Iterator<Item = &'a String> + Clone,
    ) -> Self
    where
        'b: 'a,
    {
        // Each validation check may add errors to multiple related files.
        let mut validation = Self::default();

        let mesh = model.find_mesh();
        if let Some(mesh) = mesh {
            validate_mesh_subindices(&mut validation, mesh);
        }

        if let Some(matl) = model.find_matl() {
            validate_required_attributes(
                &mut validation,
                matl,
                model.find_modl(),
                model.find_mesh(),
                shader_database,
            );

            validate_texture_format_usage(&mut validation, matl, &model.nutexbs);
            validate_texture_dimensions(&mut validation, matl, &model.nutexbs);
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

// TODO: Check for unsupported vertex attribute names?
#[derive(Debug, PartialEq, Eq, Error)]
pub enum MeshValidationError {
    #[error("Mesh {mesh_name:?} is missing attributes {} required by assigned material {material_label:?}.",
        missing_attributes.join(", "),
    )]
    MissingRequiredVertexAttributes {
        mesh_object_index: usize,
        mesh_name: String,
        material_label: String,
        missing_attributes: Vec<String>,
    },

    #[error("Mesh {mesh_name:?} repeats subindex {subindex}. Subindices must be unique.")]
    DuplicateSubindex {
        mesh_object_index: usize,
        mesh_name: String,
        subindex: u64,
    },
}

impl MeshValidationError {
    pub fn mesh_index(&self) -> usize {
        // Use the index to associate errors to mesh objects.
        // The mesh name isn't always unique.
        match self {
            Self::MissingRequiredVertexAttributes {
                mesh_object_index, ..
            } => *mesh_object_index,
            Self::DuplicateSubindex {
                mesh_object_index, ..
            } => *mesh_object_index,
        }
    }
}

pub struct SkelValidationError;
impl Display for SkelValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

// TODO: Move the entry index and label out of the enum?
#[derive(Debug, PartialEq, Eq, Error)]
pub enum MatlValidationError {
    #[error("Mesh {mesh_name:?} is missing attributes {} required by assigned material {material_label:?}.",
        missing_attributes.join(", ")
    )]
    MissingRequiredVertexAttributes {
        entry_index: usize,
        material_label: String,
        mesh_name: String,
        missing_attributes: Vec<String>,
    },

    #[error("Texture {nutexb:?} for material {material_label:?} has format {format:?}, but {param} {} an sRGB format.",
        if expects_srgb(*param) {
            "expects"
        } else {
            "does not expect"
        }
    )]
    UnexpectedTextureFormat {
        entry_index: usize,
        material_label: String,
        param: ParamId,
        nutexb: String,
        format: NutexbFormat,
    },

    // TODO: Add severity levels and make this the highest severity.
    #[error("Texture {nutexb:?} for material {material_label:?} has dimensions {actual:?}, but {param} requires {expected:?}.")]
    UnexpectedTextureDimension {
        entry_index: usize,
        material_label: String,
        param: ParamId,
        nutexb: String,
        expected: TextureDimension,
        actual: TextureDimension,
    },

    #[error(
        "Texture {nutexb:?} assigned to param {param} for material {material_label:?} is missing."
    )]
    MissingTexture {
        entry_index: usize,
        material_label: String,
        param: ParamId,
        nutexb: String,
    },

    #[error(
        "Mesh {mesh_name:?} has the RENORMAL material {material_label:?} but no corresponding entry in the model.adjb."
    )]
    RenormalMaterialMissingMeshAdjEntry {
        entry_index: usize,
        material_label: String,
        mesh_name: String,
    },

    #[error(
        "Material {material_label:?} is a RENORMAL material, but the model.adjb file is missing."
    )]
    RenormalMaterialMissingAdj {
        entry_index: usize,
        material_label: String,
    },
}

impl MatlValidationError {
    pub fn entry_index(&self) -> usize {
        // Use the index to associate errors to entries.
        // The material label in user created files isn't always unique.
        match self {
            Self::MissingRequiredVertexAttributes { entry_index, .. } => *entry_index,
            Self::UnexpectedTextureFormat { entry_index, .. } => *entry_index,
            Self::RenormalMaterialMissingMeshAdjEntry { entry_index, .. } => *entry_index,
            Self::RenormalMaterialMissingAdj { entry_index, .. } => *entry_index,
            Self::UnexpectedTextureDimension { entry_index, .. } => *entry_index,
            Self::MissingTexture { entry_index, .. } => *entry_index,
        }
    }
}

pub struct ModlValidationError;
impl Display for ModlValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

pub struct AdjValidationError;
impl Display for AdjValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
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
            // TODO: make this a method of the database?
            if let Some(program) = shader_database.get(entry.shader_label.get(..24).unwrap_or("")) {
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
                        let matl_error = MatlValidationError::MissingRequiredVertexAttributes {
                            entry_index,
                            material_label: entry.material_label.clone(),
                            mesh_name: o.name.clone(),
                            missing_attributes: missing_attributes.clone(),
                        };
                        validation.matl_errors.push(matl_error);

                        let mesh_error = MeshValidationError::MissingRequiredVertexAttributes {
                            mesh_object_index: i,
                            mesh_name: o.name.clone(),
                            material_label: entry.material_label.clone(),
                            missing_attributes,
                        };
                        validation.mesh_errors.push(mesh_error);
                    }
                }
            }
        }
    }
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
                    let error = MatlValidationError::UnexpectedTextureFormat {
                        entry_index,
                        material_label: entry.material_label.clone(),
                        param: texture.param_id,
                        nutexb: f.clone(),
                        format: nutexb.footer.image_format,
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
    default_textures: impl Iterator<Item = &'a String> + Clone,
) where
    'b: 'a,
{
    for (entry_index, entry) in matl.entries.iter().enumerate() {
        for texture in &entry.textures {
            // TODO: Check if is default texture?
            if !nutexbs
                .iter()
                .map(|(f, _)| f)
                .chain(default_textures.clone().into_iter())
                .any(|f| {
                    Path::new(f)
                        .with_extension("")
                        .as_os_str()
                        .eq_ignore_ascii_case(&texture.data)
                })
            {
                let error = MatlValidationError::MissingTexture {
                    entry_index,
                    material_label: entry.material_label.clone(),
                    param: texture.param_id,
                    nutexb: texture.data.clone(),
                };
                validation.matl_errors.push(error);
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TextureDimension {
    Texture2d,
    Texture3d,
    TextureCube,
}

fn nutexb_dimension(nutexb: &NutexbFile) -> TextureDimension {
    // Assume no array layers for depth and cube maps.
    if nutexb.footer.depth > 1 {
        TextureDimension::Texture3d
    } else if nutexb.footer.layer_count == 6 {
        TextureDimension::TextureCube
    } else {
        TextureDimension::Texture2d
    }
}

fn expected_texture_dimension(param: ParamId) -> TextureDimension {
    match param {
        ParamId::Texture2 | ParamId::Texture7 | ParamId::Texture8 => TextureDimension::TextureCube,
        _ => TextureDimension::Texture2d,
    }
}

fn validate_texture_dimensions(
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
                let expected = expected_texture_dimension(texture.param_id);
                let actual = nutexb_dimension(nutexb);
                if actual != expected {
                    // The dimension is a fundamental part of the texture.
                    // Add errors to the matl since users should just assign a new texture.
                    let error = MatlValidationError::UnexpectedTextureDimension {
                        entry_index,
                        material_label: entry.material_label.clone(),
                        param: texture.param_id,
                        nutexb: f.clone(),
                        expected,
                        actual,
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
    // TODO: Errors for both matl and adj?
    // TODO: Is this check case sensitive?
    for (entry_index, entry) in matl
        .entries
        .iter()
        .filter(|e| e.material_label.contains("RENORMAL"))
        .enumerate()
    {
        if let Some(adj) = adj {
            // TODO: Get assigned meshes
            if let Some(modl) = modl {
                if let Some(mesh) = mesh {
                    for (mesh_index, mesh) in modl
                        .entries
                        .iter()
                        .filter(|e| e.material_label == entry.material_label)
                        .filter_map(|e| {
                            mesh.objects.iter().find(|o| {
                                o.name == e.mesh_object_name && o.subindex == e.mesh_object_subindex
                            })
                        })
                        .enumerate()
                    {
                        if !adj
                            .entries
                            .iter()
                            .any(|a| a.mesh_object_index == mesh_index)
                        {
                            let error = MatlValidationError::RenormalMaterialMissingMeshAdjEntry {
                                entry_index,
                                material_label: entry.material_label.clone(),
                                mesh_name: mesh.name.clone(),
                            };
                            validation.matl_errors.push(error);
                        }
                    }
                }
            }
        } else {
            let error = MatlValidationError::RenormalMaterialMissingAdj {
                entry_index,
                material_label: entry.material_label.clone(),
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
            let error = MeshValidationError::DuplicateSubindex {
                mesh_object_index: i,
                mesh_name: o.name.clone(),
                subindex: o.subindex,
            };
            validation.mesh_errors.push(error);
        }
    }
}

#[cfg(test)]
mod tests {
    use nutexb::{NutexbFile, NutexbFooter, NutexbFormat};
    use ssbh_data::{
        matl_data::{MatlEntryData, TextureParam},
        mesh_data::MeshObjectData,
        modl_data::ModlEntryData,
    };
    use ssbh_wgpu::create_database;

    use super::*;

    fn nutexb(image_format: NutexbFormat) -> NutexbFile {
        NutexbFile {
            data: Vec::new(),
            layer_mipmaps: Vec::new(),
            footer: NutexbFooter {
                string: Vec::new().into(),
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
                string: Vec::new().into(),
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
        let shader_database = create_database();
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
            vec![MatlValidationError::MissingRequiredVertexAttributes {
                entry_index: 0,
                material_label: "a".to_owned(),
                mesh_name: "object1".to_owned(),
                missing_attributes: vec!["map1".to_owned(), "uvSet".to_owned()]
            }],
            validation.matl_errors
        );

        assert_eq!(
            vec![MeshValidationError::MissingRequiredVertexAttributes {
                mesh_object_index: 0,
                mesh_name: "object1".to_owned(),
                material_label: "a".to_owned(),
                missing_attributes: vec!["map1".to_owned(), "uvSet".to_owned()]
            }],
            validation.mesh_errors
        );

        assert_eq!(
            r#"Mesh "object1" is missing attributes map1, uvSet required by assigned material "a"."#,
            format!("{}", validation.matl_errors[0])
        );

        assert_eq!(
            r#"Mesh "object1" is missing attributes map1, uvSet required by assigned material "a"."#,
            format!("{}", validation.mesh_errors[0])
        );
    }

    #[test]
    fn renormal_material_missing_adj() {
        let matl = MatlData {
            major_version: 1,
            minor_version: 6,
            entries: vec![MatlEntryData {
                material_label: "a_RENORMAL".to_owned(),
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
                material_label: "a_RENORMAL".to_owned(),
            }],
        };

        let mut validation = ModelValidationErrors::default();
        validate_renormal_material_entries(&mut validation, &matl, None, Some(&modl), Some(&mesh));

        assert_eq!(
            vec![MatlValidationError::RenormalMaterialMissingAdj {
                entry_index: 0,
                material_label: "a_RENORMAL".to_owned(),
            }],
            validation.matl_errors
        );

        assert_eq!(
            r#"Material "a_RENORMAL" is a RENORMAL material, but the model.adjb file is missing."#,
            format!("{}", validation.matl_errors[0])
        );
    }

    #[test]
    fn renormal_material_missing_adj_entry() {
        let matl = MatlData {
            major_version: 1,
            minor_version: 6,
            entries: vec![MatlEntryData {
                material_label: "a_RENORMAL".to_owned(),
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
                material_label: "a_RENORMAL".to_owned(),
            }],
        };
        let adj = AdjData {
            entries: Vec::new(),
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
            vec![MatlValidationError::RenormalMaterialMissingMeshAdjEntry {
                entry_index: 0,
                material_label: "a_RENORMAL".to_owned(),
                mesh_name: "object1".to_owned()
            }],
            validation.matl_errors
        );

        assert_eq!(
            r#"Mesh "object1" has the RENORMAL material "a_RENORMAL" but no corresponding entry in the model.adjb."#,
            format!("{}", validation.matl_errors[0])
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
                MatlValidationError::UnexpectedTextureFormat {
                    entry_index: 0,
                    material_label: "a".to_owned(),
                    param: ParamId::Texture0,
                    nutexb: "texture0".to_owned(),
                    format: NutexbFormat::BC1Unorm
                },
                MatlValidationError::UnexpectedTextureFormat {
                    entry_index: 0,
                    material_label: "a".to_owned(),
                    param: ParamId::Texture4,
                    nutexb: "texture4".to_owned(),
                    format: NutexbFormat::BC2Srgb
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
            ["#replace_cubemap".to_owned()].iter(),
        );

        assert_eq!(
            vec![MatlValidationError::MissingTexture {
                entry_index: 0,
                material_label: "a".to_owned(),
                param: ParamId::Texture0,
                nutexb: "texture0".to_owned(),
            },],
            validation.matl_errors
        );

        assert_eq!(
            r#"Texture "texture0" assigned to param Texture0 for material "a" is missing."#,
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
        validate_texture_dimensions(&mut validation, &matl, &textures);

        assert_eq!(
            vec![
                MatlValidationError::UnexpectedTextureDimension {
                    entry_index: 0,
                    material_label: "a".to_owned(),
                    param: ParamId::Texture0,
                    nutexb: "texture0".to_owned(),
                    expected: TextureDimension::Texture2d,
                    actual: TextureDimension::TextureCube
                },
                MatlValidationError::UnexpectedTextureDimension {
                    entry_index: 0,
                    material_label: "a".to_owned(),
                    param: ParamId::Texture7,
                    nutexb: "texture7".to_owned(),
                    expected: TextureDimension::TextureCube,
                    actual: TextureDimension::Texture2d
                }
            ],
            validation.matl_errors
        );

        assert_eq!(
            r#"Texture "texture0" for material "a" has dimensions TextureCube, but Texture0 requires Texture2d."#,
            format!("{}", validation.matl_errors[0])
        );
        assert_eq!(
            r#"Texture "texture7" for material "a" has dimensions Texture2d, but Texture7 requires TextureCube."#,
            format!("{}", validation.matl_errors[1])
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
            vec![MeshValidationError::DuplicateSubindex {
                mesh_object_index: 2,
                mesh_name: "a".to_owned(),
                subindex: 0
            }],
            validation.mesh_errors
        );

        assert_eq!(
            r#"Mesh "a" repeats subindex 0. Subindices must be unique."#,
            format!("{}", validation.mesh_errors[0])
        );
    }
}
