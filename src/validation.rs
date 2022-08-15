use std::{error::Error, fmt::Display, path::Path};

use nutexb::{NutexbFile, NutexbFormat};
use ssbh_data::{matl_data::ParamId, prelude::*};
use ssbh_wgpu::{ModelFolder, ShaderDatabase};

// TODO: How to update these only when a file changes?
// TODO: Only validate known names like model.numatb or model.numdlb?
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
    pub fn from_model(model: &ModelFolder, shader_database: &ShaderDatabase) -> Self {
        let mut validation = Self::default();

        // Each validation check may add errors to multiple related files.
        if let Some(matl) = model.find_matl() {
            validate_required_attributes(
                &mut validation,
                matl,
                model.find_modl(),
                model.find_mesh(),
                shader_database,
            );

            validate_texture_format_usage(&mut validation, matl, &model.nutexbs);

            validate_renormal_material_entries(
                &mut validation,
                matl,
                model.find_adj(),
                model.find_modl(),
                model.find_mesh(),
            );

            // TODO: Validate mismatches with cube maps and 2D textures.
        }
        validation
    }
}

// TODO: Use thiserror instead?
#[derive(Debug, PartialEq, Eq)]
pub enum MeshValidationError {
    MissingRequiredVertexAttributes {
        mesh_object_index: usize,
        mesh_name: String,
        material_label: String,
        missing_attributes: Vec<String>,
    },
}

impl Display for MeshValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeshValidationError::MissingRequiredVertexAttributes {
                mesh_name,
                material_label,
                missing_attributes,
                ..
            } => write!(
                f,
                "Mesh {:?} is missing attributes {} required by assigned material {:?}.",
                mesh_name,
                missing_attributes.join(", "),
                material_label
            ),
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
#[derive(Debug, PartialEq, Eq)]
pub enum MatlValidationError {
    MissingRequiredVertexAttributes {
        entry_index: usize,
        material_label: String,
        mesh_name: String,
        missing_attributes: Vec<String>,
    },
    InvalidTextureFormat {
        entry_index: usize,
        material_label: String,
        param: ParamId,
        nutexb: String,
        format: NutexbFormat,
    },
    RenormalMaterialMissingMeshAdjEntry {
        entry_index: usize,
        material_label: String,
        mesh_name: String,
    },
    RenormalMaterialMissingAdj {
        entry_index: usize,
        material_label: String,
    },
}

impl Display for MatlValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatlValidationError::MissingRequiredVertexAttributes {
                material_label,
                mesh_name,
                missing_attributes,
                ..
            } => write!(
                f,
                "Mesh {:?} is missing attributes {} required by assigned material {:?}.",
                mesh_name,
                missing_attributes.join(", "),
                material_label
            ),
            MatlValidationError::InvalidTextureFormat {
                material_label,
                param,
                nutexb,
                format,
                ..
            } => write!(
                f,
                "Texture {:?} for material {:?} has format {:?}, but {} {} an sRGB format.",
                nutexb,
                material_label,
                format,
                param,
                if expects_srgb(*param) {
                    "expects"
                } else {
                    "does not expect"
                }
            ),
            MatlValidationError::RenormalMaterialMissingMeshAdjEntry {
                material_label,
                mesh_name,
                ..
            } => write!(
                f,
                "Mesh {:?} has the RENORMAL material {:?} but no corresponding entry in the model.adjb.",
                mesh_name,
                material_label
            ),
            MatlValidationError::RenormalMaterialMissingAdj {
                material_label,
                ..
            } => write!(
                f,
                "Material {:?} is a RENORMAL material, but the model.adjb file is missing.",
                material_label
            ),
        }
    }
}

impl MatlValidationError {
    pub fn entry_index(&self) -> usize {
        // Use the index to associate errors to entries.
        // The material label in user created files isn't always unique.
        match self {
            Self::MissingRequiredVertexAttributes { entry_index, .. } => *entry_index,
            Self::InvalidTextureFormat { entry_index, .. } => *entry_index,
            Self::RenormalMaterialMissingMeshAdjEntry { entry_index, .. } => *entry_index,
            Self::RenormalMaterialMissingAdj { entry_index, .. } => *entry_index,
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

pub struct NutexbValidationError;
impl Display for NutexbValidationError {
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
            // TODO: make this a method of the database?
            if let Some(program) = shader_database.get(entry.shader_label.get(..24).unwrap_or("")) {
                for (i, o) in mesh.objects.iter().enumerate().filter(|(_, o)| {
                    modl.entries
                        .iter()
                        .filter(|e| e.material_label == entry.material_label)
                        .any(|e| {
                            e.mesh_object_name == o.name && e.mesh_object_sub_index == o.sub_index
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
    nutexbs: &[(String, Result<NutexbFile, Box<dyn Error>>)],
) {
    // TODO: Errors for both matl and nutexb?
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
                    let error = MatlValidationError::InvalidTextureFormat {
                        entry_index,
                        material_label: entry.material_label.clone(),
                        param: texture.param_id,
                        nutexb: f.clone(),
                        format: nutexb.footer.image_format,
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
                                o.name == e.mesh_object_name
                                    && o.sub_index == e.mesh_object_sub_index
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
    // These formats will render inaccurately with sRGB.
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
                sub_index: 0,
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
                mesh_object_sub_index: 0,
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
                sub_index: 0,
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
                mesh_object_sub_index: 0,
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
                sub_index: 0,
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
                mesh_object_sub_index: 0,
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

        // TODO: Add a nutexb error as well?
        assert_eq!(
            vec![
                MatlValidationError::InvalidTextureFormat {
                    entry_index: 0,
                    material_label: "a".to_owned(),
                    param: ParamId::Texture0,
                    nutexb: "texture0".to_owned(),
                    format: NutexbFormat::BC1Unorm
                },
                MatlValidationError::InvalidTextureFormat {
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
    }
}
