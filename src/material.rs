// TODO: Share vectors between ssbh_data types?
use ssbh_data::{matl_data::*, meshex_data::Vector4};
use ssbh_wgpu::ShaderProgram;

// TODO: Add presets?
pub fn default_material() -> MatlEntryData {
    // TODO: Make sure the name is unique?
    // TODO: Add defaults for other parameters?
    MatlEntryData {
        material_label: "NEW_MATERIAL".to_string(),
        shader_label: "SFX_PBS_0100000008008269_opaque".to_string(),
        blend_states: vec![BlendStateParam {
            param_id: ParamId::BlendState0,
            data: Default::default(),
        }],
        floats: vec![FloatParam {
            param_id: ParamId::CustomFloat8,
            data: 0.4,
        }],
        booleans: vec![
            BooleanParam {
                param_id: ParamId::CustomBoolean1,
                data: true,
            },
            BooleanParam {
                param_id: ParamId::CustomBoolean3,
                data: true,
            },
            BooleanParam {
                param_id: ParamId::CustomBoolean4,
                data: true,
            },
        ],
        vectors: vec![
            Vector4Param {
                // Set to all zeros to allow for transparency.
                param_id: ParamId::CustomVector0,
                data: Vector4::new(0.0, 0.0, 0.0, 0.0),
            },
            Vector4Param {
                param_id: ParamId::CustomVector13,
                data: Vector4::new(1.0, 1.0, 1.0, 1.0),
            },
            Vector4Param {
                param_id: ParamId::CustomVector14,
                data: Vector4::new(1.0, 1.0, 1.0, 1.0),
            },
            Vector4Param {
                param_id: ParamId::CustomVector8,
                data: Vector4::new(1.0, 1.0, 1.0, 1.0),
            },
        ],
        rasterizer_states: vec![RasterizerStateParam {
            param_id: ParamId::RasterizerState0,
            data: Default::default(),
        }],
        samplers: vec![
            SamplerParam {
                param_id: ParamId::Sampler0,
                data: Default::default(),
            },
            SamplerParam {
                param_id: ParamId::Sampler4,
                data: Default::default(),
            },
            SamplerParam {
                param_id: ParamId::Sampler6,
                data: Default::default(),
            },
            SamplerParam {
                param_id: ParamId::Sampler7,
                data: Default::default(),
            },
        ],
        textures: vec![
            TextureParam {
                param_id: ParamId::Texture0,
                data: default_texture(ParamId::Texture0),
            },
            TextureParam {
                param_id: ParamId::Texture4,
                data: default_texture(ParamId::Texture4),
            },
            TextureParam {
                param_id: ParamId::Texture6,
                data: default_texture(ParamId::Texture6),
            },
            TextureParam {
                param_id: ParamId::Texture7,
                data: default_texture(ParamId::Texture7),
            },
        ],
    }
}

pub fn missing_parameters(entry: &MatlEntryData, program: &ShaderProgram) -> Vec<ParamId> {
    program
        .material_parameters
        .iter()
        .copied()
        .filter(|param| {
            entry
                .booleans
                .iter()
                .map(|p| p.param_id)
                .chain(entry.floats.iter().map(|p| p.param_id))
                .chain(entry.vectors.iter().map(|p| p.param_id))
                .chain(entry.textures.iter().map(|p| p.param_id))
                .chain(entry.samplers.iter().map(|p| p.param_id))
                .chain(entry.blend_states.iter().map(|p| p.param_id))
                .chain(entry.rasterizer_states.iter().map(|p| p.param_id))
                .find(|p| p == param)
                .is_none()
        })
        .collect()
}

pub fn unused_parameters(entry: &MatlEntryData, program: &ShaderProgram) -> Vec<ParamId> {
    entry
        .booleans
        .iter()
        .map(|p| p.param_id)
        .chain(entry.floats.iter().map(|p| p.param_id))
        .chain(entry.vectors.iter().map(|p| p.param_id))
        .chain(entry.textures.iter().map(|p| p.param_id))
        .chain(entry.samplers.iter().map(|p| p.param_id))
        .chain(entry.blend_states.iter().map(|p| p.param_id))
        .chain(entry.rasterizer_states.iter().map(|p| p.param_id))
        .filter(|param| !program.material_parameters.contains(param))
        .collect()
}

pub fn add_parameters(entry: &mut MatlEntryData, parameters: &[ParamId]) {
    // TODO: More intelligently pick defaults
    for param_id in parameters.iter().copied() {
        if is_blend(param_id) {
            entry.blend_states.push(BlendStateParam {
                param_id,
                data: BlendStateData::default(),
            });
        } else if is_float(param_id) {
            entry.floats.push(FloatParam {
                param_id,
                data: 0.0,
            });
        } else if is_bool(param_id) {
            entry.booleans.push(BooleanParam {
                param_id,
                data: false,
            });
        } else if is_vector(param_id) {
            entry.vectors.push(Vector4Param {
                param_id,
                data: Vector4::default(),
            });
        } else if is_rasterizer(param_id) {
            entry.rasterizer_states.push(RasterizerStateParam {
                param_id,
                data: RasterizerStateData::default(),
            });
        } else if is_sampler(param_id) {
            entry.samplers.push(SamplerParam {
                param_id,
                data: SamplerData::default(),
            });
        } else if is_texture(param_id) {
            entry.textures.push(TextureParam {
                param_id,
                data: default_texture(param_id),
            });
        }
    }

    // Sort the parameters to match Smash Ultimate's conventions.
    entry.blend_states.sort_by_key(|p| p.param_id as u64);
    entry.floats.sort_by_key(|p| p.param_id as u64);
    entry.booleans.sort_by_key(|p| p.param_id as u64);
    entry.vectors.sort_by_key(|p| p.param_id as u64);
    entry.rasterizer_states.sort_by_key(|p| p.param_id as u64);
    entry.samplers.sort_by_key(|p| p.param_id as u64);
    entry.textures.sort_by_key(|p| p.param_id as u64);
}

pub fn remove_parameters(entry: &mut MatlEntryData, parameters: &[ParamId]) {
    // Using the faster swap_remove function since we sort at the end anyway.
    for param in parameters.iter().copied() {
        if let Some(index) = entry.blend_states.iter().position(|p| p.param_id == param) {
            entry.blend_states.swap_remove(index);
        } else if let Some(index) = entry.floats.iter().position(|p| p.param_id == param) {
            entry.floats.swap_remove(index);
        } else if let Some(index) = entry.booleans.iter().position(|p| p.param_id == param) {
            entry.booleans.swap_remove(index);
        } else if let Some(index) = entry.vectors.iter().position(|p| p.param_id == param) {
            entry.vectors.swap_remove(index);
        } else if let Some(index) = entry
            .rasterizer_states
            .iter()
            .position(|p| p.param_id == param)
        {
            entry.rasterizer_states.swap_remove(index);
        } else if let Some(index) = entry.samplers.iter().position(|p| p.param_id == param) {
            entry.samplers.swap_remove(index);
        } else if let Some(index) = entry.textures.iter().position(|p| p.param_id == param) {
            entry.textures.swap_remove(index);
        }
    }

    // Sort the parameters to match Smash Ultimate's conventions.
    entry.blend_states.sort_by_key(|p| p.param_id as u64);
    entry.floats.sort_by_key(|p| p.param_id as u64);
    entry.booleans.sort_by_key(|p| p.param_id as u64);
    entry.vectors.sort_by_key(|p| p.param_id as u64);
    entry.rasterizer_states.sort_by_key(|p| p.param_id as u64);
    entry.samplers.sort_by_key(|p| p.param_id as u64);
    entry.textures.sort_by_key(|p| p.param_id as u64);
}

// TODO: Move this to ssbh_wgpu?
fn is_vector(p: ParamId) -> bool {
    match p {
        ParamId::CustomVector0 => true,
        ParamId::CustomVector1 => true,
        ParamId::CustomVector2 => true,
        ParamId::CustomVector3 => true,
        ParamId::CustomVector4 => true,
        ParamId::CustomVector5 => true,
        ParamId::CustomVector6 => true,
        ParamId::CustomVector7 => true,
        ParamId::CustomVector8 => true,
        ParamId::CustomVector9 => true,
        ParamId::CustomVector10 => true,
        ParamId::CustomVector11 => true,
        ParamId::CustomVector12 => true,
        ParamId::CustomVector13 => true,
        ParamId::CustomVector14 => true,
        ParamId::CustomVector15 => true,
        ParamId::CustomVector16 => true,
        ParamId::CustomVector17 => true,
        ParamId::CustomVector18 => true,
        ParamId::CustomVector19 => true,
        ParamId::CustomVector20 => true,
        ParamId::CustomVector21 => true,
        ParamId::CustomVector22 => true,
        ParamId::CustomVector23 => true,
        ParamId::CustomVector24 => true,
        ParamId::CustomVector25 => true,
        ParamId::CustomVector26 => true,
        ParamId::CustomVector27 => true,
        ParamId::CustomVector28 => true,
        ParamId::CustomVector29 => true,
        ParamId::CustomVector30 => true,
        ParamId::CustomVector31 => true,
        ParamId::CustomVector32 => true,
        ParamId::CustomVector33 => true,
        ParamId::CustomVector34 => true,
        ParamId::CustomVector35 => true,
        ParamId::CustomVector36 => true,
        ParamId::CustomVector37 => true,
        ParamId::CustomVector38 => true,
        ParamId::CustomVector39 => true,
        ParamId::CustomVector40 => true,
        ParamId::CustomVector41 => true,
        ParamId::CustomVector42 => true,
        ParamId::CustomVector43 => true,
        ParamId::CustomVector44 => true,
        ParamId::CustomVector45 => true,
        ParamId::CustomVector46 => true,
        ParamId::CustomVector47 => true,
        ParamId::CustomVector48 => true,
        ParamId::CustomVector49 => true,
        ParamId::CustomVector50 => true,
        ParamId::CustomVector51 => true,
        ParamId::CustomVector52 => true,
        ParamId::CustomVector53 => true,
        ParamId::CustomVector54 => true,
        ParamId::CustomVector55 => true,
        ParamId::CustomVector56 => true,
        ParamId::CustomVector57 => true,
        ParamId::CustomVector58 => true,
        ParamId::CustomVector59 => true,
        ParamId::CustomVector60 => true,
        ParamId::CustomVector61 => true,
        ParamId::CustomVector62 => true,
        ParamId::CustomVector63 => true,

        _ => false,
    }
}

fn is_rasterizer(p: ParamId) -> bool {
    match p {
        ParamId::RasterizerState0 => true,
        ParamId::RasterizerState1 => true,
        ParamId::RasterizerState2 => true,
        ParamId::RasterizerState3 => true,
        ParamId::RasterizerState4 => true,
        ParamId::RasterizerState5 => true,
        ParamId::RasterizerState6 => true,
        ParamId::RasterizerState7 => true,
        ParamId::RasterizerState8 => true,
        ParamId::RasterizerState9 => true,
        ParamId::RasterizerState10 => true,
        _ => false,
    }
}

fn is_blend(p: ParamId) -> bool {
    match p {
        ParamId::BlendState0 => true,
        ParamId::BlendState1 => true,
        ParamId::BlendState2 => true,
        ParamId::BlendState3 => true,
        ParamId::BlendState4 => true,
        ParamId::BlendState5 => true,
        ParamId::BlendState6 => true,
        ParamId::BlendState7 => true,
        ParamId::BlendState8 => true,
        ParamId::BlendState9 => true,
        ParamId::BlendState10 => true,
        _ => false,
    }
}

fn is_float(p: ParamId) -> bool {
    match p {
        ParamId::CustomFloat0 => true,
        ParamId::CustomFloat1 => true,
        ParamId::CustomFloat2 => true,
        ParamId::CustomFloat3 => true,
        ParamId::CustomFloat4 => true,
        ParamId::CustomFloat5 => true,
        ParamId::CustomFloat6 => true,
        ParamId::CustomFloat7 => true,
        ParamId::CustomFloat8 => true,
        ParamId::CustomFloat9 => true,
        ParamId::CustomFloat10 => true,
        ParamId::CustomFloat11 => true,
        ParamId::CustomFloat12 => true,
        ParamId::CustomFloat13 => true,
        ParamId::CustomFloat14 => true,
        ParamId::CustomFloat15 => true,
        ParamId::CustomFloat16 => true,
        ParamId::CustomFloat17 => true,
        ParamId::CustomFloat18 => true,
        ParamId::CustomFloat19 => true,
        _ => false,
    }
}

fn is_texture(p: ParamId) -> bool {
    match p {
        ParamId::Texture0 => true,
        ParamId::Texture1 => true,
        ParamId::Texture2 => true,
        ParamId::Texture3 => true,
        ParamId::Texture4 => true,
        ParamId::Texture5 => true,
        ParamId::Texture6 => true,
        ParamId::Texture7 => true,
        ParamId::Texture8 => true,
        ParamId::Texture9 => true,
        ParamId::Texture10 => true,
        ParamId::Texture11 => true,
        ParamId::Texture12 => true,
        ParamId::Texture13 => true,
        ParamId::Texture14 => true,
        ParamId::Texture15 => true,
        ParamId::Texture16 => true,
        ParamId::Texture17 => true,
        ParamId::Texture18 => true,
        ParamId::Texture19 => true,
        _ => false,
    }
}

fn is_sampler(p: ParamId) -> bool {
    match p {
        ParamId::Sampler0 => true,
        ParamId::Sampler1 => true,
        ParamId::Sampler2 => true,
        ParamId::Sampler3 => true,
        ParamId::Sampler4 => true,
        ParamId::Sampler5 => true,
        ParamId::Sampler6 => true,
        ParamId::Sampler7 => true,
        ParamId::Sampler8 => true,
        ParamId::Sampler9 => true,
        ParamId::Sampler10 => true,
        ParamId::Sampler11 => true,
        ParamId::Sampler12 => true,
        ParamId::Sampler13 => true,
        ParamId::Sampler14 => true,
        ParamId::Sampler15 => true,
        ParamId::Sampler16 => true,
        ParamId::Sampler17 => true,
        ParamId::Sampler18 => true,
        ParamId::Sampler19 => true,
        _ => false,
    }
}

fn is_bool(p: ParamId) -> bool {
    match p {
        ParamId::CustomBoolean0 => true,
        ParamId::CustomBoolean1 => true,
        ParamId::CustomBoolean2 => true,
        ParamId::CustomBoolean3 => true,
        ParamId::CustomBoolean4 => true,
        ParamId::CustomBoolean5 => true,
        ParamId::CustomBoolean6 => true,
        ParamId::CustomBoolean7 => true,
        ParamId::CustomBoolean8 => true,
        ParamId::CustomBoolean9 => true,
        ParamId::CustomBoolean10 => true,
        ParamId::CustomBoolean11 => true,
        ParamId::CustomBoolean12 => true,
        ParamId::CustomBoolean13 => true,
        ParamId::CustomBoolean14 => true,
        ParamId::CustomBoolean15 => true,
        ParamId::CustomBoolean16 => true,
        ParamId::CustomBoolean17 => true,
        ParamId::CustomBoolean18 => true,
        ParamId::CustomBoolean19 => true,
        _ => false,
    }
}

fn default_texture(p: ParamId) -> String {
    // The default texture should have as close as possible to no effect.
    // This reduces the number of textures that need to be manually assigned.
    match p {
        ParamId::Texture0 => "/common/shader/sfxpbs/default_white".to_string(),
        ParamId::Texture1 => "/common/shader/sfxpbs/default_white".to_string(),
        ParamId::Texture2 => "#replace_cubemap".to_string(),
        ParamId::Texture3 => "/common/shader/sfxpbs/default_white".to_string(),
        ParamId::Texture4 => "/common/shader/sfxpbs/fighter/default_normal".to_string(),
        ParamId::Texture5 => "/common/shader/sfxpbs/default_black".to_string(),
        ParamId::Texture6 => "/common/shader/sfxpbs/fighter/default_params".to_string(),
        ParamId::Texture7 => "#replace_cubemap".to_string(),
        ParamId::Texture8 => "#replace_cubemap".to_string(), // TODO: Better default cube map?
        ParamId::Texture9 => "/common/shader/sfxpbs/default_black".to_string(),
        ParamId::Texture10 => "/common/shader/sfxpbs/default_white".to_string(),
        ParamId::Texture11 => "/common/shader/sfxpbs/default_white".to_string(),
        ParamId::Texture12 => "/common/shader/sfxpbs/default_white".to_string(),
        ParamId::Texture13 => "/common/shader/sfxpbs/default_white".to_string(),
        ParamId::Texture14 => "/common/shader/sfxpbs/default_black".to_string(),
        ParamId::Texture15 => "/common/shader/sfxpbs/default_white".to_string(),
        ParamId::Texture16 => "/common/shader/sfxpbs/default_white".to_string(),
        ParamId::Texture17 => "/common/shader/sfxpbs/default_white".to_string(),
        ParamId::Texture18 => "/common/shader/sfxpbs/default_white".to_string(),
        ParamId::Texture19 => "/common/shader/sfxpbs/default_white".to_string(),
        _ => "/common/shader/sfxpbs/default_white".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_parameters_all_missing() {
        let mut entry = MatlEntryData {
            material_label: String::new(),
            shader_label: String::new(),
            blend_states: Vec::new(),
            floats: Vec::new(),
            booleans: Vec::new(),
            vectors: Vec::new(),
            rasterizer_states: Vec::new(),
            samplers: Vec::new(),
            textures: Vec::new(),
        };

        let required_parameters = missing_parameters(
            &entry,
            &ShaderProgram {
                discard: false,
                vertex_attributes: Vec::new(),
                material_parameters: vec![
                    ParamId::BlendState0,
                    ParamId::CustomFloat0,
                    ParamId::CustomBoolean0,
                    ParamId::CustomVector0,
                    ParamId::RasterizerState0,
                    ParamId::Sampler0,
                    ParamId::Texture0,
                ],
            },
        );
        add_parameters(&mut entry, &required_parameters);

        // TODO: Add tests for better default values.
        assert_eq!(
            MatlEntryData {
                material_label: String::new(),
                shader_label: String::new(),
                blend_states: vec![BlendStateParam {
                    param_id: ParamId::BlendState0,
                    data: Default::default(),
                }],
                floats: vec![FloatParam {
                    param_id: ParamId::CustomFloat0,
                    data: Default::default(),
                }],
                booleans: vec![BooleanParam {
                    param_id: ParamId::CustomBoolean0,
                    data: Default::default(),
                }],
                vectors: vec![Vector4Param {
                    param_id: ParamId::CustomVector0,
                    data: Default::default(),
                }],
                rasterizer_states: vec![RasterizerStateParam {
                    param_id: ParamId::RasterizerState0,
                    data: Default::default(),
                }],
                samplers: vec![SamplerParam {
                    param_id: ParamId::Sampler0,
                    data: Default::default(),
                }],
                textures: vec![TextureParam {
                    param_id: ParamId::Texture0,
                    data: "/common/shader/sfx_pbs/default_white".to_string(),
                }],
            },
            entry
        );
    }

    #[test]
    fn remove_parameters_all_unused() {
        let mut entry = MatlEntryData {
            material_label: String::new(),
            shader_label: String::new(),
            blend_states: vec![BlendStateParam {
                param_id: ParamId::BlendState0,
                data: Default::default(),
            }],
            floats: vec![FloatParam {
                param_id: ParamId::CustomFloat0,
                data: Default::default(),
            }],
            booleans: vec![BooleanParam {
                param_id: ParamId::CustomBoolean0,
                data: Default::default(),
            }],
            vectors: vec![Vector4Param {
                param_id: ParamId::CustomVector0,
                data: Default::default(),
            }],
            rasterizer_states: vec![RasterizerStateParam {
                param_id: ParamId::RasterizerState0,
                data: Default::default(),
            }],
            samplers: vec![SamplerParam {
                param_id: ParamId::Sampler0,
                data: Default::default(),
            }],
            textures: vec![TextureParam {
                param_id: ParamId::Texture0,
                data: Default::default(),
            }],
        };

        let unused_parameters = unused_parameters(
            &entry,
            &ShaderProgram {
                discard: false,
                vertex_attributes: Vec::new(),
                material_parameters: Vec::new(),
            },
        );
        remove_parameters(&mut entry, &unused_parameters);

        assert!(entry.blend_states.is_empty());
        assert!(entry.floats.is_empty());
        assert!(entry.booleans.is_empty());
        assert!(entry.vectors.is_empty());
        assert!(entry.rasterizer_states.is_empty());
        assert!(entry.samplers.is_empty());
        assert!(entry.textures.is_empty());
    }
}
