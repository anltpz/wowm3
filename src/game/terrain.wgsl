// Pyungmoo arazisi: StandardMaterial uzantısı. Splat haritası (R yol, G kaya, B kum, A dere yatağı;
// çim = kalan) ile WoW zemin dokularını karıştırır; aydınlatma Bevy PBR'den.

#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var splat_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var splat_smp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var grass_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var grass_smp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(104) var road_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(105) var road_smp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(106) var rock_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(107) var rock_smp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(108) var sand_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(109) var sand_smp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(110) var bed_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(111) var bed_smp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(112) var flower_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(113) var flower_smp: sampler;

// WoW dokuları gama uzayında saklanır (Rgba8Unorm): doğrusallaştır.
fn lin(c: vec3<f32>) -> vec3<f32> {
    return pow(max(c, vec3<f32>(0.0)), vec3<f32>(2.2));
}

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let w = textureSample(splat_tex, splat_smp, in.uv);
    let wp = in.world_position.xz;
    let t = wp / 4.0;
    let grass = lin(textureSample(grass_tex, grass_smp, t).rgb);
    let flower = lin(textureSample(flower_tex, flower_smp, t * 0.9).rgb);
    let road_raw = lin(textureSample(road_tex, road_smp, t * 0.8).rgb);
    // Metin2 yolları açık kahve/gri: Westfall toprağının turuncusunu kır.
    let road = mix(road_raw, vec3<f32>(dot(road_raw, vec3<f32>(0.3, 0.55, 0.15))), 0.55) * vec3<f32>(1.0, 0.95, 0.88);
    let rock = lin(textureSample(rock_tex, rock_smp, wp / 7.0).rgb);
    let sand = lin(textureSample(sand_tex, sand_smp, t).rgb);
    let bed = lin(textureSample(bed_tex, bed_smp, t).rgb);

    // Çimde çiçek ve renk lekeleri (düzlük tekdüze görünmesin).
    let n = vnoise(wp / 23.0) * 0.6 + vnoise(wp / 7.0) * 0.4;
    let flower_w = smoothstep(0.62, 0.8, n);
    var g = mix(grass, flower, flower_w * 0.8);
    g = g * mix(0.85, 1.12, vnoise(wp / 41.0));

    let road_w = clamp(w.r, 0.0, 1.0);
    let rock_w = clamp(w.g, 0.0, 1.0);
    let sand_w = clamp(w.b, 0.0, 1.0);
    let bed_w = clamp(w.a, 0.0, 1.0);
    var col = g;
    col = mix(col, sand, sand_w);
    col = mix(col, road, road_w);
    col = mix(col, bed, bed_w * 0.85);
    col = mix(col, rock, rock_w);

    pbr_input.material.base_color = vec4<f32>(col, 1.0);
    pbr_input.material.perceptual_roughness = 0.92;
    pbr_input.material.reflectance = vec3<f32>(0.02);

#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif
    return out;
}
